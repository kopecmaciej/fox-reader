use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::glib::{self, Object};
use gtk::{
    self,
    cairo::Context,
    gdk_pixbuf::{Colorspace, Pixbuf},
    glib::clone,
};
use pdfium_render::prelude::{PdfDocument, PdfPage, PdfPoints, PdfRenderConfig};
use std::{cell::RefCell, collections::BTreeMap, fmt::Debug, rc::Rc};

use crate::{
    core::{runtime::runtime, tts::TTSEvent},
    settings::SETTINGS,
    utils::pdf_highlighter::PdfReadingBlock,
};

use super::{
    dialogs::{self, file_dialog, show_error_dialog},
    voice_events::event_emiter,
};

mod imp {
    use crate::{
        ui::audio_controls::AudioControls,
        utils::{pdf_highlighter::PdfHighlighter, pdfium::PdfiumWrapper},
    };

    use super::*;
    use gtk::CompositeTemplate;
    use pdfium_render::prelude::PdfPageIndex;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/pdf_reader.ui")]
    pub struct PdfReader {
        #[template_child]
        pub drop_area: TemplateChild<gtk::DropTarget>,
        #[template_child]
        pub content_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub open_pdf: TemplateChild<gtk::Button>,
        #[template_child]
        pub overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub drawing_area: TemplateChild<gtk::DrawingArea>,
        #[template_child]
        pub prev_page: TemplateChild<gtk::Button>,
        #[template_child]
        pub next_page: TemplateChild<gtk::Button>,
        #[template_child]
        pub current_page: TemplateChild<gtk::Entry>,
        #[template_child]
        pub total_pages: TemplateChild<gtk::Label>,
        #[template_child]
        pub close_pdf: TemplateChild<gtk::Button>,
        #[template_child]
        pub zoom_in: TemplateChild<gtk::Button>,
        #[template_child]
        pub zoom_out: TemplateChild<gtk::Button>,
        #[template_child]
        pub audio_controls: TemplateChild<AudioControls>,

        pub scale_factor: RefCell<f32>,
        pub pdf_wrapper: RefCell<PdfiumWrapper>,
        pub current_page_num: RefCell<PdfPageIndex>,
        pub pdf_highlighter: RefCell<PdfHighlighter>,
        pub highlight_area: gtk::DrawingArea,
        pub page_dimensions: RefCell<(u32, u32)>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PdfReader {
        const NAME: &'static str = "PdfReader";
        type Type = super::PdfReader;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PdfReader {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
            Self::connect_signals(self, &obj);
        }

        fn dispose(&self) {
            self.content_stack.unparent();
        }
    }

    impl WidgetImpl for PdfReader {}

    impl PdfReader {
        fn connect_signals(&self, obj: &super::PdfReader) {
            self.open_pdf.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.open_file_dialog();
                }
            ));

            self.drop_area.connect_drop(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |_, value, _, _| -> bool {
                    if let Ok(file) = value.get::<gio::File>() {
                        obj.load_pdf(file);
                        true
                    } else {
                        false
                    }
                }
            ));

            self.prev_page.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.navigate_page(-1);
                }
            ));

            self.next_page.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.navigate_page(1);
                }
            ));

            self.zoom_in.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| obj.scale_pdf(0.25)
            ));

            self.zoom_out.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| obj.scale_pdf(-0.25)
            ));

            self.current_page.connect_activate(clone!(
                #[weak]
                obj,
                move |entry| {
                    if let Ok(page) = entry.text().parse::<i32>() {
                        obj.go_to_page(page - 1);
                    }
                }
            ));

            self.close_pdf.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.close_pdf();
                }
            ));
        }
    }
}

glib::wrapper! {
    pub struct PdfReader(ObjectSubclass<imp::PdfReader>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for PdfReader {
    fn default() -> Self {
        Object::builder().build()
    }
}

impl PdfReader {
    pub fn init(&self) {
        let imp = self.imp();
        imp.audio_controls.init();
        imp.audio_controls.connect_pdf_audio_events();
        if let Err(e) = imp.pdf_wrapper.borrow_mut().init() {
            show_error_dialog(&format!("Error initializing pdfium: {}", e), self);
        };
        self.init_audio_control_buttons();
        imp.scale_factor.replace(1.5);
        SETTINGS.connect_theme_changed(clone!(
            #[weak(rename_to=this)]
            self,
            move |_settings, _key| {
                this.refresh_view();
            }
        ));
    }

    pub fn refresh_view(&self) {
        let imp = self.imp();
        if let Some(doc) = imp.pdf_wrapper.borrow().get_document() {
            self.render_current_page(doc);
        }
    }

    fn load_pdf(&self, file: gio::File) {
        let path = match file.path() {
            Some(path) => path,
            None => {
                show_error_dialog("Could not get file path", self);
                return;
            }
        };
        if let Err(e) = self.imp().pdf_wrapper.borrow_mut().load_document(&path) {
            show_error_dialog(&format!("{}", e), self);
        }

        if let Some(pdf_document) = self.imp().pdf_wrapper.borrow().get_document() {
            let imp = self.imp();
            *imp.current_page_num.borrow_mut() = 0;
            imp.total_pages
                .set_text(&pdf_document.pages().len().to_string());

            self.render_current_page(pdf_document);
            imp.content_stack.set_visible_child_name("pdf_view");
        }
    }

    fn render_current_page(&self, doc: &PdfDocument) {
        let imp = self.imp();
        let current_page = *imp.current_page_num.borrow();

        match doc.pages().get(current_page) {
            Ok(page) => {
                let (width, height) = (page.width(), page.height());

                if imp
                    .pdf_highlighter
                    .borrow_mut()
                    .generate_reading_blocks(&page, current_page)
                    .is_err()
                {
                    return;
                }

                self.render_pdf(&page, width, height);

                self.setup_click_controller(&page);
                self.setup_hover_controller(&page);
                imp.current_page.set_text(&(current_page + 1).to_string());
            }
            Err(e) => {
                eprintln!("Error rendering PDF: {}", e);
                show_error_dialog(&format!("Error rendering PDF: {}", e), self);
            }
        }
    }

    fn render_pdf(&self, page: &PdfPage, width: PdfPoints, height: PdfPoints) {
        let imp = self.imp();
        let scale_factor = *imp.scale_factor.borrow();
        let rendered = page
            .render_with_config(
                &PdfRenderConfig::new()
                    .set_target_width((width.value) as i32)
                    .set_target_height((height.value) as i32)
                    .scale_page_height_by_factor(scale_factor)
                    .scale_page_width_by_factor(scale_factor),
            )
            .expect("Failed to render PDF page");

        let dynamic_image = rendered.as_image();
        let mut rgba_image = dynamic_image.to_rgba8();

        if SETTINGS.is_dark_color_scheme() {
            for pixel in rgba_image.pixels_mut() {
                if pixel[3] == 0 {
                    continue;
                }

                if pixel[0] > 240 && pixel[1] > 240 && pixel[2] > 240 {
                    pixel[0] = 30;
                    pixel[1] = 30;
                    pixel[2] = 30;
                } else {
                    pixel[0] = 255 - pixel[0];
                    pixel[1] = 255 - pixel[1];
                    pixel[2] = 255 - pixel[2];
                }
            }
        }

        let (img_width, img_height) = rgba_image.dimensions();
        *imp.page_dimensions.borrow_mut() = (img_width, img_height);
        let rowstride = img_width * 4;

        let pixbuf = Pixbuf::from_mut_slice(
            rgba_image.into_raw(),
            Colorspace::Rgb,
            true,
            8,
            img_width as i32,
            img_height as i32,
            rowstride as i32,
        );

        let drawing_area = &imp.drawing_area;
        drawing_area.set_content_width(img_width as i32);
        drawing_area.set_content_height(img_height as i32);

        imp.highlight_area.set_content_width(img_width as i32);
        imp.highlight_area.set_content_height(img_height as i32);
        imp.highlight_area.set_halign(gtk::Align::Center);

        drawing_area.set_draw_func(move |_, cr: &Context, _width, _height| {
            cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
            cr.paint().expect("Failed to paint PDF page");

            cr.fill().expect("Failed to fill base highlights");
        });
    }

    fn setup_click_controller(&self, page: &PdfPage) {
        let imp = self.imp();
        let scale_factor = *imp.scale_factor.borrow();
        let drawing_area = &imp.highlight_area;

        let reading_blocks = imp.pdf_highlighter.borrow().get_reading_blocks();
        let click_controller = gtk::GestureClick::new();
        let page_size = page.page_size();

        click_controller.connect_pressed(clone!(move |_, _, x, y| {
            let clicked_block = reading_blocks.iter().find_map(|block| {
                let scale_factor = scale_factor as f64;
                block.rectangles.iter().find_map(|rect| {
                    let scaled_rect = (
                        rect.left().value as f64 * scale_factor,
                        page_size.top().value as f64 * scale_factor
                            - rect.top().value as f64 * scale_factor,
                        rect.width().value as f64 * scale_factor,
                        rect.height().value as f64 * scale_factor,
                    );

                    let margin_x = scaled_rect.2 * 0.05;
                    let margin_y = scaled_rect.3 * 0.05;

                    let hover_area = (
                        scaled_rect.0 - margin_x,
                        scaled_rect.1 - margin_y,
                        scaled_rect.2 + (margin_x * 2.0),
                        scaled_rect.3 + (margin_y * 2.0),
                    );

                    if x >= hover_area.0
                        && x <= hover_area.0 + hover_area.2
                        && y >= hover_area.1
                        && y <= hover_area.1 + hover_area.3
                    {
                        Some(block.id)
                    } else {
                        None
                    }
                })
            });

            if let Some(block_id) = clicked_block {
                let events = event_emiter();
                events.emit_audio_play(block_id);
            }
        }));

        drawing_area.add_controller(click_controller);
        imp.overlay.add_overlay(drawing_area);
    }

    fn setup_hover_controller(&self, page: &PdfPage) {
        let imp = self.imp();
        let page_size = page.page_size();
        let drawing_area = &imp.highlight_area;
        let motion_controller = gtk::EventControllerMotion::new();
        let reading_blocks = imp.pdf_highlighter.borrow().get_reading_blocks();
        let scale_factor = *imp.scale_factor.borrow();

        let hover_rect = reading_blocks.to_vec();
        let hovered_rect = Rc::new(RefCell::new(Option::<usize>::None));
        let hovered_rect_clone = hovered_rect.clone();

        let scale_factor = scale_factor as f64;

        motion_controller.connect_motion(clone!(
            #[weak]
            drawing_area,
            move |_, x, y| {
                let mut current_hover = hovered_rect.borrow_mut();

                let new_hover = hover_rect.iter().enumerate().find_map(|(index, block)| {
                    block.rectangles.iter().find_map(|rect| {
                        let scaled_rect = (
                            rect.left().value as f64 * scale_factor,
                            page_size.top().value as f64 * scale_factor
                                - rect.top().value as f64 * scale_factor,
                            rect.width().value as f64 * scale_factor,
                            rect.height().value as f64 * scale_factor,
                        );

                        let margin_x = scaled_rect.2 * 0.05;
                        let margin_y = scaled_rect.3 * 0.05;

                        let hover_area = (
                            scaled_rect.0 - margin_x,
                            scaled_rect.1 - margin_y,
                            scaled_rect.2 + (margin_x * 2.0),
                            scaled_rect.3 + (margin_y * 2.0),
                        );

                        if x >= hover_area.0
                            && x <= hover_area.0 + hover_area.2
                            && y >= hover_area.1
                            && y <= hover_area.1 + hover_area.3
                        {
                            Some(index)
                        } else {
                            None
                        }
                    })
                });

                if *current_hover != new_hover {
                    *current_hover = new_hover;
                    drawing_area.queue_draw();
                }
            }
        ));

        let (red, green, blue) = self.get_rgba_colors();

        drawing_area.set_draw_func(clone!(
            #[weak]
            imp,
            move |_, cr: &Context, _width, _height| {
                // Hovered block
                cr.set_source_rgba(red.into(), green.into(), blue.into(), 0.6);
                if let Some(hover_index) = *hovered_rect_clone.borrow() {
                    let highlighted_block = &reading_blocks[hover_index];

                    for rect in highlighted_block.rectangles.iter() {
                        cr.rectangle(
                            rect.left().value as f64 * scale_factor,
                            page_size.top().value as f64 * scale_factor
                                - rect.top().value as f64 * scale_factor,
                            rect.width().value as f64 * scale_factor,
                            rect.height().value as f64 * scale_factor,
                        );
                    }
                }

                // Currently reading block
                let reading_block = imp.pdf_highlighter.borrow().get_highlighted_block();
                if let Some(reading_block) = reading_block.as_ref() {
                    for rect in reading_block.rectangles.iter() {
                        cr.rectangle(
                            rect.left().value as f64 * scale_factor,
                            page_size.top().value as f64 * scale_factor
                                - rect.top().value as f64 * scale_factor,
                            rect.width().value as f64 * scale_factor,
                            rect.height().value as f64 * scale_factor,
                        );
                    }
                }
                cr.fill().expect("Failed to fill base highlights");
            }
        ));

        drawing_area.add_controller(motion_controller);
        imp.overlay.add_overlay(drawing_area);
    }

    fn navigate_page(&self, delta: i32) {
        let imp = self.imp();
        if let Some(doc) = imp.pdf_wrapper.borrow().get_document() {
            let current = *imp.current_page_num.borrow() as i32;
            let new_page = (current + delta).max(0);

            if new_page < doc.pages().len() as i32 {
                *imp.current_page_num.borrow_mut() = new_page as u16;
                self.refresh_view();
            }
        }
    }

    fn go_to_page(&self, page_number: i32) {
        let imp = self.imp();
        if let Some(doc) = imp.pdf_wrapper.borrow().get_document() {
            if page_number >= 0 && page_number < doc.pages().len() as i32 {
                imp.current_page_num.replace(page_number as u16);
                self.refresh_view();
            }
        }
    }

    fn close_pdf(&self) {
        let imp = self.imp();
        self.imp().pdf_wrapper.borrow_mut().remove_pdf();
        *imp.current_page_num.borrow_mut() = 0;
        imp.current_page.set_text("1");
        imp.total_pages.set_text("1");
        imp.content_stack.set_visible_child_name("empty");
    }

    fn open_file_dialog(&self) {
        let file_dialog = file_dialog();
        let parent = self.root().and_downcast::<gtk::Window>();

        file_dialog.open(
            parent.as_ref(),
            None::<&gio::Cancellable>,
            clone!(
                #[weak(rename_to=this)]
                self,
                move |result| {
                    if let Ok(file) = result {
                        this.load_pdf(file);
                    }
                }
            ),
        );
    }

    fn init_audio_control_buttons(&self) {
        let imp = self.imp();
        imp.audio_controls.set_stop_handler(clone!(
            #[weak]
            imp,
            move || {
                imp.pdf_highlighter.borrow_mut().clear_highlight();
                imp.drawing_area.queue_draw();
            }
        ));

        imp.audio_controls.set_read_handler(clone!(
            #[weak]
            imp,
            #[weak(rename_to=this)]
            self,
            move |id: u32| {
                let current_page_num = *imp.current_page_num.borrow();
                let page = match imp.pdf_wrapper.borrow().get_document() {
                    Some(page) => page.pages().get(current_page_num).unwrap(),
                    None => {
                        dialogs::show_error_dialog("No PDF document loaded", &this);
                        return;
                    }
                };

                if imp.pdf_highlighter.borrow().is_pdf_page_empty(&page) {
                    dialogs::show_error_dialog(
                        "Page has no text content to read",
                        &imp.obj().clone().upcast::<gtk::Widget>(),
                    );
                    return;
                }
                let reading_blocks = imp.pdf_highlighter.borrow().get_reading_blocks_map();
                this.start_reading_pdf(reading_blocks, id);
            }
        ));
    }

    fn start_reading_pdf(&self, reading_blocks: BTreeMap<u32, PdfReadingBlock>, start_from: u32) {
        let imp = self.imp();
        if imp.audio_controls.imp().tts.is_playing() {
            let _ = runtime().block_on(imp.audio_controls.imp().tts.stop(true));
        }

        glib::spawn_future_local(clone!(
            #[weak]
            imp,
            #[weak(rename_to=this)]
            self,
            async move {
                let mut subscriber = imp.audio_controls.imp().tts.sender.subscribe();

                while let Ok(event) = subscriber.recv().await {
                    match event {
                        TTSEvent::Progress { block_id } => {
                            imp.pdf_highlighter.borrow_mut().highlight(block_id);
                            imp.highlight_area.queue_draw();
                        }
                        TTSEvent::Error(e) => {
                            dialogs::show_error_dialog(&e, &this);
                            imp.pdf_highlighter.borrow_mut().clear_highlight();
                            this.refresh_view();
                            break;
                        }
                        TTSEvent::Next | TTSEvent::Prev => {
                            imp.pdf_highlighter.borrow_mut().clear_highlight();
                            imp.highlight_area.queue_draw();
                        }
                        TTSEvent::Stop => {
                            imp.pdf_highlighter.borrow_mut().clear_highlight();
                            imp.highlight_area.queue_draw();
                            break;
                        }
                    }
                }
            }
        ));

        glib::spawn_future_local(clone!(
            #[weak]
            imp,
            #[weak(rename_to=this)]
            self,
            async move {
                if let Some(voice) = imp.audio_controls.get_selected_voice_key() {
                    if let Err(e) = imp
                        .audio_controls
                        .imp()
                        .tts
                        .read_blocks_by_voice(voice, reading_blocks, start_from)
                        .await
                    {
                        let err_msg = format!("Error while reading text by given voice: {}", e);
                        dialogs::show_error_dialog(&err_msg, &this);
                    }

                    imp.pdf_highlighter.borrow_mut().clear_highlight();
                    this.refresh_view();
                }
            }
        ));
    }

    fn scale_pdf(&self, factor: f32) {
        self.imp()
            .scale_factor
            .replace_with(|old| (*old + factor).max(0.5));
        self.refresh_view();
    }

    fn get_rgba_colors(&self) -> (f32, f32, f32) {
        let highlight_color = SETTINGS.get_highlight_rgba();
        let (red, green, blue) = (
            highlight_color.red(),
            highlight_color.green(),
            highlight_color.blue(),
        );

        (red, green, blue)
    }
}

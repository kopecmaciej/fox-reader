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
use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::{config::UserConfig, core::tts::TTSEvent};

use super::dialogs::{self, show_error_dialog};

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
        pub pdf_content: TemplateChild<gtk::Box>,
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

        //TODO: Find better way of sharing user_config
        pub user_config: RefCell<Rc<RefCell<UserConfig>>>,
        pub scale_factor: RefCell<f32>,
        pub pdf_wrapper: RefCell<PdfiumWrapper>,
        pub current_page_num: RefCell<PdfPageIndex>,
        pub pdf_highlighter: RefCell<PdfHighlighter>,
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
    pub fn init(&self, user_config: Rc<RefCell<UserConfig>>) {
        let imp = self.imp();
        imp.audio_controls.init();
        self.init_audio_control_buttons();
        imp.scale_factor.replace(1.0);
        imp.user_config.replace(user_config);
    }

    fn load_pdf(&self, file: gio::File) {
        let path = match file.path() {
            Some(path) => path,
            None => {
                show_error_dialog("Could not get file path", self);
                return;
            }
        };
        let _ = self.imp().pdf_wrapper.borrow_mut().load_document(&path);

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

                self.create_drawing_area(page, 9999999, width, height);
                imp.current_page.set_text(&(current_page + 1).to_string());
            }
            Err(e) => {
                eprintln!("Error rendering PDF: {}", e);
                show_error_dialog(&format!("Error rendering PDF: {}", e), self);
            }
        }
    }

    fn create_drawing_area(
        &self,
        page: PdfPage,
        reading_index: u32,
        width: PdfPoints,
        height: PdfPoints,
    ) {
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

        if imp.user_config.borrow().borrow().is_dark_color_scheme() {
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
        let page_size = page.page_size();

        let hovered_rect = Rc::new(RefCell::new(Option::<usize>::None));
        let hovered_rect_clone = Rc::clone(&hovered_rect);

        let motion_controller = gtk::EventControllerMotion::new();
        let all_rect = imp.pdf_highlighter.borrow().get_reading_blocks();

        // After user click from this point we'll start reading
        let click_controller = gtk::GestureClick::new();

        let hover_rect = all_rect.to_vec();
        let hover_rect_clone = hover_rect.to_vec();
        let click_rect = hover_rect.clone();

        click_controller.connect_pressed(clone!(
            #[weak]
            imp,
            move |_, _, x, y| {
                let clicked_block = click_rect.iter().find_map(|block| {
                    let scale_factor = scale_factor as f64;
                    block.rectangles.iter().find_map(|rect| {
                        let scaled_rect = (
                            rect.left().value as f64 * scale_factor,
                            page_size.top().value as f64 * scale_factor
                                - rect.top().value as f64 * scale_factor,
                            rect.width().value as f64 * scale_factor,
                            rect.height().value as f64 * scale_factor,
                        );

                        if x >= scaled_rect.0
                            && x <= scaled_rect.0 + scaled_rect.2
                            && y >= scaled_rect.1
                            && y <= scaled_rect.1 + scaled_rect.3
                        {
                            Some(block.id)
                        } else {
                            None
                        }
                    })
                });

                if let Some(block_id) = clicked_block {
                    if let Some(voice) = imp.audio_controls.get_selected_voice() {
                        let reading_blocks = imp
                            .pdf_highlighter
                            .borrow()
                            .get_reading_blocks_from_id(block_id);

                        glib::spawn_future_local(clone!(
                            #[weak]
                            imp,
                            async move {
                                if let Err(e) = imp
                                    .audio_controls
                                    .imp()
                                    .tts
                                    .read_blocks_by_voice(voice, reading_blocks)
                                    .await
                                {
                                    let err_msg =
                                        format!("Error while reading text by given voice: {}", e);
                                    //dialogs::show_error_dialog(&err_msg, gesture_click);
                                }

                                imp.pdf_highlighter.borrow_mut().clear_highlight();
                            }
                        ));
                    }
                }
            }
        ));
        drawing_area.add_controller(click_controller);

        let reading_block = all_rect
            .iter()
            .find(|block| block.id == reading_index)
            .cloned();

        // Maybe not neccessary but usable following user cursor
        motion_controller.connect_motion(clone!(
            #[weak]
            drawing_area,
            move |_, x, y| {
                let mut current_hover = hovered_rect_clone.borrow_mut();

                let new_hover = hover_rect.iter().enumerate().find_map(|(index, block)| {
                    let scale_factor = scale_factor as f64;
                    block.rectangles.iter().find_map(|rect| {
                        let scaled_rect = (
                            rect.left().value as f64 * scale_factor,
                            page_size.top().value as f64 * scale_factor
                                - rect.top().value as f64 * scale_factor,
                            rect.width().value as f64 * scale_factor,
                            rect.height().value as f64 * scale_factor,
                        );

                        if x >= scaled_rect.0
                            && x <= scaled_rect.0 + scaled_rect.2
                            && y >= scaled_rect.1
                            && y <= scaled_rect.1 + scaled_rect.3
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
        drawing_area.add_controller(motion_controller);

        let (red, green, blue) = self.get_rgba_colors();
        drawing_area.set_draw_func(move |_, cr: &Context, _width, _height| {
            cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
            cr.paint().expect("Failed to paint PDF page");

            // Readable block
            cr.set_source_rgba(red.into(), green.into(), blue.into(), 0.3);
            let scale_factor = scale_factor as f64;
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

            // Hovered block
            if let Some(hover_index) = *hovered_rect.borrow() {
                cr.set_source_rgba(red.into(), green.into(), blue.into(), 0.6);
                let highlighted_block = &hover_rect_clone[hover_index];

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
            cr.fill().expect("Failed to fill base highlights");
        });
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

    fn refresh_view(&self) {
        let imp = self.imp();
        if let Some(doc) = imp.pdf_wrapper.borrow().get_document() {
            self.render_current_page(doc);
        }
    }

    pub fn scale_pdf(&self, factor: f32) {
        self.imp()
            .scale_factor
            .replace_with(|old| (*old + factor).max(0.5));
        self.refresh_view();
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
        let file_chooser = gtk::FileDialog::builder()
            .title("Open PDF")
            .accept_label("Open")
            .build();

        let filter = gtk::FileFilter::new();
        filter.add_mime_type("application/pdf");
        filter.set_name(Some("PDF files"));
        file_chooser.set_default_filter(Some(&filter));

        let parent = self
            .root()
            .and_then(|r| r.clone().downcast::<gtk::Window>().ok());

        file_chooser.open(
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

    pub fn init_audio_control_buttons(&self) {
        let imp = self.imp();
        imp.audio_controls.set_stop_handler(clone!(
            #[weak]
            imp,
            move || {
                imp.pdf_highlighter.borrow_mut().clear_highlight();
                // Refresh the view to clear highlights
                if let Some(parent) = imp.pdf_content.parent() {
                    if let Some(widget) = parent.downcast_ref::<gtk::Widget>() {
                        widget.queue_draw();
                    }
                }
            }
        ));

        imp.audio_controls.set_read_handler(clone!(
            #[weak]
            imp,
            #[weak(rename_to=this)]
            self,
            move |voice: String, button: &gtk::Button| {
                // Get the current page
                let current_page_num = *imp.current_page_num.borrow();
                let page = match imp.pdf_wrapper.borrow().get_document() {
                    Some(page) => page.pages().get(current_page_num).unwrap(),
                    None => {
                        dialogs::show_error_dialog("No PDF document loaded", button);
                        return;
                    }
                };

                if imp.pdf_highlighter.borrow().is_pdf_page_empty(&page) {
                    dialogs::show_error_dialog("Page has no text content to read", button);
                    return;
                }

                // Handle TTSEvent progress updates
                glib::spawn_future_local(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    this,
                    #[weak]
                    button,
                    async move {
                        let mut subscriber = imp.audio_controls.imp().tts.sender.subscribe();

                        while let Ok(event) = subscriber.recv().await {
                            match event {
                                TTSEvent::Progress { block_id } => {
                                    imp.pdf_highlighter.borrow_mut().highlight(block_id);

                                    this.refresh_view_with_highlight(block_id);
                                }
                                TTSEvent::Error(e) => {
                                    dialogs::show_error_dialog(&e, &button);
                                    imp.pdf_highlighter.borrow_mut().clear_highlight();
                                    this.refresh_view();
                                    break;
                                }
                                TTSEvent::Next | TTSEvent::Prev => {
                                    imp.pdf_highlighter.borrow_mut().clear_highlight();
                                    this.refresh_view();
                                }
                                TTSEvent::Stop => {
                                    imp.pdf_highlighter.borrow_mut().clear_highlight();
                                    this.refresh_view();
                                    break;
                                }
                            }
                        }
                    }
                ));

                let reading_blocks = imp.pdf_highlighter.borrow().get_reading_blocks();
                glib::spawn_future_local(clone!(
                    #[weak]
                    imp,
                    #[weak]
                    this,
                    #[weak]
                    button,
                    async move {
                        if let Err(e) = imp
                            .audio_controls
                            .imp()
                            .tts
                            .read_blocks_by_voice(voice, reading_blocks.to_vec())
                            .await
                        {
                            let err_msg = format!("Error while reading text by given voice: {}", e);
                            dialogs::show_error_dialog(&err_msg, &button);
                        }

                        imp.pdf_highlighter.borrow_mut().clear_highlight();
                        this.refresh_view();
                    }
                ));
            }
        ));
    }

    fn refresh_view_with_highlight(&self, block_id: u32) {
        let imp = self.imp();

        if let Some(doc) = imp.pdf_wrapper.borrow().get_document() {
            let current_page = *imp.current_page_num.borrow();

            let page = doc.pages().get(current_page).unwrap();
            let (width, height) = (page.width(), page.height());

            self.create_drawing_area(page, block_id, width, height);
        }
    }

    pub fn get_rgba_colors(&self) -> (f32, f32, f32) {
        let highlight_color = self
            .imp()
            .user_config
            .borrow()
            .borrow()
            .get_highlight_rgba();
        let (red, green, blue) = (
            highlight_color.red(),
            highlight_color.green(),
            highlight_color.blue(),
        );

        (red, green, blue)
    }
}

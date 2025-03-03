use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::glib::{self, Object};
use gtk::{
    self,
    cairo::Context,
    gdk_pixbuf::{Colorspace, Pixbuf},
    glib::clone,
};
use pdfium_render::prelude::{PdfDocument, PdfPage, PdfPoints, PdfRect, PdfRenderConfig};
use std::{cell::RefCell, fmt::Debug};

use crate::core::tts::TTSEvent;

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
    pub fn init(&self, highlight_color: gtk::gdk::RGBA) {
        let imp = self.imp();
        imp.audio_controls.init();
        self.init_audio_control_buttons();
        imp.scale_factor.replace(1.0);
        self.set_highlight_color(highlight_color);
    }

    pub fn set_highlight_color(&self, rgba: gtk::gdk::RGBA) {
        self.imp()
            .pdf_highlighter
            .borrow_mut()
            .set_highlight_color(rgba);
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

                self.create_drawing_area(page, &Vec::new(), width, height);
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
        rectangles: &[PdfRect],
        width: PdfPoints,
        height: PdfPoints,
    ) {
        let scale_factor = *self.imp().scale_factor.borrow();
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

        // TODO: only if dark theme is on
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

        let drawing_area = &self.imp().drawing_area;
        drawing_area.set_content_width(img_width as i32);
        drawing_area.set_content_height(img_height as i32);
        let page_size = page.page_size();

        let rec = rectangles.to_vec();

        let (red, blue, green) = self.imp().pdf_highlighter.borrow().get_rgba_colors();
        drawing_area.set_draw_func(move |_, cr: &Context, _width, _height| {
            cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
            cr.paint().expect("Failed to paint PDF page");

            cr.set_source_rgba(red.into(), green.into(), blue.into(), 0.3);

            let scale_factor = scale_factor as f64;
            for rect in rec.iter() {
                cr.rectangle(
                    rect.left().value as f64 * scale_factor,
                    page_size.top().value as f64 * scale_factor
                        - rect.top().value as f64 * scale_factor,
                    rect.width().value as f64 * scale_factor,
                    rect.height().value as f64 * scale_factor,
                );
            }
            cr.fill().expect("Failed to fill highlight");
        });
    }

    fn navigate_page(&self, delta: i32) {
        let imp = self.imp();
        if let Some(doc) = imp.pdf_wrapper.borrow().get_document() {
            let current = *imp.current_page_num.borrow() as i32;
            let new_page = current + delta;

            if new_page >= 0 && new_page < doc.pages().len() as i32 {
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
                imp.pdf_highlighter.borrow().clear();
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

                if imp
                    .pdf_highlighter
                    .borrow()
                    .generate_reading_blocks(page)
                    .is_err()
                {
                    dialogs::show_error_dialog(
                        "Error while parsing pdf into blocks that could be read",
                        button,
                    );
                    return;
                }

                let reading_blocks = imp.pdf_highlighter.borrow().get_reading_blocks();

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
                                    imp.pdf_highlighter.borrow().highlight(block_id);

                                    if let Some(current_block) =
                                        reading_blocks.iter().find(|b| b.id == block_id)
                                    {
                                        this.refresh_view_with_highlight(&current_block.rectangles);
                                    }
                                }
                                TTSEvent::Error(e) => {
                                    dialogs::show_error_dialog(&e, &button);
                                    imp.pdf_highlighter.borrow().clear();
                                    this.refresh_view();
                                    break;
                                }
                                TTSEvent::Next | TTSEvent::Prev => {
                                    imp.pdf_highlighter.borrow().clear();
                                    this.refresh_view();
                                }
                                TTSEvent::Stop => {
                                    imp.pdf_highlighter.borrow().clear();
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

                        imp.pdf_highlighter.borrow().clear();
                        this.refresh_view();
                    }
                ));
            }
        ));
    }

    fn refresh_view_with_highlight(&self, highlight_rect: &[PdfRect]) {
        let imp = self.imp();

        if let Some(doc) = imp.pdf_wrapper.borrow().get_document() {
            let current_page = *imp.current_page_num.borrow();

            let page = doc.pages().get(current_page).unwrap();
            let (width, height) = (page.width(), page.height());

            self.create_drawing_area(page, highlight_rect, width, height);
        }
    }
}

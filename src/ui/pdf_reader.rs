use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::glib::{self, Object};
use gtk::{
    self,
    cairo::Context,
    gdk_pixbuf::{Colorspace, Pixbuf},
    glib::clone,
};
use pdfium_render::prelude::{
    PdfDocument, PdfPage, PdfPageObjectsCommon, PdfPoints, PdfRect, PdfRenderConfig, Pdfium,
};
use std::{cell::RefCell, fmt::Debug, rc::Rc};

use crate::{core::tts::TTSEvent, utils::pdfium::PdfiumWrapper};

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
        pub audio_controls: TemplateChild<AudioControls>,

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
        self.init_audio_control_buttons();
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
                let drawing_area = Self::create_drawing_area(page, None, width, height);
                let scrolled_window = Self::create_scrolled_window(&drawing_area);

                Self::clear_container(imp.pdf_content.as_ref());
                imp.pdf_content.append(&scrolled_window);
                imp.current_page.set_text(&(current_page + 1).to_string());
            }
            Err(e) => {
                eprintln!("Error rendering PDF: {}", e);
                show_error_dialog(&format!("Error rendering PDF: {}", e), self);
            }
        }
    }

    fn create_drawing_area(
        page: PdfPage,
        rect: Option<PdfRect>,
        width: PdfPoints,
        height: PdfPoints,
    ) -> gtk::DrawingArea {
        let scale_factor = 1.5;
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
        let rgba_image = dynamic_image.to_rgba8();
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

        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_content_width(img_width as i32);
        drawing_area.set_content_height(img_height as i32);
        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);
        let page_size = page.page_size();

        drawing_area.set_draw_func(move |_, cr: &Context, _width, _height| {
            // First, draw the PDF page image.
            cr.set_source_pixbuf(&pixbuf, 0.0, 0.0);
            cr.paint().expect("Failed to paint PDF page");

            // Now overlay the highlights with proper scaling
            cr.set_source_rgba(1.0, 1.0, 0.0, 0.5);
            let scale_factor = scale_factor as f64;
            if let Some(rect) = rect {
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

        drawing_area
    }

    fn create_scrolled_window(child: &gtk::DrawingArea) -> gtk::ScrolledWindow {
        let scrolled_window = gtk::ScrolledWindow::new();
        scrolled_window.set_hexpand(true);
        scrolled_window.set_vexpand(true);
        scrolled_window.set_child(Some(child));
        scrolled_window
    }

    fn clear_container(container: &gtk::Box) {
        while let Some(child) = container.first_child() {
            container.remove(&child);
        }
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
        Self::clear_container(imp.pdf_content.as_ref());
        if let Some(doc) = imp.pdf_wrapper.borrow().get_document() {
            self.render_current_page(doc);
        }
    }

    fn close_pdf(&self) {
        let imp = self.imp();
        //let rc_doc = &mut self.imp().pdf_document;
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

                // Check if page is empty
                if imp.pdf_highlighter.borrow().is_pdf_page_empty(&page) {
                    dialogs::show_error_dialog("Page has no text content to read", button);
                    return;
                }

                // Generate reading blocks
                imp.pdf_highlighter.borrow().generate_reading_blocks(page);

                // Get reading blocks
                let reading_blocks = match imp.pdf_highlighter.borrow().get_reading_blocks() {
                    Some(blocks) => blocks,
                    None => {
                        dialogs::show_error_dialog("Failed to generate reading blocks", button);
                        return;
                    }
                };

                if reading_blocks.is_empty() {
                    dialogs::show_error_dialog("No readable content found on this page", button);
                    return;
                }

                // Create a clone of reading blocks for the progress handler
                let reading_blocks_clone = reading_blocks.clone();

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
                                    // Highlight the current block
                                    imp.pdf_highlighter.borrow().highlight(block_id);

                                    // Find the current reading block to get its rectangle
                                    if let Some(current_block) =
                                        reading_blocks_clone.iter().find(|b| b.id == block_id)
                                    {
                                        // Update the current reading rectangle and refresh the view
                                        this.update_highlight_rectangle(current_block.rectangle);
                                    }
                                }
                                TTSEvent::Error(e) => {
                                    dialogs::show_error_dialog(&e, &button);
                                    imp.pdf_highlighter.borrow().clear();
                                    this.clear_highlight_rectangle();
                                    break;
                                }
                                TTSEvent::Next | TTSEvent::Prev => {
                                    imp.pdf_highlighter.borrow().clear();
                                    this.clear_highlight_rectangle();
                                }
                                TTSEvent::Stop => {
                                    imp.pdf_highlighter.borrow().clear();
                                    this.clear_highlight_rectangle();
                                    break;
                                }
                            }
                        }
                    }
                ));

                // Start the TTS reading
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
                            .read_block_by_voice(&voice, reading_blocks)
                            .await
                        {
                            let err_msg = format!("Error while reading text by given voice: {}", e);
                            dialogs::show_error_dialog(&err_msg, &button);
                        }

                        // Clean up when reading is done
                        imp.pdf_highlighter.borrow().clear();
                        this.clear_highlight_rectangle();
                    }
                ));
            }
        ));
    }

    pub fn update_highlight_rectangle(&self, rectangle: PdfRect) {
        // Store the current rectangle in the PdfReader state if needed
        // For now we'll just refresh the view with the new rectangle
        self.refresh_view_with_highlight(rectangle);
    }

    // Clear the highlight rectangle
    pub fn clear_highlight_rectangle(&self) {
        // Clear any stored rectangle and refresh the view
        self.refresh_view();
    }

    // Refresh the view with a specific highlight rectangle
    fn refresh_view_with_highlight(&self, highlight_rect: PdfRect) {
        let imp = self.imp();

        if let Some(doc) = imp.pdf_wrapper.borrow().get_document() {
            let current_page = *imp.current_page_num.borrow();

            let page = doc.pages().get(current_page).unwrap();
            let (width, height) = (page.width(), page.height());

            // Create drawing area with the current highlight rectangle
            let drawing_area = Self::create_drawing_area(page, Some(highlight_rect), width, height);
            let scrolled_window = Self::create_scrolled_window(&drawing_area);

            // Replace the current content with the updated one
            Self::clear_container(imp.pdf_content.as_ref());
            imp.pdf_content.append(&scrolled_window);
        }
    }
}

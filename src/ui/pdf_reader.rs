use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::glib::{self, Object};
use gtk::{self, glib::clone};
use poppler::Rectangle;
use std::{cell::RefCell, fmt::Debug};

use poppler::Document;

use crate::core::tts::TTSEvent;

use super::{
    dialogs::{self, show_error_dialog},
    voice_row::VoiceRow,
};

mod imp {
    use crate::{ui::audio_controls::AudioControls, utils::pdf_highlighter::PdfHighlighter};

    use super::*;
    use gtk::CompositeTemplate;

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
        pub pdf_view: TemplateChild<gtk::Box>,
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

        pub pdf_document: RefCell<Option<Document>>,
        pub current_page_num: RefCell<i32>,
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

    pub fn load_pdf(&self, file: gio::File) {
        let path_str = file.path().and_then(|p| p.to_str().map(|s| s.to_owned()));

        if let Some(str_path) = path_str {
            match Document::from_file(&format!("file:///{}", str_path), None) {
                Ok(doc) => {
                    let imp = self.imp();
                    *imp.current_page_num.borrow_mut() = 0;
                    *imp.pdf_document.borrow_mut() = Some(doc);

                    if let Some(ref doc) = *imp.pdf_document.borrow() {
                        imp.total_pages.set_text(&doc.n_pages().to_string());
                    }

                    self.render_current_page();
                    imp.content_stack.set_visible_child_name("pdf_view");
                }
                Err(e) => show_error_dialog(&format!("Error loading PDF: {}", e), self),
            }
        } else {
            show_error_dialog("Error retrieving or converting file path", self);
        }
    }

    fn render_current_page(&self) {
        let imp = self.imp();
        let current_page = *imp.current_page_num.borrow();

        if let Some(ref doc) = *imp.pdf_document.borrow() {
            if let Some(page) = doc.page(current_page) {
                let (width, height) = page.size();
                let rect = self
                    .get_text_positions(current_page as usize)
                    .unwrap_or_default();

                let drawing_area = Self::create_drawing_area(page.clone(), rect, width, height);
                let scrolled_window = Self::create_scrolled_window(&drawing_area);

                Self::clear_container(imp.pdf_content.as_ref());
                imp.pdf_content.append(&scrolled_window);
                imp.current_page.set_text(&(current_page + 1).to_string());
            }
        }
    }

    fn create_drawing_area(
        page: poppler::Page,
        rec: Rectangle,
        width: f64,
        height: f64,
    ) -> gtk::DrawingArea {
        let drawing_area = gtk::DrawingArea::new();
        drawing_area.set_content_width(width as i32);
        drawing_area.set_content_height((height + 240.0) as i32);
        drawing_area.set_hexpand(true);
        drawing_area.set_vexpand(true);

        drawing_area.set_draw_func(move |da, cr, draw_width, _| {
            let (page_width, page_height) = page.size();
            let scale = draw_width as f64 / page_width;
            cr.scale(scale, scale);

            page.render(cr);

            let highlight_x = rec.x1();
            let highlight_y = page_height - rec.y2();
            let highlight_width = rec.x2() - rec.x1();
            let highlight_height = rec.y2() - rec.y1();

            cr.set_source_rgba(1.0, 1.0, 0.0, 0.5);
            cr.rectangle(highlight_x, highlight_y, highlight_width, highlight_height);
            if let Err(e) = cr.fill() {
                show_error_dialog(&format!("Error rendering PDF: {}", e), da);
            }
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
        if let Some(ref doc) = *imp.pdf_document.borrow() {
            let current = *imp.current_page_num.borrow();
            let new_page = current + delta;

            if new_page >= 0 && new_page < doc.n_pages() {
                *imp.current_page_num.borrow_mut() = new_page;
                self.refresh_view();
            }
        }
    }

    fn go_to_page(&self, page: i32) {
        let imp = self.imp();
        if let Some(ref doc) = *imp.pdf_document.borrow() {
            if page >= 0 && page < doc.n_pages() {
                *imp.current_page_num.borrow_mut() = page;
                self.refresh_view();
            }
        }
    }

    fn refresh_view(&self) {
        let imp = self.imp();
        Self::clear_container(imp.pdf_content.as_ref());
        self.render_current_page();
    }

    fn close_pdf(&self) {
        let imp = self.imp();
        *imp.pdf_document.borrow_mut() = None;
        *imp.current_page_num.borrow_mut() = 0;
        imp.current_page.set_text("1");
        imp.total_pages.set_text("1");
        imp.content_stack.set_visible_child_name("empty");
    }

    pub fn get_text_positions(&self, page_index: usize) -> Option<Rectangle> {
        let imp = self.imp();
        if let Some(ref doc) = *imp.pdf_document.borrow() {
            if let Some(page) = doc.page(page_index as i32) {
                let page_text = page.text().unwrap_or_default();
                if page_text.is_empty() {
                    return None;
                }
                let mut position = Rectangle::new();

                for (n, word) in page_text.split_whitespace().enumerate() {
                    if n == 1 {
                        break;
                    }
                    if let Some(rect) = page.find_text(word).first() {
                        position = *rect;
                    }
                    if word.ends_with('.') {
                        break;
                    }
                }
                return Some(position);
            }
        }
        None
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
                let current_page = match imp.pdf_document.borrow().as_ref() {
                    Some(doc) => doc.page(current_page_num),
                    None => {
                        dialogs::show_error_dialog("No PDF document loaded", button);
                        return;
                    }
                };

                let current_page = match current_page {
                    Some(page) => page,
                    None => {
                        dialogs::show_error_dialog("Failed to load page", button);
                        return;
                    }
                };

                // Check if page is empty
                if imp
                    .pdf_highlighter
                    .borrow()
                    .is_pdf_page_empty(current_page.clone())
                {
                    dialogs::show_error_dialog("Page has no text content to read", button);
                    return;
                }

                // Generate reading blocks
                imp.pdf_highlighter
                    .borrow()
                    .generate_reading_blocks(current_page.clone());

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

    pub fn update_highlight_rectangle(&self, rectangle: Rectangle) {
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
    fn refresh_view_with_highlight(&self, highlight_rect: Rectangle) {
        let imp = self.imp();

        if let Some(ref doc) = *imp.pdf_document.borrow() {
            let current_page = *imp.current_page_num.borrow();

            if let Some(page) = doc.page(current_page) {
                let (width, height) = page.size();

                // Create drawing area with the current highlight rectangle
                let drawing_area =
                    Self::create_drawing_area(page.clone(), highlight_rect, width, height);
                let scrolled_window = Self::create_scrolled_window(&drawing_area);

                // Replace the current content with the updated one
                Self::clear_container(imp.pdf_content.as_ref());
                imp.pdf_content.append(&scrolled_window);
            }
        }
    }
}

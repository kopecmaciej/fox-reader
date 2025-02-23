use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::glib::{self, Object};
use gtk::{self, glib::clone};
use poppler::Rectangle;
use std::{cell::RefCell, fmt::Debug};

use poppler::Document;

use super::dialogs::show_error_dialog;

mod imp {
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

        pub pdf_document: RefCell<Option<Document>>,
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
        }

        fn dispose(&self) {
            self.content_stack.unparent();
        }
    }

    impl WidgetImpl for PdfReader {}
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
    pub fn load_pdf(&self, file: gio::File) {
        if let Some(path) = file.path() {
            if let Some(str_path) = path.to_str() {
                match Document::from_file(&format!("file:///{}", str_path), None) {
                    Ok(doc) => {
                        let imp = self.imp();
                        *imp.pdf_document.borrow_mut() = Some(doc);

                        let pdf_box: &gtk::Box = imp.pdf_view.as_ref();
                        while let Some(child) = pdf_box.first_child() {
                            pdf_box.remove(&child);
                        }

                        self.render_current_page();
                        imp.content_stack.set_visible_child_name("pdf_view");
                    }
                    Err(e) => {
                        show_error_dialog(&format!("Error loading PDF: {}", e), self);
                    }
                }
            } else {
                show_error_dialog("Error converting file path to string", self);
            }
        } else {
            show_error_dialog("Error retrieving file path", self);
        }
    }

    fn render_current_page(&self) {
        let imp = self.imp();
        if let Some(ref doc) = *imp.pdf_document.borrow() {
            if let Some(page) = doc.page(0) {
                let (width, height) = page.size();

                let rec = self.get_text_positions(0).unwrap();

                let drawing_area = gtk::DrawingArea::new();
                drawing_area.set_content_width(width as i32);
                drawing_area.set_content_height((height + 240.0) as i32);
                drawing_area.set_hexpand(true);
                drawing_area.set_vexpand(true);

                let scrolled_window = gtk::ScrolledWindow::new();
                scrolled_window.set_hexpand(true);
                scrolled_window.set_vexpand(true);
                scrolled_window.set_child(Some(&drawing_area));

                drawing_area.set_draw_func(move |da, cr, draw_width, _draw_height| {
                    let (page_width, page_height) = page.size();
                    let scale = draw_width as f64 / page_width;
                    cr.scale(scale, scale);

                    page.render(cr);

                    let x1 = rec.x1();
                    let x2 = rec.x2();
                    let y1 = rec.y1();
                    let y2 = rec.y2();

                    let highlight_x = x1;
                    let highlight_y = page_height - y2; // we have to reverse the `y` coordinate
                    let highlight_width = x2 - x1;
                    let highlight_height = y2 - y1;

                    cr.set_source_rgba(1.0, 1.0, 0.0, 0.5);
                    cr.rectangle(highlight_x, highlight_y, highlight_width, highlight_height);
                    if let Err(e) = cr.fill() {
                        show_error_dialog(&format!("Error rendering PDF: {}", e), da);
                    }
                });

                let pdf_box: &gtk::Box = imp.pdf_view.as_ref();
                pdf_box.append(&scrolled_window);
            }
        }
    }

    pub fn get_text_positions(&self, page_index: usize) -> Option<Rectangle> {
        let imp = self.imp();
        if let Some(ref doc) = *imp.pdf_document.borrow() {
            if let Some(page) = doc.page(page_index as i32) {
                let page_text = page.text().unwrap_or_else(|| "".into());
                if page_text == "" {
                    return None;
                }
                let mut position = Rectangle::new();

                for (n, word) in page_text.split_whitespace().enumerate() {
                    if n == 1 {
                        break;
                    }
                    let rects = page.find_text(word);
                    if let Some(rect) = rects.first() {
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
}

use adw::prelude::*;
use adw::subclass::prelude::*;
use gio::glib::{self, Object};
use gtk::{self, glib::clone};
use std::cell::RefCell;

use poppler::PopplerDocument;

#[derive(Debug, Clone)]
struct TextSelection {
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    text: String,
}

mod imp {
    use std::collections::HashSet;

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

        pub pdf_document: RefCell<Option<PopplerDocument>>,
        pub text_selections: RefCell<HashSet<TextSelection>>,
        pub current_selection: RefCell<Option<(f64, f64)>>, // Store mouse
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

            // Set up drop target
            self.drop_area.connect_drop(clone!(
                #[weak]
                obj,
                #[upgrade_or]
                false,
                move |_, value, _, _| -> bool {
                    println!("TEST");
                    println!("{:?}", value);
                    if let Ok(file) = value.get::<gio::File>() {
                        println!("{:?}", file);
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
            match PopplerDocument::new_from_file(&path, None) {
                Ok(doc) => {
                    let imp = self.imp();
                    *imp.pdf_document.borrow_mut() = Some(doc);

                    // Clear any existing content in the PDF view
                    let pdf_box: &gtk::Box = imp.pdf_view.as_ref();
                    while let Some(child) = pdf_box.first_child() {
                        pdf_box.remove(&child);
                    }

                    self.render_current_page();

                    // Switch to the PDF view stack page
                    imp.content_stack.set_visible_child_name("pdf_view");
                }
                Err(err) => {
                    eprintln!("Error loading PDF: {}", err);
                    // Show an error dialog
                }
            }
        }
    }

    fn render_current_page(&self) {
        let imp = self.imp();
        if let Some(ref doc) = *imp.pdf_document.borrow() {
            if let Some(page) = doc.get_page(0) {
                let (width, height) = page.get_size();

                let drawing_area = gtk::DrawingArea::new();
                drawing_area.set_content_width(width as i32);
                drawing_area.set_content_height(height as i32);
                drawing_area.set_hexpand(true);
                drawing_area.set_vexpand(true);

                // Add ScrolledWindow to handle large PDFs
                let scrolled_window = gtk::ScrolledWindow::new();
                scrolled_window.set_hexpand(true);
                scrolled_window.set_vexpand(true);
                scrolled_window.set_child(Some(&drawing_area));

                drawing_area.set_draw_func(move |_, cr, width, height| {
                    let scale = width as f64 / page.get_size().0;
                    cr.scale(scale, scale);
                    page.render(cr);
                });

                let pdf_box: &gtk::Box = imp.pdf_view.as_ref();
                pdf_box.append(&scrolled_window);
            }
        }
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

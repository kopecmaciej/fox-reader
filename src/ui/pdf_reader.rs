use adw::prelude::*;
use adw::subclass::prelude::*;
use cairo::{Context, Format, ImageSurface};
use gio::glib::{self, Object};
use gtk::{self, glib::clone};
use poppler::PopplerDocument;

mod imp {
    use super::*;
    use gtk::CompositeTemplate;
    use std::cell::RefCell;

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
        pub current_file: RefCell<Option<gio::File>>,
        pub pdf_surface: RefCell<Option<cairo::PdfSurface>>,
        pub current_page: RefCell<usize>,
        pub pdf_document: RefCell<Option<PopplerDocument>>,
        pub page_surface: RefCell<Option<gtk::Picture>>,
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

            self.current_page.replace(0);

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

            self.open_pdf.connect_clicked(clone!(
                #[weak]
                obj,
                move |_| {
                    obj.open_file_dialog();
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
        let imp = self.imp();

        if let Some(path) = file.path() {
            if let Ok(document) = PopplerDocument::new_from_file(path, None) {
                imp.pdf_document.replace(Some(document));
                imp.current_file.replace(Some(file));
                imp.current_page.replace(0);
                self.render_current_page();
                imp.content_stack.set_visible_child(&*imp.pdf_view);
            } else {
                eprintln!("Failed to load PDF document");
            }
        }
    }

    fn render_current_page(&self) {
        let imp = self.imp();

        if let Some(document) = imp.pdf_document.borrow().as_ref() {
            let current_page = *imp.current_page.borrow();

            if let Some(page) = document.get_page(current_page) {
                let (width, height) = page.get_size();

                // Create a surface to render the page
                let mut surface = ImageSurface::create(Format::Rgb24, width as i32, height as i32)
                    .expect("Failed to create image surface");

                let context = Context::new(&surface).expect("Failed to create Cairo context");

                // Fill background with white
                context.set_source_rgb(1.0, 1.0, 1.0);
                context.paint().expect("Failed to paint background");

                // Render the PDF page
                context.save().expect("Failed to save context");
                page.render(&context);
                context.restore().expect("Failed to restore context");

                // Ensure surface is finished
                surface.flush();

                // Create a new buffer and copy the surface data
                let stride = surface.stride();
                let height = surface.height();
                let mut data = vec![0u8; (stride * height) as usize];

                if let Ok(surface_data) = surface.data() {
                    data.copy_from_slice(&surface_data);

                    let pixbuf = gtk::gdk_pixbuf::Pixbuf::from_mut_slice(
                        data,
                        gtk::gdk_pixbuf::Colorspace::Rgb,
                        false,
                        8,
                        width as i32,
                        height,
                        stride,
                    );

                    let texture = gtk::gdk::Texture::for_pixbuf(&pixbuf);
                    let picture = gtk::Picture::new();
                    picture.set_paintable(Some(&texture));
                    picture.set_can_shrink(true);
                    picture.set_keep_aspect_ratio(true);

                    if let Some(old_surface) = imp.page_surface.take() {
                        imp.pdf_view.remove(&old_surface);
                    }

                    imp.pdf_view.append(&picture);
                    imp.page_surface.replace(Some(picture));
                } else {
                    eprintln!("Failed to access surface data");
                };
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

        file_chooser.open(
            self.root()
                .as_ref()
                .and_then(|r| r.clone().downcast::<gtk::Window>().ok())
                .as_ref(),
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

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};

use super::keybindings::KeyBindingManager;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/kopecmaciej/fox-reader/ui/about_dialog.ui")]
    pub struct AboutDialog {
        #[template_child]
        pub keybindings_list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub github_row: TemplateChild<adw::ActionRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AboutDialog {
        const NAME: &'static str = "AboutDialog";
        type Type = super::AboutDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AboutDialog {}
    impl WidgetImpl for AboutDialog {}
    impl AdwDialogImpl for AboutDialog {}
}

glib::wrapper! {
    pub struct AboutDialog(ObjectSubclass<imp::AboutDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for AboutDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl AboutDialog {
    pub fn new() -> Self {
        let dialog: Self = glib::Object::builder().build();
        dialog.setup_keybindings();
        dialog.setup_github_link();
        dialog
    }

    fn setup_keybindings(&self) {
        let imp = self.imp();
        let keybinding_manager = KeyBindingManager::new();
        let bindings = keybinding_manager.get_all_bindings();

        for binding in bindings {
            let row = adw::ActionRow::new();
            row.set_title(&binding.description);

            let key_combo =
                super::keybindings::format_key_combination(binding.key, binding.modifiers);
            let key_label = gtk::Label::new(Some(&key_combo));
            key_label.add_css_class("monospace");
            key_label.add_css_class("dim-label");

            row.add_suffix(&key_label);
            imp.keybindings_list.append(&row);
        }
    }

    fn setup_github_link(&self) {
        let imp = self.imp();
        imp.github_row.connect_activated(move |_| {
            let launcher = gtk::UriLauncher::new("https://github.com/kopecmaciej/fox-reader");
            launcher.launch(gtk::Window::NONE, gtk::gio::Cancellable::NONE, |result| {
                if let Err(e) = result {
                    eprintln!("Failed to open GitHub link: {}", e);
                }
            });
        });
    }
}

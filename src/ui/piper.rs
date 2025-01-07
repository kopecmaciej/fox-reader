use adw::subclass::prelude::*;
use gio::glib::Object;
use gtk::glib::{self, clone};
use gtk::prelude::*;
use reqwest;
use std::fs::{self, create_dir_all};
use std::process::Command;

use crate::core::runtime::runtime;

mod imp {
    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/piper_window.ui")]
    pub struct PiperWindow {
        #[template_child]
        pub path_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub download_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub confirm_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PiperWindow {
        const NAME: &'static str = "PiperWindow";
        type Type = super::PiperWindow;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PiperWindow {}
    impl WidgetImpl for PiperWindow {}
    impl WindowImpl for PiperWindow {}
    impl AdwWindowImpl for PiperWindow {}
}

glib::wrapper! {
    pub struct PiperWindow(ObjectSubclass<imp::PiperWindow>)
        @extends gtk::Widget, gtk::Window, adw::Window,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for PiperWindow {
    fn default() -> Self {
        PiperWindow::new()
    }
}

impl PiperWindow {
    pub fn new() -> Self {
        let window: Self = Object::new();

        window.setup_buttons();

        window
    }

    pub fn is_paper_available() -> bool {
        which::which("piper").is_ok() || which::which("piper-tts").is_ok()
    }

    fn setup_buttons(&self) {
        self.imp().download_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |button| {
                button.set_sensitive(false);
                let spinner = gtk::Spinner::new();
                spinner.start();
                button.set_child(Some(&spinner));

                glib::spawn_future_local(clone!(
                    #[weak]
                    this,
                    #[weak]
                    button,
                    async move {
                        match download_piper().await {
                            Ok(path) => {
                                this.imp().path_entry.set_text(&path);
                                button.set_label("Downloaded");
                            }
                            Err(e) => {
                                super::dialogs::show_error_dialog(
                                    &format!("Failed to download Piper: {}", e),
                                    &this,
                                );
                                button.set_label("Download");
                                button.set_sensitive(true);
                            }
                        }
                    }
                ));
            }
        ));

        self.imp().confirm_button.connect_clicked(clone!(
            #[weak(rename_to=this)]
            self,
            move |_| {
                let path = this.imp().path_entry.text();
                if Self::verify_piper_path(&path) {
                    this.close();
                } else {
                    super::dialogs::show_error_dialog("Invalid piper path", &this);
                }
            }
        ));
    }

    fn verify_piper_path(path: &str) -> bool {
        if path.is_empty() {
            return false;
        }

        Command::new(path)
            .arg("--help")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

async fn download_piper() -> Result<String, Box<dyn std::error::Error>> {
    use crate::config::piper_config;
    use flate2::read::GzDecoder;
    use tar::Archive;

    let download_path = piper_config::get_download_path();
    create_dir_all(&download_path)?;

    let url = piper_config::get_download_url();
    let compressed_data = runtime()
        .spawn(async move {
            let response = reqwest::get(&url).await?;
            response.bytes().await
        })
        .await??;

    let gz = GzDecoder::new(&compressed_data[..]);
    let mut archive = Archive::new(gz);

    archive.unpack(&download_path)?;

    let binary_path = piper_config::get_binary_path();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms)?;
    }

    Ok(binary_path)
}

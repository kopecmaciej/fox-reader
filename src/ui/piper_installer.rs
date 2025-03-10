use adw::prelude::AdwDialogExt;
use adw::subclass::prelude::*;
use gio::glib::Object;
use gtk::glib::{self, clone};
use gtk::prelude::*;
use reqwest;
use std::error::Error;
use std::fs::{self, create_dir_all};
use std::process::Command;

use crate::core::runtime::runtime;
use crate::core::speech_dispatcher::SpeechDispatcher;

mod imp {
    use super::*;
    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/org/fox-reader/ui/piper_window.ui")]
    pub struct PiperInstallerler {
        #[template_child]
        pub path_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub download_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub confirm_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PiperInstallerler {
        const NAME: &'static str = "PiperInstaller";
        type Type = super::PiperInstaller;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PiperInstallerler {}
    impl WidgetImpl for PiperInstallerler {}
    impl WindowImpl for PiperInstallerler {}
    impl AdwDialogImpl for PiperInstallerler {}
}

glib::wrapper! {
    pub struct PiperInstaller(ObjectSubclass<imp::PiperInstallerler>)
        @extends gtk::Widget, adw::Dialog,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Default for PiperInstaller {
    fn default() -> Self {
        PiperInstaller::new()
    }
}

impl PiperInstaller {
    pub fn new() -> Self {
        let window: Self = Object::new();

        window.setup_buttons();

        window
    }

    pub fn check_piper() -> Result<bool, Box<dyn Error>> {
        let is_piper_added = SpeechDispatcher::check_if_piper_already_added()?;
        Ok(is_piper_added)
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
                            Ok(piper_dir) => {
                                let piper_exec = format!("{}/piper", piper_dir);
                                if let Err(e) = SpeechDispatcher::update_piper_path(&piper_exec) {
                                    super::dialogs::show_error_dialog(
                                        &format!("Failed to add piper to configuration: {}", e),
                                        &this,
                                    );
                                }
                                button.set_label("Downloaded");
                                this.close();
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
                    if let Err(e) = SpeechDispatcher::update_piper_path(&path) {
                        super::dialogs::show_error_dialog(
                            &format!("Failed to add piper to configuration: {}", e),
                            &this,
                        );
                    }
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

async fn download_piper() -> Result<String, Box<dyn Error>> {
    use crate::paths::piper_config;
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

use adw::{prelude::*, AlertDialog};

pub fn show_error_dialog(err_msg: &str, widget: &impl IsA<gtk::Widget>) {
    let dialog = AlertDialog::builder()
        .heading("Error")
        .body(err_msg)
        .build();

    dialog.add_response("ok", "OK");
    dialog.set_default_response(Some("ok"));

    if let Some(root) = widget.root() {
        if let Some(window) = root.downcast_ref::<gtk::Window>() {
            dialog.present(Some(window));
        }
    }
}

pub fn file_dialog() -> gtk::FileDialog {
    let file_chooser = gtk::FileDialog::builder()
        .title("Open PDF")
        .accept_label("Open")
        .modal(true)
        .build();

    let filter = gtk::FileFilter::new();
    filter.add_mime_type("application/pdf");
    filter.set_name(Some("PDF files"));
    file_chooser.set_default_filter(Some(&filter));
    file_chooser
}

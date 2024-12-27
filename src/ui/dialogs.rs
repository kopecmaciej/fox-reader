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

use std::{cell::RefCell, rc::Rc};

use gio::glib::{self};

pub struct Debouncer {
    timeout_handle: Rc<RefCell<Option<glib::SourceId>>>,
    duration: std::time::Duration,
}

impl Debouncer {
    pub fn new(duration: std::time::Duration) -> Self {
        Self {
            timeout_handle: Rc::new(RefCell::new(None)),
            duration,
        }
    }

    pub fn debounce<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        if let Some(handle) = self.timeout_handle.borrow_mut().take() {
            if glib::MainContext::default()
                .find_source_by_id(&handle)
                .is_some()
            {
                handle.remove();
            }
        }

        let timeout_handle = self.timeout_handle.clone();
        *timeout_handle.borrow_mut() = Some(glib::timeout_add_local(self.duration, move || {
            callback();
            glib::ControlFlow::Break
        }));
    }
}

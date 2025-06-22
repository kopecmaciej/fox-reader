use gtk::{
    gdk::{self},
    glib::{self, clone},
    prelude::*,
};

use super::ai_chat::AiChat;

#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub key: gdk::Key,
    pub modifiers: gdk::ModifierType,
    pub description: String,
    pub action_name: String,
}

impl KeyBinding {
    pub fn new(
        key: gdk::Key,
        modifiers: gdk::ModifierType,
        description: &str,
        action_name: &str,
    ) -> Self {
        Self {
            key,
            modifiers,
            description: description.to_string(),
            action_name: action_name.to_string(),
        }
    }
}

#[derive(Debug)]
pub struct KeyBindingManager {
    bindings: Vec<KeyBinding>,
}

impl Default for KeyBindingManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyBindingManager {
    pub fn new() -> Self {
        let mut manager = Self {
            bindings: Vec::new(),
        };

        manager.add_ai_chat_bindings();

        manager
    }
    fn add_ai_chat_bindings(&mut self) {
        self.bindings.extend(vec![
            KeyBinding::new(
                gdk::Key::space,
                gdk::ModifierType::CONTROL_MASK,
                "Toggle AI recording (start/stop)",
                "ai-chat.toggle-recording",
            ),
            KeyBinding::new(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                "Stop AI audio playback",
                "ai-chat.stop-audio",
            ),
        ]);
    }

    pub fn setup_ai_chat_keybindings(&self, window: &impl IsA<gtk::Widget>, ai_chat: &AiChat) {
        let key_controller = gtk::EventControllerKey::new();

        key_controller.connect_key_pressed(clone!(
            #[weak]
            ai_chat,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_controller, key, _keycode, modifiers| {
                if key == gdk::Key::space && modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
                    ai_chat.toggle_recording();
                    return glib::Propagation::Stop;
                }

                if key == gdk::Key::Escape && modifiers.is_empty() {
                    ai_chat.force_stop_audio();
                    return glib::Propagation::Stop;
                }

                glib::Propagation::Proceed
            }
        ));

        window.add_controller(key_controller);
    }

    pub fn get_all_bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }

    pub fn get_bindings_for_action_prefix(&self, prefix: &str) -> Vec<&KeyBinding> {
        self.bindings
            .iter()
            .filter(|binding| binding.action_name.starts_with(prefix))
            .collect()
    }
}

pub fn format_key_combination(key: gdk::Key, modifiers: gdk::ModifierType) -> String {
    let mut parts = Vec::new();

    if modifiers.contains(gdk::ModifierType::CONTROL_MASK) {
        parts.push("Ctrl");
    }
    if modifiers.contains(gdk::ModifierType::ALT_MASK) {
        parts.push("Alt");
    }
    if modifiers.contains(gdk::ModifierType::SHIFT_MASK) {
        parts.push("Shift");
    }
    if modifiers.contains(gdk::ModifierType::SUPER_MASK) {
        parts.push("Super");
    }

    let key_name = key.name().unwrap_or_else(|| "Unknown".into());
    parts.push(&key_name);

    parts.join("+")
}

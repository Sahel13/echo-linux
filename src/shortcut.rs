use crate::settings;
use gtk::{gdk, glib::translate::IntoGlib};
use std::{sync::mpsc, thread};

mod x11;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortcutEvent {
    Pressed,
    Released,
    Escape,
    Active,
    Conflict,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Binding {
    pub key: String,
    pub keysym: u32,
    pub modifiers: Vec<settings::Modifier>,
}

pub enum UpdateResult {
    Applied,
    Conflict,
    Unavailable,
}

#[derive(Clone)]
pub struct ShortcutController {
    commands: mpsc::Sender<Command>,
}

pub enum Command {
    Update {
        binding: Binding,
        result: mpsc::Sender<UpdateResult>,
    },
    SetRecording(bool),
}

/// Platform seam for the future portal-based Wayland shortcut backend.
pub trait ShortcutBackend: Send + 'static {
    fn run(
        self,
        binding: Binding,
        commands: mpsc::Receiver<Command>,
        sender: mpsc::Sender<ShortcutEvent>,
    );
}

pub fn binding_from_settings(shortcut: &settings::Shortcut) -> Option<Binding> {
    let key = gdk::Key::from_name(&shortcut.key)?;
    Some(Binding {
        key: shortcut.key.clone(),
        keysym: key.into_glib(),
        modifiers: shortcut.modifiers.clone(),
    })
}

pub fn captured_binding(key: gdk::Key, state: gdk::ModifierType) -> Option<Binding> {
    if is_rejected_key(key) {
        return None;
    }
    Some(Binding {
        key: key.name()?.to_string(),
        keysym: key.into_glib(),
        modifiers: captured_modifiers(state),
    })
}

pub fn display_name(binding: &Binding) -> String {
    let mut parts = binding
        .modifiers
        .iter()
        .map(|modifier| match modifier {
            settings::Modifier::Control => "Ctrl",
            settings::Modifier::Alt => "Alt",
            settings::Modifier::Shift => "Shift",
            settings::Modifier::Super => "Super",
        })
        .collect::<Vec<_>>();
    parts.push(&binding.key);
    parts.join("+")
}

pub fn start_x11(binding: Binding, sender: mpsc::Sender<ShortcutEvent>) -> ShortcutController {
    let (command_sender, command_receiver) = mpsc::channel();
    thread::spawn(move || x11::X11ShortcutBackend.run(binding, command_receiver, sender));
    ShortcutController {
        commands: command_sender,
    }
}

impl ShortcutController {
    pub fn update(&self, binding: Binding) -> mpsc::Receiver<UpdateResult> {
        let (sender, receiver) = mpsc::channel();
        let _ = self.commands.send(Command::Update {
            binding,
            result: sender,
        });
        receiver
    }

    pub fn set_recording(&self, recording: bool) {
        let _ = self.commands.send(Command::SetRecording(recording));
    }
}

fn captured_modifiers(state: gdk::ModifierType) -> Vec<settings::Modifier> {
    [
        (gdk::ModifierType::CONTROL_MASK, settings::Modifier::Control),
        (gdk::ModifierType::ALT_MASK, settings::Modifier::Alt),
        (gdk::ModifierType::SHIFT_MASK, settings::Modifier::Shift),
        (gdk::ModifierType::SUPER_MASK, settings::Modifier::Super),
    ]
    .into_iter()
    .filter_map(|(mask, modifier)| state.contains(mask).then_some(modifier))
    .collect()
}

fn is_rejected_key(key: gdk::Key) -> bool {
    matches!(
        key,
        gdk::Key::Escape
            | gdk::Key::Shift_L
            | gdk::Key::Shift_R
            | gdk::Key::Control_L
            | gdk::Key::Control_R
            | gdk::Key::Alt_L
            | gdk::Key::Alt_R
            | gdk::Key::Super_L
            | gdk::Key::Super_R
            | gdk::Key::Meta_L
            | gdk::Key::Meta_R
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captures_a_named_key_and_its_modifiers() {
        let binding = captured_binding(
            gdk::Key::F9,
            gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
        )
        .expect("F9 is accepted");

        assert_eq!(binding.key, "F9");
        assert_eq!(
            binding.modifiers,
            vec![settings::Modifier::Control, settings::Modifier::Shift]
        );
        assert_eq!(display_name(&binding), "Ctrl+Shift+F9");
    }

    #[test]
    fn rejects_escape_and_bare_modifiers() {
        assert!(captured_binding(gdk::Key::Escape, gdk::ModifierType::empty()).is_none());
        assert!(captured_binding(gdk::Key::Control_L, gdk::ModifierType::empty()).is_none());
    }

    #[test]
    fn accepts_the_thinkpad_phone_marked_keysym_name() {
        let shortcut = settings::Shortcut {
            key: "XF86Favorites".into(),
            modifiers: vec![],
        };
        assert_eq!(
            binding_from_settings(&shortcut).map(|binding| binding.keysym),
            Some(0x1008_ff30)
        );
    }
}

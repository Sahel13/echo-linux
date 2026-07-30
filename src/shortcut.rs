use std::{sync::mpsc::Sender, thread};

mod x11;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortcutEvent {
    Pressed,
    Released,
    Active,
    Conflict,
    Unavailable,
}

/// Platform seam for the future portal-based Wayland shortcut backend.
pub trait ShortcutBackend: Send + 'static {
    fn run(self, sender: Sender<ShortcutEvent>);
}

pub fn start_default_x11(sender: Sender<ShortcutEvent>) {
    thread::spawn(move || x11::X11ShortcutBackend.run(sender));
}

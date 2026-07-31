use std::{sync::mpsc, thread};

mod x11;

/// The result of attempting to insert text into the previously focused X11 client.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PasteResult {
    Inserted,
    ClipboardOnly,
}

#[derive(Clone)]
pub struct PasteController {
    commands: mpsc::Sender<Command>,
}

pub(crate) enum Command {
    Insert {
        transcript: String,
        result: mpsc::Sender<PasteResult>,
    },
}

/// Platform seam for the future portal-based Wayland paste backend.
pub trait PasteBackend: Send + 'static {
    fn run(self, commands: mpsc::Receiver<Command>);
}

pub fn start_x11() -> PasteController {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || x11::X11PasteBackend.run(receiver));
    PasteController { commands: sender }
}

impl PasteController {
    pub fn insert(&self, transcript: String) -> mpsc::Receiver<PasteResult> {
        let (sender, receiver) = mpsc::channel();
        let _ = self.commands.send(Command::Insert {
            transcript,
            result: sender,
        });
        receiver
    }
}

#[cfg(test)]
fn failure_message(result: PasteResult) -> Option<&'static str> {
    match result {
        PasteResult::Inserted => None,
        PasteResult::ClipboardOnly => Some("Couldn't paste — transcript is on the clipboard."),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ClipboardSnapshot {
    Text(String),
    Unreadable,
}

fn should_restore(snapshot: &ClipboardSnapshot, clipboard_still_owned: bool) -> Option<&str> {
    if !clipboard_still_owned {
        return None;
    }
    match snapshot {
        ClipboardSnapshot::Text(text) => Some(text),
        ClipboardSnapshot::Unreadable => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_readable_text_only_when_nobody_changed_the_clipboard() {
        let snapshot = ClipboardSnapshot::Text("previous text".into());
        assert_eq!(should_restore(&snapshot, true), Some("previous text"));
        assert_eq!(should_restore(&snapshot, false), None);
    }

    #[test]
    fn leaves_the_transcript_when_the_previous_clipboard_was_not_text() {
        assert_eq!(should_restore(&ClipboardSnapshot::Unreadable, true), None);
    }

    #[test]
    fn injection_failure_explains_that_the_transcript_remains_available() {
        assert_eq!(failure_message(PasteResult::Inserted), None);
        assert_eq!(
            failure_message(PasteResult::ClipboardOnly),
            Some("Couldn't paste — transcript is on the clipboard.")
        );
    }
}

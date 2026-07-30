use super::{should_restore, ClipboardSnapshot, Command, PasteBackend, PasteResult};
use glib::variant::ToVariant;
use gtk::{gio, glib};
use std::{
    collections::HashMap,
    sync::mpsc::Receiver,
    thread,
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{
            Atom, AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, WindowClass,
            KEY_PRESS_EVENT, KEY_RELEASE_EVENT,
        },
        xtest::{self, ConnectionExt as _},
        Event,
    },
    rust_connection::RustConnection,
};

const CONTROL_L_KEYSYM: u32 = 0xffe3;
const SHIFT_L_KEYSYM: u32 = 0xffe1;
const V_KEYSYM: u32 = b'v' as u32;
const CURRENT_TIME: u32 = 0;
const CLIPBOARD_READ_TIMEOUT: Duration = Duration::from_millis(250);
const TARGET_CONSUME_DELAY: Duration = Duration::from_millis(150);
const TERMINAL_CONSUME_DELAY: Duration = Duration::from_millis(1_500);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const MAX_CLIPBOARD_BYTES: u32 = 256 * 1024;
const MAX_WINDOW_ANCESTORS: usize = 16;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PasteShortcut {
    ControlV,
    ControlShiftV,
    GhosttyAction,
}

pub struct X11PasteBackend;

impl PasteBackend for X11PasteBackend {
    fn run(self, commands: Receiver<Command>) {
        let Ok((connection, screen)) = x11rb::connect(None) else {
            report_unavailable(commands);
            return;
        };
        let root = connection.setup().roots[screen].root;
        let Ok(window) = create_selection_window(&connection, root) else {
            report_unavailable(commands);
            return;
        };
        let Some(atoms) = Atoms::intern(&connection) else {
            report_unavailable(commands);
            return;
        };
        if xtest::get_version(&connection, 2, 2)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .is_none()
        {
            report_unavailable(commands);
            return;
        }
        let Ok(mut clipboard) = clipboard_x11::Clipboard::connect() else {
            report_unavailable(commands);
            return;
        };

        while let Ok(Command::Insert { transcript, result }) = commands.recv() {
            let paste_result = insert(
                &connection,
                root,
                window,
                &atoms,
                &mut clipboard,
                &transcript,
            );
            let _ = result.send(paste_result);
        }
    }
}

fn report_unavailable(commands: Receiver<Command>) {
    while let Ok(Command::Insert { result, .. }) = commands.recv() {
        let _ = result.send(PasteResult::ClipboardOnly);
    }
}

fn create_selection_window(connection: &RustConnection, root: u32) -> Result<u32, ()> {
    let window = connection.generate_id().map_err(|_| ())?;
    connection
        .create_window(
            0,
            window,
            root,
            0,
            0,
            1,
            1,
            0,
            WindowClass::INPUT_OUTPUT,
            0,
            &CreateWindowAux::new().event_mask(EventMask::PROPERTY_CHANGE),
        )
        .map_err(|_| ())?
        .check()
        .map_err(|_| ())?;
    Ok(window)
}

struct Atoms {
    clipboard: Atom,
    utf8_string: Atom,
    text: Atom,
    text_plain_utf8_upper: Atom,
    text_plain_utf8: Atom,
    text_plain: Atom,
    selection_property: Atom,
}

impl Atoms {
    fn intern(connection: &RustConnection) -> Option<Self> {
        Some(Self {
            clipboard: intern_atom(connection, b"CLIPBOARD")?,
            utf8_string: intern_atom(connection, b"UTF8_STRING")?,
            text: intern_atom(connection, b"TEXT")?,
            text_plain_utf8_upper: intern_atom(connection, b"text/plain;charset=UTF-8")?,
            text_plain_utf8: intern_atom(connection, b"text/plain;charset=utf-8")?,
            text_plain: intern_atom(connection, b"text/plain")?,
            selection_property: intern_atom(connection, b"ECHO_SELECTION")?,
        })
    }
}

fn intern_atom(connection: &RustConnection, name: &[u8]) -> Option<Atom> {
    connection
        .intern_atom(false, name)
        .ok()?
        .reply()
        .ok()
        .map(|reply| reply.atom)
}

fn insert(
    connection: &RustConnection,
    root: u32,
    window: u32,
    atoms: &Atoms,
    clipboard: &mut clipboard_x11::Clipboard,
    transcript: &str,
) -> PasteResult {
    let snapshot = snapshot_text(connection, window, atoms);
    if clipboard.write(transcript.to_owned()).is_err() {
        return PasteResult::ClipboardOnly;
    }
    let Some(echo_owner) = selection_owner(connection, atoms.clipboard).filter(|owner| *owner != 0)
    else {
        return PasteResult::ClipboardOnly;
    };

    let shortcut = paste_shortcut_for_focused_window(connection, root);
    if inject_paste_shortcut(connection, root, shortcut).is_err() {
        return PasteResult::ClipboardOnly;
    }

    thread::sleep(if shortcut == PasteShortcut::ControlV {
        TARGET_CONSUME_DELAY
    } else {
        TERMINAL_CONSUME_DELAY
    });
    let still_owned = selection_owner(connection, atoms.clipboard) == Some(echo_owner);
    if let Some(previous) = should_restore(&snapshot, still_owned) {
        let _ = clipboard.write(previous.to_owned());
    }
    PasteResult::Inserted
}

fn snapshot_text(connection: &RustConnection, window: u32, atoms: &Atoms) -> ClipboardSnapshot {
    for target in [
        atoms.text_plain_utf8_upper,
        atoms.text_plain_utf8,
        atoms.utf8_string,
        atoms.text_plain,
        AtomEnum::STRING.into(),
        atoms.text,
    ] {
        if let Some(text) = request_selection_text(connection, window, atoms, target) {
            return ClipboardSnapshot::Text(text);
        }
    }
    ClipboardSnapshot::Unreadable
}

fn request_selection_text(
    connection: &RustConnection,
    window: u32,
    atoms: &Atoms,
    target: Atom,
) -> Option<String> {
    if selection_owner(connection, atoms.clipboard)? == 0 {
        return None;
    }
    connection
        .convert_selection(
            window,
            atoms.clipboard,
            target,
            atoms.selection_property,
            CURRENT_TIME,
        )
        .ok()?
        .check()
        .ok()?;
    connection.flush().ok()?;

    let deadline = Instant::now() + CLIPBOARD_READ_TIMEOUT;
    while Instant::now() < deadline {
        match connection.poll_for_event().ok()? {
            Some(Event::SelectionNotify(notify))
                if notify.requestor == window
                    && notify.selection == atoms.clipboard
                    && notify.target == target =>
            {
                if notify.property == 0 {
                    return None;
                }
                let reply = connection
                    .get_property(
                        true,
                        window,
                        notify.property,
                        AtomEnum::ANY,
                        0,
                        MAX_CLIPBOARD_BYTES / 4,
                    )
                    .ok()?
                    .reply()
                    .ok()?;
                if reply.bytes_after != 0 {
                    return None;
                }
                return String::from_utf8(reply.value8()?.collect()).ok();
            }
            Some(_) => {}
            None => thread::sleep(POLL_INTERVAL),
        }
    }
    None
}

fn selection_owner(connection: &RustConnection, clipboard: Atom) -> Option<u32> {
    connection
        .get_selection_owner(clipboard)
        .ok()?
        .reply()
        .ok()
        .map(|reply| reply.owner)
}

fn paste_shortcut_for_focused_window(connection: &RustConnection, root: u32) -> PasteShortcut {
    focused_wm_class(connection, root)
        .as_deref()
        .map_or(PasteShortcut::ControlV, paste_shortcut_for_wm_class)
}

fn focused_wm_class(connection: &RustConnection, root: u32) -> Option<Vec<u8>> {
    let focus = connection.get_input_focus().ok()?.reply().ok()?.focus;
    wm_class_from_window_or_ancestor(connection, focus, root).or_else(|| {
        let active_window = active_window(connection, root)?;
        wm_class_from_window_or_ancestor(connection, active_window, root)
    })
}

fn active_window(connection: &RustConnection, root: u32) -> Option<u32> {
    let atom = intern_atom(connection, b"_NET_ACTIVE_WINDOW")?;
    connection
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)
        .ok()?
        .reply()
        .ok()?
        .value32()?
        .next()
}

fn wm_class_from_window_or_ancestor(
    connection: &RustConnection,
    mut window: u32,
    root: u32,
) -> Option<Vec<u8>> {
    if window == 0 {
        return None;
    }
    for _ in 0..MAX_WINDOW_ANCESTORS {
        let property = connection
            .get_property(false, window, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 1024)
            .ok()?
            .reply()
            .ok()?;
        if !property.value.is_empty() {
            return Some(property.value);
        }
        if window == root {
            return None;
        }
        let parent = connection.query_tree(window).ok()?.reply().ok()?.parent;
        if parent == window {
            return None;
        }
        window = parent;
    }
    None
}

fn is_terminal_wm_class(wm_class: &[u8]) -> bool {
    wm_class
        .split(|byte| *byte == 0)
        .filter_map(|part| std::str::from_utf8(part).ok())
        .map(str::to_ascii_lowercase)
        .any(|class| {
            matches!(
                class.as_str(),
                "alacritty"
                    | "com.mitchellh.ghostty"
                    | "ghostty"
                    | "gnome-terminal"
                    | "gnome-terminal-server"
                    | "kitty"
                    | "konsole"
                    | "org.gnome.console"
                    | "org.gnome.ptyxis"
                    | "org.gnome.terminal"
                    | "org.kde.konsole"
                    | "ptyxis"
                    | "rio"
                    | "rxvt"
                    | "st"
                    | "terminator"
                    | "tilix"
                    | "urxvt"
                    | "uxterm"
                    | "wezterm"
                    | "xfce4-terminal"
                    | "xterm"
            )
        })
}

fn paste_shortcut_for_wm_class(wm_class: &[u8]) -> PasteShortcut {
    let is_ghostty = wm_class
        .split(|byte| *byte == 0)
        .filter_map(|part| std::str::from_utf8(part).ok())
        .any(|class| {
            class.eq_ignore_ascii_case("ghostty")
                || class.eq_ignore_ascii_case("com.mitchellh.ghostty")
        });
    if is_ghostty {
        PasteShortcut::GhosttyAction
    } else if is_terminal_wm_class(wm_class) {
        PasteShortcut::ControlShiftV
    } else {
        PasteShortcut::ControlV
    }
}

fn shortcut_keysyms(shortcut: PasteShortcut) -> &'static [u32] {
    match shortcut {
        PasteShortcut::ControlV => &[CONTROL_L_KEYSYM, V_KEYSYM],
        PasteShortcut::ControlShiftV => &[CONTROL_L_KEYSYM, SHIFT_L_KEYSYM, V_KEYSYM],
        PasteShortcut::GhosttyAction => &[],
    }
}

fn inject_paste_shortcut(
    connection: &RustConnection,
    root: u32,
    shortcut: PasteShortcut,
) -> Result<(), ()> {
    if shortcut == PasteShortcut::GhosttyAction {
        return activate_ghostty_paste(connection, root);
    }
    let keycodes = shortcut_keysyms(shortcut)
        .iter()
        .map(|keysym| keycode_for_keysym(connection, *keysym).ok_or(()))
        .collect::<Result<Vec<_>, _>>()?;
    for (event_type, keycode) in keycodes
        .iter()
        .map(|keycode| (KEY_PRESS_EVENT, *keycode))
        .chain(
            keycodes
                .iter()
                .rev()
                .map(|keycode| (KEY_RELEASE_EVENT, *keycode)),
        )
    {
        connection
            .xtest_fake_input(event_type, keycode, 0, root, 0, 0, 0)
            .map_err(|_| ())?
            .check()
            .map_err(|_| ())?;
    }
    connection.flush().map_err(|_| ())
}

fn activate_ghostty_paste(connection: &RustConnection, root: u32) -> Result<(), ()> {
    let window = active_window(connection, root).ok_or(())?;
    let application_id =
        window_text_property(connection, window, b"_GTK_APPLICATION_ID").ok_or(())?;
    if application_id != "com.mitchellh.ghostty" {
        return Err(());
    }
    let bus_name = window_text_property(connection, window, b"_GTK_UNIQUE_BUS_NAME").ok_or(())?;
    let object_path =
        window_text_property(connection, window, b"_GTK_WINDOW_OBJECT_PATH").ok_or(())?;
    if !bus_name.starts_with(':') || !object_path.starts_with("/com/mitchellh/ghostty/window/") {
        return Err(());
    }

    let parameters = (
        "paste",
        Vec::<glib::Variant>::new(),
        HashMap::<String, glib::Variant>::new(),
    )
        .to_variant();
    let session =
        gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>).map_err(|_| ())?;
    session
        .call_sync(
            Some(&bus_name),
            &object_path,
            "org.gtk.Actions",
            "Activate",
            Some(&parameters),
            None,
            gio::DBusCallFlags::NONE,
            1_000,
            None::<&gio::Cancellable>,
        )
        .map(|_| ())
        .map_err(|_| ())
}

fn window_text_property(connection: &RustConnection, window: u32, name: &[u8]) -> Option<String> {
    let property = intern_atom(connection, name)?;
    let reply = connection
        .get_property(
            false,
            window,
            property,
            AtomEnum::ANY,
            0,
            MAX_CLIPBOARD_BYTES / 4,
        )
        .ok()?
        .reply()
        .ok()?;
    if reply.bytes_after != 0 {
        return None;
    }
    String::from_utf8(reply.value).ok()
}

fn keycode_for_keysym(connection: &RustConnection, keysym: u32) -> Option<u8> {
    let setup = connection.setup();
    let first = setup.min_keycode;
    let count = setup.max_keycode.checked_sub(first)?.checked_add(1)?;
    let mapping = connection
        .get_keyboard_mapping(first, count)
        .ok()?
        .reply()
        .ok()?;
    if mapping.keysyms_per_keycode == 0 {
        return None;
    }
    mapping
        .keysyms
        .chunks_exact(usize::from(mapping.keysyms_per_keycode))
        .position(|keysyms| keysyms.contains(&keysym))
        .and_then(|index| first.checked_add(u8::try_from(index).ok()?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_window_classes_use_the_terminal_paste_shortcut() {
        assert_eq!(
            paste_shortcut_for_wm_class(b"ghostty\0com.mitchellh.ghostty\0"),
            PasteShortcut::GhosttyAction
        );
        for wm_class in [
            b"gnome-terminal-server\0Gnome-terminal\0".as_slice(),
            b"kitty\0kitty\0".as_slice(),
            b"Alacritty\0Alacritty\0".as_slice(),
            b"konsole\0org.kde.konsole\0".as_slice(),
            b"xterm\0XTerm\0".as_slice(),
        ] {
            assert!(is_terminal_wm_class(wm_class));
            assert_eq!(
                paste_shortcut_for_wm_class(wm_class),
                PasteShortcut::ControlShiftV
            );
        }
        assert!(shortcut_keysyms(PasteShortcut::GhosttyAction).is_empty());
    }

    #[test]
    fn ordinary_windows_keep_the_standard_paste_shortcut() {
        assert!(!is_terminal_wm_class(b"firefox\0firefox\0"));
        assert!(!is_terminal_wm_class(b"libreoffice\0libreoffice-writer\0"));
        assert_eq!(
            paste_shortcut_for_wm_class(b"firefox\0firefox\0"),
            PasteShortcut::ControlV
        );
        assert_eq!(
            shortcut_keysyms(paste_shortcut_for_wm_class(b"firefox\0firefox\0")),
            &[CONTROL_L_KEYSYM, V_KEYSYM]
        );
    }
}

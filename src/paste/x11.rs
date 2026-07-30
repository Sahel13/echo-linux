use super::{should_restore, ClipboardSnapshot, Command, PasteBackend, PasteResult};
use std::{
    sync::mpsc::{Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    protocol::{
        xproto::{
            Atom, AtomEnum, ConnectionExt as _, CreateWindowAux, EventMask, PropMode,
            SelectionNotifyEvent, WindowClass, KEY_PRESS_EVENT, KEY_RELEASE_EVENT,
            SELECTION_NOTIFY_EVENT,
        },
        xtest::{self, ConnectionExt as _},
        Event,
    },
    rust_connection::RustConnection,
    wrapper::ConnectionExt as _,
};

const CONTROL_L_KEYSYM: u32 = 0xffe3;
const V_KEYSYM: u32 = b'v' as u32;
const CURRENT_TIME: u32 = 0;
const CLIPBOARD_READ_TIMEOUT: Duration = Duration::from_millis(250);
const TARGET_CONSUME_DELAY: Duration = Duration::from_millis(150);
const POLL_INTERVAL: Duration = Duration::from_millis(5);
const MAX_CLIPBOARD_BYTES: u32 = 256 * 1024;

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

        let mut owned_text = None;
        loop {
            match commands.recv_timeout(POLL_INTERVAL) {
                Ok(Command::Insert { transcript, result }) => {
                    let paste_result = insert(
                        &connection,
                        root,
                        window,
                        &atoms,
                        &mut owned_text,
                        &transcript,
                    );
                    let _ = result.send(paste_result);
                }
                Err(RecvTimeoutError::Timeout) => {
                    serve_pending_selection_requests(&connection, &atoms, &owned_text);
                }
                Err(RecvTimeoutError::Disconnected) => return,
            }
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
    targets: Atom,
    selection_property: Atom,
}

impl Atoms {
    fn intern(connection: &RustConnection) -> Option<Self> {
        Some(Self {
            clipboard: intern_atom(connection, b"CLIPBOARD")?,
            utf8_string: intern_atom(connection, b"UTF8_STRING")?,
            text: intern_atom(connection, b"TEXT")?,
            targets: intern_atom(connection, b"TARGETS")?,
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
    owned_text: &mut Option<String>,
    transcript: &str,
) -> PasteResult {
    let snapshot = snapshot_text(connection, window, atoms, owned_text);
    if set_clipboard_owner(connection, window, atoms.clipboard).is_err() {
        return PasteResult::ClipboardOnly;
    }
    *owned_text = Some(transcript.to_owned());

    if inject_ctrl_v(connection, root).is_err() {
        return PasteResult::ClipboardOnly;
    }

    serve_selection_requests(connection, window, atoms, owned_text, TARGET_CONSUME_DELAY);
    let still_owned = selection_owner(connection, atoms.clipboard) == Some(window);
    if let Some(previous) = should_restore(&snapshot, still_owned) {
        *owned_text = Some(previous.to_owned());
    }
    PasteResult::Inserted
}

fn snapshot_text(
    connection: &RustConnection,
    window: u32,
    atoms: &Atoms,
    owned_text: &Option<String>,
) -> ClipboardSnapshot {
    for target in [atoms.utf8_string, AtomEnum::STRING.into(), atoms.text] {
        if let Some(text) = request_selection_text(connection, window, atoms, owned_text, target) {
            return ClipboardSnapshot::Text(text);
        }
    }
    ClipboardSnapshot::Unreadable
}

fn request_selection_text(
    connection: &RustConnection,
    window: u32,
    atoms: &Atoms,
    owned_text: &Option<String>,
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
            Some(Event::SelectionRequest(request)) => {
                respond_to_selection_request(connection, request, atoms, owned_text).ok()?;
            }
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

fn set_clipboard_owner(
    connection: &RustConnection,
    window: u32,
    clipboard: Atom,
) -> Result<(), ()> {
    connection
        .set_selection_owner(window, clipboard, CURRENT_TIME)
        .map_err(|_| ())?
        .check()
        .map_err(|_| ())?;
    connection.flush().map_err(|_| ())?;
    (selection_owner(connection, clipboard) == Some(window))
        .then_some(())
        .ok_or(())
}

fn selection_owner(connection: &RustConnection, clipboard: Atom) -> Option<u32> {
    connection
        .get_selection_owner(clipboard)
        .ok()?
        .reply()
        .ok()
        .map(|reply| reply.owner)
}

fn inject_ctrl_v(connection: &RustConnection, root: u32) -> Result<(), ()> {
    let control = keycode_for_keysym(connection, CONTROL_L_KEYSYM).ok_or(())?;
    let v = keycode_for_keysym(connection, V_KEYSYM).ok_or(())?;
    for (event_type, keycode) in [
        (KEY_PRESS_EVENT, control),
        (KEY_PRESS_EVENT, v),
        (KEY_RELEASE_EVENT, v),
        (KEY_RELEASE_EVENT, control),
    ] {
        connection
            .xtest_fake_input(event_type, keycode, 0, root, 0, 0, 0)
            .map_err(|_| ())?
            .check()
            .map_err(|_| ())?;
    }
    connection.flush().map_err(|_| ())
}

fn serve_selection_requests(
    connection: &RustConnection,
    window: u32,
    atoms: &Atoms,
    owned_text: &Option<String>,
    duration: Duration,
) {
    let deadline = Instant::now() + duration;
    while Instant::now() < deadline {
        match connection.poll_for_event() {
            Ok(Some(Event::SelectionRequest(request))) => {
                let _ = respond_to_selection_request(connection, request, atoms, owned_text);
            }
            Ok(Some(_)) | Err(_) => {}
            Ok(None) => thread::sleep(POLL_INTERVAL),
        }
    }
    let _ = window;
}

fn serve_pending_selection_requests(
    connection: &RustConnection,
    atoms: &Atoms,
    owned_text: &Option<String>,
) {
    loop {
        match connection.poll_for_event() {
            Ok(Some(Event::SelectionRequest(request))) => {
                let _ = respond_to_selection_request(connection, request, atoms, owned_text);
            }
            Ok(Some(_)) | Err(_) => {}
            Ok(None) => return,
        }
    }
}

fn respond_to_selection_request(
    connection: &RustConnection,
    request: x11rb::protocol::xproto::SelectionRequestEvent,
    atoms: &Atoms,
    owned_text: &Option<String>,
) -> Result<(), ()> {
    let property = if request.property != 0 {
        request.property
    } else {
        request.target
    };
    let response_property = match owned_text {
        Some(text) if request.selection == atoms.clipboard && request.target == atoms.targets => {
            connection
                .change_property32(
                    PropMode::REPLACE,
                    request.requestor,
                    property,
                    AtomEnum::ATOM,
                    &[
                        atoms.targets,
                        atoms.utf8_string,
                        atoms.text,
                        AtomEnum::STRING.into(),
                    ],
                )
                .map_err(|_| ())?;
            property
        }
        Some(text)
            if request.selection == atoms.clipboard
                && [atoms.utf8_string, atoms.text, AtomEnum::STRING.into()]
                    .contains(&request.target) =>
        {
            connection
                .change_property8(
                    PropMode::REPLACE,
                    request.requestor,
                    property,
                    atoms.utf8_string,
                    text.as_bytes(),
                )
                .map_err(|_| ())?;
            property
        }
        _ => AtomEnum::NONE.into(),
    };
    let notify = SelectionNotifyEvent {
        response_type: SELECTION_NOTIFY_EVENT,
        sequence: 0,
        time: request.time,
        requestor: request.requestor,
        selection: request.selection,
        target: request.target,
        property: response_property,
    };
    connection
        .send_event(false, request.requestor, EventMask::NO_EVENT, notify)
        .map_err(|_| ())?;
    connection.flush().map_err(|_| ())
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

use super::{Binding, Command, ShortcutBackend, ShortcutEvent, UpdateResult};
use crate::settings;
use std::{
    sync::mpsc::{Receiver, Sender},
    thread,
    time::{Duration, Instant},
};
use x11rb::{
    connection::Connection,
    errors::ReplyError,
    protocol::{
        xkb::{self, BoolCtrl, PerClientFlag, ID},
        xproto::{ConnectionExt as _, GrabMode, Mapping, ModMask},
        ErrorKind, Event,
    },
    rust_connection::RustConnection,
};

const NUM_LOCK_KEYSYM: u32 = 0xff7f;
const FALLBACK_RELEASE_DELAY: Duration = Duration::from_millis(30);
const POLL_INTERVAL: Duration = Duration::from_millis(5);

pub struct X11ShortcutBackend;

impl ShortcutBackend for X11ShortcutBackend {
    fn run(
        self,
        initial_binding: Binding,
        commands: Receiver<Command>,
        sender: Sender<ShortcutEvent>,
    ) {
        let Ok((connection, screen)) = x11rb::connect(None) else {
            let _ = sender.send(ShortcutEvent::Unavailable);
            return;
        };

        let root = connection.setup().roots[screen].root;
        let detectable_auto_repeat = enable_detectable_auto_repeat(&connection);
        let mut binding = match GrabbedBinding::resolve(&connection, root, &initial_binding) {
            Ok(binding) => binding,
            Err(GrabError::Conflict) => {
                let _ = sender.send(ShortcutEvent::Conflict);
                return;
            }
            Err(GrabError::Unavailable) => {
                let _ = sender.send(ShortcutEvent::Unavailable);
                return;
            }
        };
        let _ = sender.send(ShortcutEvent::Active);

        let mut repeat_filter = RepeatFilter::new(detectable_auto_repeat);
        loop {
            while let Ok(Command::Update {
                binding: requested,
                result,
            }) = commands.try_recv()
            {
                if requested == binding.requested {
                    let _ = result.send(UpdateResult::Applied);
                    continue;
                }
                match GrabbedBinding::resolve(&connection, root, &requested) {
                    Ok(new_binding) => {
                        binding.ungrab(&connection);
                        binding = new_binding;
                        let _ = result.send(UpdateResult::Applied);
                        let _ = sender.send(ShortcutEvent::Active);
                    }
                    Err(GrabError::Conflict) => {
                        let _ = result.send(UpdateResult::Conflict);
                    }
                    Err(GrabError::Unavailable) => {
                        let _ = result.send(UpdateResult::Unavailable);
                    }
                }
            }
            match connection.poll_for_event() {
                Ok(Some(event)) => {
                    let events = match event {
                        Event::KeyPress(event) if event.detail == binding.keycode => {
                            repeat_filter.key_press(event.time)
                        }
                        Event::KeyRelease(event) if event.detail == binding.keycode => {
                            repeat_filter.key_release(event.time)
                        }
                        Event::MappingNotify(event)
                            if event.request == Mapping::KEYBOARD
                                || event.request == Mapping::MODIFIER =>
                        {
                            send_events(&sender, repeat_filter.flush());
                            binding.ungrab(&connection);
                            match GrabbedBinding::resolve(&connection, root, &binding.requested) {
                                Ok(new_binding) => {
                                    binding = new_binding;
                                    let _ = sender.send(ShortcutEvent::Active);
                                }
                                Err(GrabError::Conflict) => {
                                    let _ = sender.send(ShortcutEvent::Conflict);
                                    return;
                                }
                                Err(GrabError::Unavailable) => {
                                    let _ = sender.send(ShortcutEvent::Unavailable);
                                    return;
                                }
                            }
                            Vec::new()
                        }
                        Event::Error(_) => {
                            let _ = sender.send(ShortcutEvent::Unavailable);
                            return;
                        }
                        _ => Vec::new(),
                    };
                    send_events(&sender, events);
                }
                Ok(None) => {
                    send_events(&sender, repeat_filter.release_if_due());
                    thread::sleep(POLL_INTERVAL);
                }
                Err(_) => {
                    let _ = sender.send(ShortcutEvent::Unavailable);
                    return;
                }
            }
        }
    }
}

fn send_events(sender: &Sender<ShortcutEvent>, events: Vec<ShortcutEvent>) {
    for event in events {
        if sender.send(event).is_err() {
            return;
        }
    }
}

fn enable_detectable_auto_repeat(connection: &RustConnection) -> bool {
    let Some(version) = xkb::use_extension(connection, 1, 0)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
    else {
        return false;
    };
    if !version.supported {
        return false;
    }

    let flag = PerClientFlag::DETECTABLE_AUTO_REPEAT;
    xkb::per_client_flags(
        connection,
        ID::USE_CORE_KBD.into(),
        flag,
        flag,
        BoolCtrl::default(),
        BoolCtrl::default(),
        BoolCtrl::default(),
    )
    .ok()
    .and_then(|cookie| cookie.reply().ok())
    .map(|reply| u32::from(reply.value) & u32::from(flag) != 0)
    .unwrap_or(false)
}

struct GrabbedBinding {
    root: u32,
    keycode: u8,
    modifiers: Vec<ModMask>,
    requested: Binding,
}

impl GrabbedBinding {
    fn resolve(
        connection: &RustConnection,
        root: u32,
        requested: &Binding,
    ) -> Result<Self, GrabError> {
        let keycode =
            keycode_for_keysym(connection, requested.keysym).ok_or(GrabError::Unavailable)?;
        let num_lock = num_lock_mask(connection).ok_or(GrabError::Unavailable)?;
        let modifiers = lock_modifier_variants(modifier_mask(&requested.modifiers), num_lock);
        let binding = Self {
            root,
            keycode,
            modifiers,
            requested: requested.clone(),
        };
        binding.grab(connection)?;
        Ok(binding)
    }

    fn grab(&self, connection: &RustConnection) -> Result<(), GrabError> {
        for modifier in &self.modifiers {
            let result = connection
                .grab_key(
                    false,
                    self.root,
                    *modifier,
                    self.keycode,
                    GrabMode::ASYNC,
                    GrabMode::ASYNC,
                )
                .map_err(|_| GrabError::Unavailable)
                .and_then(|cookie| match cookie.check() {
                    Ok(()) => Ok(()),
                    Err(ReplyError::X11Error(error)) if error.error_kind == ErrorKind::Access => {
                        Err(GrabError::Conflict)
                    }
                    Err(_) => Err(GrabError::Unavailable),
                });
            if let Err(error) = result {
                self.ungrab(connection);
                return Err(error);
            }
        }
        connection.flush().map_err(|_| GrabError::Unavailable)
    }

    fn ungrab(&self, connection: &RustConnection) {
        for modifier in &self.modifiers {
            if let Ok(cookie) = connection.ungrab_key(self.keycode, self.root, *modifier) {
                let _ = cookie.check();
            }
        }
        let _ = connection.flush();
    }
}

#[derive(Clone, Copy)]
enum GrabError {
    Conflict,
    Unavailable,
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
    keycode_from_mapping(first, mapping.keysyms_per_keycode, &mapping.keysyms, keysym)
}

fn keycode_from_mapping(
    first_keycode: u8,
    keysyms_per_keycode: u8,
    keysyms: &[u32],
    wanted_keysym: u32,
) -> Option<u8> {
    if keysyms_per_keycode == 0 {
        return None;
    }
    keysyms
        .chunks_exact(usize::from(keysyms_per_keycode))
        .position(|keysyms| keysyms.contains(&wanted_keysym))
        .and_then(|index| first_keycode.checked_add(u8::try_from(index).ok()?))
}

fn num_lock_mask(connection: &RustConnection) -> Option<ModMask> {
    let mapping = connection.get_modifier_mapping().ok()?.reply().ok()?;
    let keycodes_per_modifier = usize::from(mapping.keycodes_per_modifier());
    if keycodes_per_modifier == 0 {
        return Some(ModMask::default());
    }
    for (index, keycodes) in mapping
        .keycodes
        .chunks_exact(keycodes_per_modifier)
        .enumerate()
    {
        if keycodes
            .iter()
            .any(|keycode| keycode_for_keycode(connection, *keycode, NUM_LOCK_KEYSYM))
        {
            return Some(ModMask::from(1_u16 << index));
        }
    }
    Some(ModMask::default())
}

fn keycode_for_keycode(connection: &RustConnection, keycode: u8, wanted_keysym: u32) -> bool {
    connection
        .get_keyboard_mapping(keycode, 1)
        .ok()
        .and_then(|cookie| cookie.reply().ok())
        .is_some_and(|mapping| mapping.keysyms.contains(&wanted_keysym))
}

fn modifier_mask(modifiers: &[settings::Modifier]) -> ModMask {
    modifiers.iter().fold(ModMask::default(), |mask, modifier| {
        mask | match modifier {
            settings::Modifier::Control => ModMask::CONTROL,
            settings::Modifier::Alt => ModMask::M1,
            settings::Modifier::Shift => ModMask::SHIFT,
            settings::Modifier::Super => ModMask::M4,
        }
    })
}

fn lock_modifier_variants(base: ModMask, num_lock: ModMask) -> Vec<ModMask> {
    let caps_lock = ModMask::LOCK;
    let mut variants = Vec::new();
    for modifier in [
        base,
        base | caps_lock,
        base | num_lock,
        base | caps_lock | num_lock,
    ] {
        if !variants.contains(&modifier) {
            variants.push(modifier);
        }
    }
    variants
}

struct RepeatFilter {
    detectable_auto_repeat: bool,
    held: bool,
    pending_release: Option<PendingRelease>,
}

struct PendingRelease {
    time: u32,
    received_at: Instant,
}

impl RepeatFilter {
    fn new(detectable_auto_repeat: bool) -> Self {
        Self {
            detectable_auto_repeat,
            held: false,
            pending_release: None,
        }
    }

    fn key_press(&mut self, time: u32) -> Vec<ShortcutEvent> {
        if let Some(release) = self.pending_release.take() {
            if release.time == time {
                return Vec::new();
            }
            self.held = false;
            let mut events = vec![ShortcutEvent::Released];
            events.extend(self.emit_pressed());
            return events;
        }
        self.emit_pressed()
    }

    fn key_release(&mut self, time: u32) -> Vec<ShortcutEvent> {
        if !self.held {
            return Vec::new();
        }
        if self.detectable_auto_repeat {
            self.held = false;
            vec![ShortcutEvent::Released]
        } else {
            self.pending_release = Some(PendingRelease {
                time,
                received_at: Instant::now(),
            });
            Vec::new()
        }
    }

    fn release_if_due(&mut self) -> Vec<ShortcutEvent> {
        if self
            .pending_release
            .as_ref()
            .is_some_and(|release| release.received_at.elapsed() >= FALLBACK_RELEASE_DELAY)
        {
            self.flush()
        } else {
            Vec::new()
        }
    }

    fn flush(&mut self) -> Vec<ShortcutEvent> {
        if self.pending_release.take().is_some() && self.held {
            self.held = false;
            vec![ShortcutEvent::Released]
        } else {
            Vec::new()
        }
    }

    fn emit_pressed(&mut self) -> Vec<ShortcutEvent> {
        if self.held {
            Vec::new()
        } else {
            self.held = true;
            vec![ShortcutEvent::Pressed]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_the_first_matching_keysym_from_a_keyboard_map() {
        assert_eq!(
            keycode_from_mapping(8, 2, &[0, 0, 0xffc7, 0], 0xffc7),
            Some(9)
        );
    }

    #[test]
    fn grabs_every_caps_and_num_lock_variant() {
        let num_lock = ModMask::M2;
        assert_eq!(
            lock_modifier_variants(ModMask::default(), num_lock),
            vec![
                ModMask::default(),
                ModMask::LOCK,
                ModMask::M2,
                ModMask::LOCK | ModMask::M2,
            ]
        );
    }

    #[test]
    fn combines_configured_modifiers_with_lock_variants() {
        let base = modifier_mask(&[settings::Modifier::Control, settings::Modifier::Shift]);
        assert_eq!(base, ModMask::CONTROL | ModMask::SHIFT);
        assert_eq!(
            lock_modifier_variants(base, ModMask::M2),
            vec![
                ModMask::CONTROL | ModMask::SHIFT,
                ModMask::CONTROL | ModMask::SHIFT | ModMask::LOCK,
                ModMask::CONTROL | ModMask::SHIFT | ModMask::M2,
                ModMask::CONTROL | ModMask::SHIFT | ModMask::LOCK | ModMask::M2,
            ]
        );
    }

    #[test]
    fn detectable_auto_repeat_emits_one_press_and_one_release() {
        let mut filter = RepeatFilter::new(true);
        assert_eq!(filter.key_press(10), vec![ShortcutEvent::Pressed]);
        assert!(filter.key_press(20).is_empty());
        assert_eq!(filter.key_release(30), vec![ShortcutEvent::Released]);
    }

    #[test]
    fn fallback_suppresses_a_synthetic_repeat_release_and_press_pair() {
        let mut filter = RepeatFilter::new(false);
        assert_eq!(filter.key_press(10), vec![ShortcutEvent::Pressed]);
        assert!(filter.key_release(20).is_empty());
        assert!(filter.key_press(20).is_empty());
        assert!(filter.key_release(30).is_empty());
        assert_eq!(filter.flush(), vec![ShortcutEvent::Released]);
    }
}

use adw::prelude::*;
use gtk::{
    gdk,
    glib::{self, object::ObjectType},
};
use std::{cell::Cell, ffi::c_void, os::raw::c_ulong, rc::Rc, time::Duration};
use x11rb::{
    connection::Connection,
    protocol::xproto::{
        AtomEnum, ChangeWindowAttributesAux, ConfigureWindowAux, ConnectionExt as _, PropMode,
        StackMode,
    },
    wrapper::ConnectionExt as _,
};

const WIDTH: i32 = 480;
const HEIGHT: i32 = 48;
const BOTTOM_MARGIN: i32 = 120;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Mode {
    #[default]
    Hidden,
    Recording,
    Transcribing,
    Error,
}

#[derive(Clone)]
pub struct Overlay {
    window: gtk::Window,
    recording_logo: gtk::Image,
    transcribing_logo: gtk::Image,
    message: gtk::Label,
    mode: Rc<Cell<Mode>>,
}

impl Overlay {
    pub fn new(application: &adw::Application) -> Self {
        install_css();

        let recording_logo = logo_image(include_bytes!("../assets/echo-app-mark.svg"));
        let transcribing_logo = logo_image(include_bytes!("../assets/echo-app-mark-white.svg"));

        let message = gtk::Label::new(None);
        message.add_css_class("echo-overlay-message");
        message.set_ellipsize(gtk::pango::EllipsizeMode::End);
        message.set_max_width_chars(42);
        message.set_visible(false);

        let content = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        content.add_css_class("echo-overlay");
        content.set_hexpand(true);
        content.set_vexpand(true);
        content.set_halign(gtk::Align::Center);
        content.set_valign(gtk::Align::Center);
        content.append(&recording_logo);
        content.append(&transcribing_logo);
        content.append(&message);

        let window = gtk::Window::builder()
            .application(application)
            .title("Echo dictation status")
            .decorated(false)
            .resizable(false)
            .focusable(false)
            .default_width(WIDTH)
            .default_height(HEIGHT)
            .child(&content)
            .build();
        window.set_can_focus(false);
        window.add_css_class("echo-overlay-window");
        window.connect_realize(|window| {
            if let Some(surface) = window.surface() {
                surface.set_input_region(&gtk::cairo::Region::create());
                configure_x11_window(&surface);
            }
        });
        // GTK creates a native surface only when the window is first mapped.
        // Prime it invisibly so the X11 hints and input region are ready for
        // every later state transition.
        window.set_opacity(0.0);
        window.set_visible(true);
        window.set_visible(false);
        window.set_opacity(1.0);

        let mode = Rc::new(Cell::new(Mode::Hidden));
        let pulse_phase = Rc::new(Cell::new(0_u8));
        glib::timeout_add_local(Duration::from_millis(175), {
            let recording_logo = recording_logo.clone();
            let transcribing_logo = transcribing_logo.clone();
            let mode = mode.clone();
            move || {
                let phase = pulse_phase.get();
                let opacity = pulse_opacity(mode.get(), phase);
                recording_logo.set_opacity(opacity);
                transcribing_logo.set_opacity(opacity);
                pulse_phase.set((phase + 1) % 4);
                glib::ControlFlow::Continue
            }
        });

        Self {
            window,
            recording_logo,
            transcribing_logo,
            message,
            mode,
        }
    }

    pub fn show_recording(&self) {
        self.set_mode(Mode::Recording, "");
    }

    pub fn show_transcribing(&self) {
        self.set_mode(Mode::Transcribing, "");
    }

    pub fn show_error(&self, message: &str) {
        self.set_mode(Mode::Error, message);
    }

    pub fn hide(&self) {
        self.mode.set(Mode::Hidden);
        self.window.set_visible(false);
    }

    fn set_mode(&self, mode: Mode, message: &str) {
        self.mode.set(mode);
        self.message.set_text(message);
        self.recording_logo.set_visible(mode == Mode::Recording);
        self.transcribing_logo
            .set_visible(mode == Mode::Transcribing);
        self.message.set_visible(mode == Mode::Error);
        // set_visible maps the already-realized non-focusable window without
        // issuing the activation request that present() would send.
        self.window.set_visible(true);
        if let Some(surface) = self.window.surface() {
            // GTK refreshes some WM properties while mapping. Reapply the
            // overlay contract after the map and then place it.
            surface.set_input_region(&gtk::cairo::Region::create());
            configure_x11_window(&surface);
        }
        self.position_on_active_monitor();
    }

    fn position_on_active_monitor(&self) {
        let Some(display) = gdk::Display::default() else {
            return;
        };
        let focused_monitor = x11_focus_center().and_then(|(focus_x, focus_y)| {
            let monitors = display.monitors();
            (0..monitors.n_items()).find_map(|index| {
                let monitor = monitors.item(index)?.downcast::<gdk::Monitor>().ok()?;
                rectangle_contains(&monitor.geometry(), focus_x, focus_y).then_some(monitor)
            })
        });
        let pointer_surface = display
            .default_seat()
            .and_then(|seat| seat.pointer())
            .and_then(|pointer| pointer.surface_at_position().0);
        let monitor = focused_monitor
            .or_else(|| pointer_surface.and_then(|surface| display.monitor_at_surface(&surface)))
            .or_else(|| {
                self.window
                    .surface()
                    .and_then(|surface| display.monitor_at_surface(&surface))
            })
            .or_else(|| display.monitors().item(0)?.downcast::<gdk::Monitor>().ok());
        let Some(monitor) = monitor else {
            return;
        };
        let geometry = monitor.geometry();
        let x = geometry.x() + (geometry.width() - WIDTH) / 2;
        let y = geometry.y() + geometry.height() - HEIGHT - BOTTOM_MARGIN;
        let Some(surface) = self.window.surface() else {
            return;
        };
        let xid = x11_surface_xid(&surface);
        if xid == 0 {
            return;
        }
        if let Ok((connection, _)) = x11rb::connect(None) {
            let _ = connection.configure_window(
                xid as u32,
                &ConfigureWindowAux::new()
                    .x(x)
                    .y(y)
                    .width(WIDTH as u32)
                    .height(HEIGHT as u32)
                    .stack_mode(StackMode::ABOVE),
            );
            let _ = connection.flush();
        }
    }
}

fn x11_focus_center() -> Option<(i32, i32)> {
    let (connection, screen_index) = x11rb::connect(None).ok()?;
    let root = connection.setup().roots.get(screen_index)?.root;
    let focus = connection.get_input_focus().ok()?.reply().ok()?.focus;
    let geometry = connection.get_geometry(focus).ok()?.reply().ok()?;
    let translated = connection
        .translate_coordinates(focus, root, 0, 0)
        .ok()?
        .reply()
        .ok()?;
    Some((
        i32::from(translated.dst_x) + i32::from(geometry.width) / 2,
        i32::from(translated.dst_y) + i32::from(geometry.height) / 2,
    ))
}

fn rectangle_contains(rectangle: &gdk::Rectangle, x: i32, y: i32) -> bool {
    x >= rectangle.x()
        && x < rectangle.x() + rectangle.width()
        && y >= rectangle.y()
        && y < rectangle.y() + rectangle.height()
}

fn logo_image(bytes: &'static [u8]) -> gtk::Image {
    let image = gtk::Image::new();
    if let Ok(texture) = gdk::Texture::from_bytes(&glib::Bytes::from_static(bytes)) {
        image.set_paintable(Some(&texture));
    }
    image.set_pixel_size(18);
    image.set_visible(false);
    image
}

fn pulse_opacity(mode: Mode, phase: u8) -> f64 {
    match mode {
        // Recording changes every 350 ms; transcribing changes every 175 ms.
        Mode::Recording if phase < 2 => 1.0,
        Mode::Recording => 0.45,
        Mode::Transcribing if phase % 2 == 0 => 1.0,
        Mode::Transcribing => 0.2,
        Mode::Hidden | Mode::Error => 1.0,
    }
}

fn install_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_data(
        "
        window.echo-overlay-window { background: transparent; }
        .echo-overlay {
            background: alpha(#202124, 0.78);
            border: 1px solid alpha(white, 0.12);
            border-radius: 999px;
            padding: 12px 16px;
            color: white;
        }
        .echo-overlay-message { font-weight: 600; }
        ",
    );
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn configure_x11_window(surface: &gdk::Surface) {
    let xid = x11_surface_xid(surface);
    if xid == 0 {
        return;
    }
    let Ok((connection, _)) = x11rb::connect(None) else {
        return;
    };

    let _ = connection.change_window_attributes(
        xid as u32,
        &ChangeWindowAttributesAux::new().override_redirect(1),
    );

    let window_type = atom(&connection, b"_NET_WM_WINDOW_TYPE");
    let notification = atom(&connection, b"_NET_WM_WINDOW_TYPE_NOTIFICATION");
    let wm_state = atom(&connection, b"_NET_WM_STATE");
    let above = atom(&connection, b"_NET_WM_STATE_ABOVE");
    let skip_taskbar = atom(&connection, b"_NET_WM_STATE_SKIP_TASKBAR");
    let skip_pager = atom(&connection, b"_NET_WM_STATE_SKIP_PAGER");
    let sticky = atom(&connection, b"_NET_WM_STATE_STICKY");
    let desktop = atom(&connection, b"_NET_WM_DESKTOP");

    if let (Some(property), Some(value)) = (window_type, notification) {
        let _ = connection.change_property32(
            PropMode::REPLACE,
            xid as u32,
            property,
            AtomEnum::ATOM,
            &[value],
        );
    }
    if let Some(property) = wm_state {
        let states: Vec<u32> = [above, skip_taskbar, skip_pager, sticky]
            .into_iter()
            .flatten()
            .collect();
        let _ = connection.change_property32(
            PropMode::REPLACE,
            xid as u32,
            property,
            AtomEnum::ATOM,
            &states,
        );
    }
    if let Some(property) = desktop {
        let _ = connection.change_property32(
            PropMode::REPLACE,
            xid as u32,
            property,
            AtomEnum::CARDINAL,
            &[u32::MAX],
        );
    }

    // ICCCM WM_HINTS: InputHint is set, with input=false.
    let _ = connection.change_property32(
        PropMode::REPLACE,
        xid as u32,
        AtomEnum::WM_HINTS,
        AtomEnum::WM_HINTS,
        &[1, 0, 0, 0, 0, 0, 0, 0, 0],
    );
    let _ = connection.flush();
}

fn atom(connection: &impl Connection, name: &[u8]) -> Option<u32> {
    connection
        .intern_atom(false, name)
        .ok()?
        .reply()
        .ok()
        .map(|reply| reply.atom)
}

fn x11_surface_xid(surface: &gdk::Surface) -> c_ulong {
    unsafe { gdk_x11_surface_get_xid(surface.as_ptr().cast::<c_void>()) }
}

#[link(name = "gtk-4")]
extern "C" {
    fn gdk_x11_surface_get_xid(surface: *mut c_void) -> c_ulong;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_modes_are_distinct_and_hidden_by_default() {
        assert_eq!(Mode::default(), Mode::Hidden);
        assert_ne!(Mode::Recording, Mode::Transcribing);
        assert_ne!(Mode::Transcribing, Mode::Error);
    }

    #[test]
    fn transcribing_pulse_changes_twice_as_fast_as_recording() {
        assert_eq!(pulse_opacity(Mode::Recording, 0), 1.0);
        assert_eq!(pulse_opacity(Mode::Recording, 1), 1.0);
        assert_eq!(pulse_opacity(Mode::Recording, 2), 0.45);
        assert_eq!(pulse_opacity(Mode::Recording, 3), 0.45);

        assert_eq!(pulse_opacity(Mode::Transcribing, 0), 1.0);
        assert_eq!(pulse_opacity(Mode::Transcribing, 1), 0.2);
        assert_eq!(pulse_opacity(Mode::Transcribing, 2), 1.0);
        assert_eq!(pulse_opacity(Mode::Transcribing, 3), 0.2);
    }

    #[test]
    fn monitor_bounds_include_top_left_and_exclude_bottom_right() {
        let monitor = gdk::Rectangle::new(1920, 0, 1920, 1200);
        assert!(rectangle_contains(&monitor, 1920, 0));
        assert!(rectangle_contains(&monitor, 3839, 1199));
        assert!(!rectangle_contains(&monitor, 1919, 0));
        assert!(!rectangle_contains(&monitor, 3840, 1200));
    }
}

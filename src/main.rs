use adw::prelude::*;
use std::{cell::RefCell, rc::Rc};

mod settings;

#[derive(Debug, PartialEq, Eq)]
enum SessionSupport {
    X11,
    Unsupported,
}

fn session_support(backend: Option<gtk::gdk::Backend>) -> SessionSupport {
    match backend {
        Some(gtk::gdk::Backend::X11) => SessionSupport::X11,
        _ => SessionSupport::Unsupported,
    }
}

fn status_message(session: SessionSupport) -> &'static str {
    match session {
        SessionSupport::X11 => "Echo for Linux is ready.",
        SessionSupport::Unsupported => {
            "Echo for Linux requires an X11 session. Global shortcuts and pasting are disabled."
        }
    }
}

fn main() {
    let application = adw::Application::builder()
        .application_id("io.github.sahel.Echo")
        .build();

    let quit_action = gtk::gio::SimpleAction::new("quit", None);
    let quit_application = application.clone();
    quit_action.connect_activate(move |_, _| quit_application.quit());
    application.add_action(&quit_action);

    let window = Rc::new(RefCell::new(None));
    application.connect_activate({
        let window = window.clone();
        move |application| activate(application, &window)
    });
    application.run();
}

fn activate(
    application: &adw::Application,
    existing_window: &Rc<RefCell<Option<adw::ApplicationWindow>>>,
) {
    if existing_window.borrow().is_some() {
        existing_window
            .borrow()
            .as_ref()
            .expect("existing window checked above")
            .present();
        return;
    }

    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);

    let title = gtk::Label::new(Some("Echo"));
    title.add_css_class("title-1");
    title.set_halign(gtk::Align::Start);
    content.append(&title);

    let backend = gtk::gdk::Display::default().map(|display| display.backend());
    let message = gtk::Label::new(Some(status_message(session_support(backend))));
    message.set_halign(gtk::Align::Start);
    message.set_wrap(true);
    content.append(&message);

    if let Some(error_message) = settings_load_error_message() {
        let error = gtk::Label::new(Some(&error_message));
        error.add_css_class("error");
        error.set_halign(gtk::Align::Start);
        error.set_wrap(true);
        content.append(&error);
    }

    let quit = gtk::Button::with_label("Quit");
    quit.set_halign(gtk::Align::End);
    quit.set_action_name(Some("app.quit"));
    content.append(&quit);

    let window = adw::ApplicationWindow::builder()
        .application(application)
        .title("Echo")
        .default_width(360)
        .default_height(180)
        .content(&content)
        .build();

    window.connect_close_request(|window| {
        window.hide();
        gtk::glib::Propagation::Stop
    });
    *existing_window.borrow_mut() = Some(window.clone());
    window.present();
}

fn settings_load_error_message() -> Option<String> {
    let result = (|| {
        let store = settings::SettingsStore::for_current_user()?;
        let settings = store.load()?;
        store.save(&settings)
    })();

    result
        .err()
        .map(|error| format!("Couldn't load settings: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn x11_session_is_supported() {
        assert_eq!(
            session_support(Some(gtk::gdk::Backend::X11)),
            SessionSupport::X11
        );
        assert_eq!(
            status_message(SessionSupport::X11),
            "Echo for Linux is ready."
        );
    }

    #[test]
    fn non_x11_backends_are_rejected() {
        for backend in [
            None,
            Some(gtk::gdk::Backend::Wayland),
            Some(gtk::gdk::Backend::Broadway),
            Some(gtk::gdk::Backend::MacOS),
            Some(gtk::gdk::Backend::Win32),
        ] {
            assert_eq!(session_support(backend), SessionSupport::Unsupported);
        }
        assert!(status_message(SessionSupport::Unsupported).contains("requires an X11 session"));
    }
}

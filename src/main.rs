use adw::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
    thread,
    time::Duration,
};

mod secret;
mod settings;
mod shortcut;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    let session = session_support(backend);
    let (settings_store, settings, settings_error) = load_settings();
    let settings = Rc::new(RefCell::new(settings));
    let message = gtk::Label::new(Some(status_message(session)));
    message.set_halign(gtk::Align::Start);
    message.set_wrap(true);
    content.append(&message);

    let shortcut_controller = if session == SessionSupport::X11 {
        let shortcut_status = gtk::Label::new(Some("Starting global shortcut…"));
        shortcut_status.set_halign(gtk::Align::Start);
        shortcut_status.set_wrap(true);
        content.append(&shortcut_status);
        shortcut::binding_from_settings(&settings.borrow().shortcut)
            .map(|binding| Rc::new(start_shortcut_backend(binding, shortcut_status)))
    } else {
        None
    };

    if let Some(error_message) = settings_error {
        let error = gtk::Label::new(Some(&error_message));
        error.add_css_class("error");
        error.set_halign(gtk::Align::Start);
        error.set_wrap(true);
        content.append(&error);
    }

    let shortcut_binding_label =
        gtk::Label::new(Some(&shortcut_display(&settings.borrow().shortcut)));
    shortcut_binding_label.set_halign(gtk::Align::Start);
    content.append(&shortcut_binding_label);
    let change_shortcut = gtk::Button::with_label("Change shortcut");
    change_shortcut.set_halign(gtk::Align::Start);
    change_shortcut.set_sensitive(shortcut_controller.is_some());
    content.append(&change_shortcut);

    let api_key_status = gtk::Label::new(Some("Checking secure API-key storage…"));
    api_key_status.set_halign(gtk::Align::Start);
    api_key_status.set_wrap(true);
    content.append(&api_key_status);

    let api_key_input = gtk::PasswordEntry::new();
    api_key_input.set_placeholder_text(Some("Groq API key"));
    api_key_input.set_show_peek_icon(true);
    content.append(&api_key_input);

    let api_key_actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let save_api_key = gtk::Button::with_label("Save API key");
    let remove_api_key = gtk::Button::with_label("Remove API key");
    api_key_actions.append(&save_api_key);
    api_key_actions.append(&remove_api_key);
    content.append(&api_key_actions);

    start_secret_operation(SecretOperation::Check, api_key_status.clone());

    save_api_key.connect_clicked({
        let api_key_input = api_key_input.clone();
        let api_key_status = api_key_status.clone();
        move |_| {
            let key = api_key_input.text().to_string();
            api_key_input.set_text("");
            if key.trim().is_empty() {
                api_key_status.set_text("Enter an API key before saving.");
                return;
            }

            api_key_status.set_text("Saving API key…");
            start_secret_operation(SecretOperation::Save(key), api_key_status.clone());
        }
    });

    remove_api_key.connect_clicked({
        let api_key_status = api_key_status.clone();
        move |_| {
            api_key_status.set_text("Removing API key…");
            start_secret_operation(SecretOperation::Remove, api_key_status.clone());
        }
    });

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

    install_shortcut_capture(
        &window,
        &change_shortcut,
        &shortcut_binding_label,
        shortcut_controller,
        settings_store,
        settings.clone(),
    );

    window.connect_close_request(|window| {
        window.hide();
        gtk::glib::Propagation::Stop
    });
    *existing_window.borrow_mut() = Some(window.clone());
    window.present();
}

enum SecretOperation {
    Check,
    Save(String),
    Remove,
}

enum SecretOperationResult {
    Check(Result<secret::ApiKeyStatus, secret::SecretError>),
    Save(Result<(), secret::SecretError>),
    Remove(Result<(), secret::SecretError>),
}

fn start_secret_operation(operation: SecretOperation, status: gtk::Label) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = match operation {
            SecretOperation::Check => SecretOperationResult::Check(secret::api_key_status()),
            SecretOperation::Save(key) => SecretOperationResult::Save(secret::save_api_key(&key)),
            SecretOperation::Remove => SecretOperationResult::Remove(secret::remove_api_key()),
        };
        let _ = sender.send(result);
    });

    gtk::glib::timeout_add_local(Duration::from_millis(25), move || {
        match receiver.try_recv() {
            Ok(result) => {
                status.set_text(secret_operation_message(result));
                gtk::glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
            Err(mpsc::TryRecvError::Disconnected) => {
                status.set_text("Couldn't access secure API-key storage. Check that your desktop keyring is running.");
                gtk::glib::ControlFlow::Break
            }
        }
    });
}

fn secret_operation_message(result: SecretOperationResult) -> &'static str {
    match result {
        SecretOperationResult::Check(Ok(secret::ApiKeyStatus::Saved)) => "API key saved",
        SecretOperationResult::Check(Ok(secret::ApiKeyStatus::Missing)) => "No API key saved",
        SecretOperationResult::Save(Ok(())) => "API key saved",
        SecretOperationResult::Remove(Ok(())) => "API key removed",
        SecretOperationResult::Check(Err(_))
        | SecretOperationResult::Save(Err(_))
        | SecretOperationResult::Remove(Err(_)) => {
            "Couldn't access secure API-key storage. Check that your desktop keyring is running."
        }
    }
}

fn start_shortcut_backend(
    binding: shortcut::Binding,
    status: gtk::Label,
) -> shortcut::ShortcutController {
    let (sender, receiver) = mpsc::channel();
    let controller = shortcut::start_x11(binding, sender);

    gtk::glib::timeout_add_local(Duration::from_millis(25), move || {
        while let Ok(event) = receiver.try_recv() {
            status.set_text(shortcut_status_message(event));
        }
        gtk::glib::ControlFlow::Continue
    });
    controller
}

fn shortcut_status_message(event: shortcut::ShortcutEvent) -> &'static str {
    match event {
        shortcut::ShortcutEvent::Active => "Global shortcut is active.",
        shortcut::ShortcutEvent::Pressed => "Shortcut pressed (waiting for release).",
        shortcut::ShortcutEvent::Released => "Shortcut released.",
        shortcut::ShortcutEvent::Conflict => {
            "Couldn't claim shortcut. Another application is using it; Echo remains open."
        }
        shortcut::ShortcutEvent::Unavailable => {
            "Couldn't start global F10 shortcut. Check your X11 session."
        }
    }
}

fn load_settings() -> (
    Option<settings::SettingsStore>,
    settings::Settings,
    Option<String>,
) {
    match (|| {
        let store = settings::SettingsStore::for_current_user()?;
        let settings = store.load()?;
        store.save(&settings).map(|()| (store, settings))
    })() {
        Ok((store, settings)) => (Some(store), settings, None),
        Err(error) => (
            None,
            settings::Settings::default(),
            Some(format!("Couldn't load settings: {error}")),
        ),
    }
}

fn shortcut_display(shortcut: &settings::Shortcut) -> String {
    shortcut::binding_from_settings(shortcut)
        .map(|binding| shortcut::display_name(&binding))
        .unwrap_or_else(|| shortcut.key.clone())
}

fn install_shortcut_capture(
    window: &adw::ApplicationWindow,
    change_button: &gtk::Button,
    binding_label: &gtk::Label,
    controller: Option<Rc<shortcut::ShortcutController>>,
    store: Option<settings::SettingsStore>,
    settings: Rc<RefCell<settings::Settings>>,
) {
    let Some(controller) = controller else {
        return;
    };
    let capturing = Rc::new(Cell::new(false));
    let key_controller = gtk::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);

    change_button.connect_clicked({
        let capturing = capturing.clone();
        let binding_label = binding_label.clone();
        let window = window.clone();
        move |_| {
            capturing.set(true);
            binding_label.set_text("Press a key…");
            window.grab_focus();
        }
    });

    key_controller.connect_key_pressed({
        let capturing = capturing.clone();
        let binding_label = binding_label.clone();
        let settings = settings.clone();
        let controller = controller.clone();
        move |_, key, _, state| {
            if !capturing.get() {
                return gtk::glib::Propagation::Proceed;
            }
            if key == gtk::gdk::Key::Escape {
                capturing.set(false);
                binding_label.set_text(&shortcut_display(&settings.borrow().shortcut));
                return gtk::glib::Propagation::Stop;
            }
            let Some(binding) = shortcut::captured_binding(key, state) else {
                binding_label.set_text("Press a non-modifier key.");
                return gtk::glib::Propagation::Stop;
            };
            capturing.set(false);
            binding_label.set_text(&shortcut::display_name(&binding));
            let result = controller.update(binding.clone());
            let binding_label = binding_label.clone();
            let settings = settings.clone();
            let store = store.clone();
            gtk::glib::timeout_add_local(Duration::from_millis(25), move || {
                match result.try_recv() {
                    Ok(shortcut::UpdateResult::Applied) => {
                        settings.borrow_mut().shortcut = settings::Shortcut {
                            key: binding.key.clone(),
                            modifiers: binding.modifiers.clone(),
                        };
                        if let Some(store) = &store {
                            if store.save(&settings.borrow()).is_err() {
                                binding_label.set_text("Shortcut active, but couldn't save it.");
                            }
                        }
                        gtk::glib::ControlFlow::Break
                    }
                    Ok(shortcut::UpdateResult::Conflict) => {
                        binding_label
                            .set_text("Shortcut is already in use; kept the old shortcut.");
                        gtk::glib::ControlFlow::Break
                    }
                    Ok(shortcut::UpdateResult::Unavailable)
                    | Err(mpsc::TryRecvError::Disconnected) => {
                        binding_label
                            .set_text("Couldn't activate shortcut; kept the old shortcut.");
                        gtk::glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
                }
            });
            gtk::glib::Propagation::Stop
        }
    });
    window.add_controller(key_controller);
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

    #[test]
    fn api_key_status_messages_never_include_a_key_value() {
        assert_eq!(
            secret_operation_message(SecretOperationResult::Check(Ok(
                secret::ApiKeyStatus::Saved
            ))),
            "API key saved"
        );
        assert_eq!(
            secret_operation_message(SecretOperationResult::Check(Ok(
                secret::ApiKeyStatus::Missing
            ))),
            "No API key saved"
        );
    }

    #[test]
    fn shortcut_conflict_is_actionable_without_closing_echo() {
        assert_eq!(
            shortcut_status_message(shortcut::ShortcutEvent::Conflict),
            "Couldn't claim shortcut. Another application is using it; Echo remains open."
        );
    }
}

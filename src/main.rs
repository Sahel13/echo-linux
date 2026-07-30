use adw::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc,
    thread,
    time::Duration,
};

mod audio;
mod controller;
mod groq;
mod history;
mod overlay;
mod paste;
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
    let paste_backend = Rc::new(RefCell::new(None));
    application.connect_activate({
        let window = window.clone();
        let paste_backend = paste_backend.clone();
        move |application| activate(application, &window, &paste_backend)
    });
    application.run();
}

fn activate(
    application: &adw::Application,
    existing_window: &Rc<RefCell<Option<adw::ApplicationWindow>>>,
    paste_backend: &Rc<RefCell<Option<paste::PasteController>>>,
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
    if session == SessionSupport::X11 && paste_backend.borrow().is_none() {
        *paste_backend.borrow_mut() = Some(paste::start_x11());
    }
    let (settings_store, settings, settings_error) = load_settings();
    let settings = Rc::new(RefCell::new(settings));
    let history = Rc::new(RefCell::new(history::History::new(
        settings.borrow().total_words,
    )));
    let message = gtk::Label::new(Some(status_message(session)));
    message.set_halign(gtk::Align::Start);
    message.set_wrap(true);
    content.append(&message);

    let shortcut_runtime = if session == SessionSupport::X11 {
        let shortcut_status = gtk::Label::new(Some("Starting global shortcut…"));
        shortcut_status.set_halign(gtk::Align::Start);
        shortcut_status.set_wrap(true);
        content.append(&shortcut_status);
        shortcut::binding_from_settings(&settings.borrow().shortcut).map(|binding| {
            let (controller, events) = start_shortcut_backend(binding);
            (Rc::new(controller), events, shortcut_status)
        })
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
    change_shortcut.set_sensitive(shortcut_runtime.is_some());
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

    let shortcut_controller = shortcut_runtime
        .as_ref()
        .map(|(controller, _, _)| controller.clone());
    install_shortcut_capture(
        &window,
        &change_shortcut,
        &shortcut_binding_label,
        shortcut_controller,
        settings_store.clone(),
        settings.clone(),
    );

    install_microphone_selector(&content, settings_store.clone(), settings.clone());
    install_transcription_controls(&content, settings_store.clone(), settings.clone());
    let (word_count, copy_last_transcript, history_status) =
        install_history_controls(&content, history.clone());

    if let Some((shortcut_controller, shortcut_events, shortcut_status)) = shortcut_runtime {
        let overlay = overlay::Overlay::new(application);
        let transaction_status = gtk::Label::new(Some("Ready."));
        transaction_status.set_halign(gtk::Align::Start);
        transaction_status.set_wrap(true);
        content.append(&transaction_status);
        let diagnostic_status = gtk::Label::new(Some("Diagnostic: waiting for a transaction."));
        diagnostic_status.set_halign(gtk::Align::Start);
        diagnostic_status.set_wrap(true);
        content.append(&diagnostic_status);
        let history_runtime = controller::HistoryRuntime::new(
            settings_store.clone(),
            history,
            word_count,
            copy_last_transcript,
            history_status,
        );
        let dictation = Rc::new(RefCell::new(controller::DictationController::new(
            settings.clone(),
            history_runtime,
            shortcut_controller,
            paste_backend.borrow().clone(),
            transaction_status,
            diagnostic_status,
            overlay,
        )));
        gtk::glib::timeout_add_local(Duration::from_millis(25), move || {
            while let Ok(event) = shortcut_events.try_recv() {
                match event {
                    shortcut::ShortcutEvent::Pressed
                    | shortcut::ShortcutEvent::Released
                    | shortcut::ShortcutEvent::Escape => {
                        dictation.borrow_mut().handle_shortcut_event(event);
                    }
                    shortcut::ShortcutEvent::Active | shortcut::ShortcutEvent::Conflict => {
                        shortcut_status.set_text(shortcut_status_message(event));
                    }
                    shortcut::ShortcutEvent::Unavailable => {
                        dictation.borrow_mut().backend_unavailable();
                        shortcut_status.set_text(shortcut_status_message(event));
                    }
                }
            }
            dictation.borrow_mut().tick();
            gtk::glib::ControlFlow::Continue
        });
    }

    window.connect_close_request(|window| {
        window.hide();
        gtk::glib::Propagation::Stop
    });
    *existing_window.borrow_mut() = Some(window.clone());
    window.present();
}

fn install_history_controls(
    content: &gtk::Box,
    history: Rc<RefCell<history::History>>,
) -> (gtk::Label, gtk::Button, gtk::Label) {
    let heading = gtk::Label::new(Some("History"));
    heading.set_halign(gtk::Align::Start);
    content.append(&heading);

    let word_count = gtk::Label::new(Some(&format!(
        "Lifetime dictated words: {}",
        history.borrow().total_words()
    )));
    word_count.set_halign(gtk::Align::Start);
    content.append(&word_count);

    let copy = gtk::Button::with_label("Copy last transcript");
    copy.set_halign(gtk::Align::Start);
    copy.set_sensitive(false);
    content.append(&copy);

    let status = gtk::Label::new(Some("No transcript in this session."));
    status.set_halign(gtk::Align::Start);
    status.set_wrap(true);
    content.append(&status);

    copy.connect_clicked({
        let history = history.clone();
        let status = status.clone();
        move |button| {
            let transcript = history.borrow().last_transcript().to_owned();
            if transcript.is_empty() {
                button.set_sensitive(false);
                status.set_text("No transcript in this session.");
                return;
            }
            button.clipboard().set_text(&transcript);
            status.set_text("Last transcript copied.");
        }
    });

    (word_count, copy, status)
}

const MODEL_NAMES: [&str; 2] = ["whisper-large-v3-turbo", "whisper-large-v3"];
const LANGUAGE_NAMES: [&str; 14] = [
    "Auto-detect",
    "English",
    "Spanish",
    "French",
    "German",
    "Italian",
    "Portuguese",
    "Dutch",
    "Hindi",
    "Arabic",
    "Chinese",
    "Japanese",
    "Korean",
    "Russian",
];
const STYLE_NAMES: [&str; 2] = ["Normal", "Lower Case"];

fn install_transcription_controls(
    content: &gtk::Box,
    store: Option<settings::SettingsStore>,
    settings: Rc<RefCell<settings::Settings>>,
) {
    let heading = gtk::Label::new(Some("Transcription"));
    heading.set_halign(gtk::Align::Start);
    content.append(&heading);

    let model_label = gtk::Label::new(Some("Model"));
    model_label.set_halign(gtk::Align::Start);
    content.append(&model_label);
    let model = gtk::DropDown::from_strings(&MODEL_NAMES);
    model.set_selected(model_index(&settings.borrow().model));
    content.append(&model);

    let language_label = gtk::Label::new(Some("Language"));
    language_label.set_halign(gtk::Align::Start);
    content.append(&language_label);
    let language = gtk::DropDown::from_strings(&LANGUAGE_NAMES);
    language.set_selected(language_index(&settings.borrow().language));
    content.append(&language);

    let style_label = gtk::Label::new(Some("Style"));
    style_label.set_halign(gtk::Align::Start);
    content.append(&style_label);
    let style = gtk::DropDown::from_strings(&STYLE_NAMES);
    style.set_selected(style_index(&settings.borrow().style));
    content.append(&style);

    let vocabulary_label = gtk::Label::new(Some("Custom vocabulary"));
    vocabulary_label.set_halign(gtk::Align::Start);
    content.append(&vocabulary_label);
    let vocabulary = gtk::Entry::new();
    vocabulary.set_placeholder_text(Some("Names or terms to recognize"));
    vocabulary.set_text(&settings.borrow().vocabulary);
    content.append(&vocabulary);

    let status = gtk::Label::new(None);
    status.set_halign(gtk::Align::Start);
    status.set_wrap(true);
    content.append(&status);

    model.connect_selected_notify({
        let settings = settings.clone();
        let store = store.clone();
        let status = status.clone();
        move |model| {
            let Some(selected) = model_from_index(model.selected()) else {
                return;
            };
            settings.borrow_mut().model = selected;
            update_transcription_status(&status, &store, &settings);
        }
    });

    language.connect_selected_notify({
        let settings = settings.clone();
        let store = store.clone();
        let status = status.clone();
        move |language| {
            let Some(selected) = language_from_index(language.selected()) else {
                return;
            };
            settings.borrow_mut().language = selected;
            update_transcription_status(&status, &store, &settings);
        }
    });

    style.connect_selected_notify({
        let settings = settings.clone();
        let store = store.clone();
        let status = status.clone();
        move |style| {
            let Some(selected) = style_from_index(style.selected()) else {
                return;
            };
            settings.borrow_mut().style = selected;
            update_transcription_status(&status, &store, &settings);
        }
    });

    vocabulary.connect_changed({
        let settings = settings.clone();
        let store = store.clone();
        let status = status.clone();
        move |vocabulary| {
            settings.borrow_mut().vocabulary = vocabulary.text().into();
            update_transcription_status(&status, &store, &settings);
        }
    });
}

fn update_transcription_status(
    status: &gtk::Label,
    store: &Option<settings::SettingsStore>,
    settings: &Rc<RefCell<settings::Settings>>,
) {
    match store {
        Some(store) if store.save(&settings.borrow()).is_ok() => {
            status.set_text("Transcription settings saved.")
        }
        Some(_) => status.set_text("Couldn't save transcription settings."),
        None => status.set_text("Settings storage is unavailable."),
    }
}

fn model_index(model: &settings::Model) -> u32 {
    match model {
        settings::Model::WhisperLargeV3Turbo => 0,
        settings::Model::WhisperLargeV3 => 1,
    }
}

fn model_from_index(index: u32) -> Option<settings::Model> {
    match index {
        0 => Some(settings::Model::WhisperLargeV3Turbo),
        1 => Some(settings::Model::WhisperLargeV3),
        _ => None,
    }
}

fn language_index(language: &settings::Language) -> u32 {
    match language {
        settings::Language::AutoDetect => 0,
        settings::Language::English => 1,
        settings::Language::Spanish => 2,
        settings::Language::French => 3,
        settings::Language::German => 4,
        settings::Language::Italian => 5,
        settings::Language::Portuguese => 6,
        settings::Language::Dutch => 7,
        settings::Language::Hindi => 8,
        settings::Language::Arabic => 9,
        settings::Language::Chinese => 10,
        settings::Language::Japanese => 11,
        settings::Language::Korean => 12,
        settings::Language::Russian => 13,
    }
}

fn language_from_index(index: u32) -> Option<settings::Language> {
    Some(match index {
        0 => settings::Language::AutoDetect,
        1 => settings::Language::English,
        2 => settings::Language::Spanish,
        3 => settings::Language::French,
        4 => settings::Language::German,
        5 => settings::Language::Italian,
        6 => settings::Language::Portuguese,
        7 => settings::Language::Dutch,
        8 => settings::Language::Hindi,
        9 => settings::Language::Arabic,
        10 => settings::Language::Chinese,
        11 => settings::Language::Japanese,
        12 => settings::Language::Korean,
        13 => settings::Language::Russian,
        _ => return None,
    })
}

fn style_index(style: &settings::Style) -> u32 {
    match style {
        settings::Style::Normal => 0,
        settings::Style::LowerCase => 1,
    }
}

fn style_from_index(index: u32) -> Option<settings::Style> {
    match index {
        0 => Some(settings::Style::Normal),
        1 => Some(settings::Style::LowerCase),
        _ => None,
    }
}

fn install_microphone_selector(
    content: &gtk::Box,
    store: Option<settings::SettingsStore>,
    settings: Rc<RefCell<settings::Settings>>,
) {
    let input_label = gtk::Label::new(Some("Input"));
    input_label.set_halign(gtk::Align::Start);
    content.append(&input_label);

    let selector = gtk::DropDown::from_strings(&["System Default"]);
    selector.set_halign(gtk::Align::Fill);
    selector.set_hexpand(true);
    content.append(&selector);

    let status = gtk::Label::new(Some("Loading microphones…"));
    status.set_halign(gtk::Align::Start);
    status.set_wrap(true);
    content.append(&status);

    let refresh = gtk::Button::with_label("Refresh microphones");
    refresh.set_halign(gtk::Align::Start);
    content.append(&refresh);

    let devices = Rc::new(RefCell::new(Vec::<audio::InputDevice>::new()));
    let updating = Rc::new(Cell::new(false));
    let refresh_in_progress = Rc::new(Cell::new(false));

    selector.connect_selected_notify({
        let devices = devices.clone();
        let settings = settings.clone();
        let store = store.clone();
        let status = status.clone();
        let updating = updating.clone();
        move |selector| {
            if updating.get() {
                return;
            }
            let selection = match selector.selected() {
                0 => settings::Microphone::SystemDefault,
                selected => match usize::try_from(selected - 1)
                    .ok()
                    .and_then(|index| devices.borrow().get(index).cloned())
                {
                    Some(device) => settings::Microphone::Device { id: device.id },
                    None => return,
                },
            };
            settings.borrow_mut().microphone = selection;
            if let Some(store) = &store {
                if store.save(&settings.borrow()).is_err() {
                    status.set_text("Couldn't save microphone selection.");
                    return;
                }
            }
            status.set_text("Microphone selection saved.");
        }
    });

    let refresh_devices = Rc::new({
        let selector = selector.clone();
        let status = status.clone();
        let devices = devices.clone();
        let updating = updating.clone();
        let refresh_in_progress = refresh_in_progress.clone();
        let settings = settings.clone();
        let store = store.clone();
        move || {
            if refresh_in_progress.replace(true) {
                return;
            }
            start_microphone_refresh(
                selector.clone(),
                status.clone(),
                devices.clone(),
                updating.clone(),
                refresh_in_progress.clone(),
                settings.clone(),
                store.clone(),
            );
        }
    });

    refresh.connect_clicked({
        let refresh_devices = refresh_devices.clone();
        move |_| refresh_devices()
    });

    refresh_devices();
    gtk::glib::timeout_add_local(Duration::from_secs(2), move || {
        refresh_devices();
        gtk::glib::ControlFlow::Continue
    });
}

fn start_microphone_refresh(
    selector: gtk::DropDown,
    status: gtk::Label,
    devices: Rc<RefCell<Vec<audio::InputDevice>>>,
    updating: Rc<Cell<bool>>,
    refresh_in_progress: Rc<Cell<bool>>,
    settings: Rc<RefCell<settings::Settings>>,
    store: Option<settings::SettingsStore>,
) {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(audio::list_input_devices());
    });

    gtk::glib::timeout_add_local(Duration::from_millis(25), move || {
        match receiver.try_recv() {
            Ok(Ok(new_devices)) => {
                let selection = settings.borrow().microphone.clone();
                let (selection, fell_back) = audio::reconcile_selection(&selection, &new_devices);
                let strings = gtk::StringList::new(&[]);
                strings.append("System Default");
                for device in &new_devices {
                    strings.append(&device.name);
                }

                updating.set(true);
                *devices.borrow_mut() = new_devices;
                selector.set_model(Some(&strings));
                selector.set_selected(
                    audio::selected_index(&selection, &devices.borrow()).unwrap_or(0),
                );
                updating.set(false);

                if fell_back {
                    settings.borrow_mut().microphone = selection;
                    if let Some(store) = &store {
                        if store.save(&settings.borrow()).is_err() {
                            status.set_text("Selected microphone disappeared; using System Default. Couldn't save it.");
                        } else {
                            status
                                .set_text("Selected microphone disappeared; using System Default.");
                        }
                    } else {
                        status.set_text("Selected microphone disappeared; using System Default.");
                    }
                } else {
                    status.set_text("Microphones refreshed.");
                }
                refresh_in_progress.set(false);
                gtk::glib::ControlFlow::Break
            }
            Ok(Err(_)) | Err(mpsc::TryRecvError::Disconnected) => {
                status.set_text("Couldn't list microphones. Check your audio connection.");
                refresh_in_progress.set(false);
                gtk::glib::ControlFlow::Break
            }
            Err(mpsc::TryRecvError::Empty) => gtk::glib::ControlFlow::Continue,
        }
    });
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
) -> (
    shortcut::ShortcutController,
    mpsc::Receiver<shortcut::ShortcutEvent>,
) {
    let (sender, receiver) = mpsc::channel();
    let controller = shortcut::start_x11(binding, sender);

    (controller, receiver)
}

fn shortcut_status_message(event: shortcut::ShortcutEvent) -> &'static str {
    match event {
        shortcut::ShortcutEvent::Active => "Global shortcut is active.",
        shortcut::ShortcutEvent::Pressed => "Shortcut pressed (waiting for release).",
        shortcut::ShortcutEvent::Released => "Shortcut released.",
        shortcut::ShortcutEvent::Escape => "Recording cancelled.",
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

    #[test]
    fn transcription_control_options_cover_every_required_choice() {
        assert_eq!(MODEL_NAMES, ["whisper-large-v3-turbo", "whisper-large-v3"]);
        assert_eq!(
            LANGUAGE_NAMES,
            [
                "Auto-detect",
                "English",
                "Spanish",
                "French",
                "German",
                "Italian",
                "Portuguese",
                "Dutch",
                "Hindi",
                "Arabic",
                "Chinese",
                "Japanese",
                "Korean",
                "Russian",
            ]
        );
        assert_eq!(STYLE_NAMES, ["Normal", "Lower Case"]);

        for index in 0..LANGUAGE_NAMES.len() as u32 {
            let language = language_from_index(index).expect("listed language has a setting");
            assert_eq!(language_index(&language), index);
        }
        for index in 0..MODEL_NAMES.len() as u32 {
            let model = model_from_index(index).expect("listed model has a setting");
            assert_eq!(model_index(&model), index);
        }
        for index in 0..STYLE_NAMES.len() as u32 {
            let style = style_from_index(index).expect("listed style has a setting");
            assert_eq!(style_index(&style), index);
        }
    }
}

use crate::{
    audio::{self, FinalizedRecording, Recording},
    groq,
    paste::{PasteController, PasteResult},
    secret,
    settings::Settings,
    shortcut::{ShortcutController, ShortcutEvent},
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const MINIMUM_RECORDING_DURATION: Duration = Duration::from_millis(300);
const ERROR_DURATION: Duration = Duration::from_millis(2500);
const MINIMUM_SPEECH_AMPLITUDE: u16 = 200;
static RECORDING_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum State {
    Idle,
    Recording,
    Transcribing,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Event {
    Pressed,
    Released { long_enough: bool },
    Escape,
    CaptureFailed,
    Finalized,
    FinalizeFailed,
    Transcript { empty: bool },
    TranscriptionFailed,
    PasteInserted,
    PasteFailed,
    ErrorElapsed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Effect {
    StartRecording,
    CancelRecording,
    FinalizeRecording,
    Transcribe,
    Paste,
    ShowNoSpeech,
    ShowFailure,
    Complete,
}

/// The event-only portion of a dictation transaction. It deliberately holds no
/// GTK or backend state, which makes every valid and invalid transition testable.
#[derive(Debug)]
struct Machine {
    state: State,
}

impl Machine {
    fn new() -> Self {
        Self { state: State::Idle }
    }

    fn apply(&mut self, event: Event) -> Option<Effect> {
        match (self.state, event) {
            (State::Idle, Event::Pressed) => {
                self.state = State::Recording;
                Some(Effect::StartRecording)
            }
            (State::Recording, Event::Released { long_enough: false })
            | (State::Recording, Event::Escape) => {
                self.state = State::Idle;
                Some(Effect::CancelRecording)
            }
            (State::Recording, Event::Released { long_enough: true }) => {
                self.state = State::Transcribing;
                Some(Effect::FinalizeRecording)
            }
            (State::Recording | State::Transcribing, Event::CaptureFailed)
            | (State::Transcribing, Event::FinalizeFailed)
            | (State::Transcribing, Event::TranscriptionFailed)
            | (State::Transcribing, Event::PasteFailed) => {
                self.state = State::Error;
                Some(Effect::ShowFailure)
            }
            (State::Transcribing, Event::Finalized) => Some(Effect::Transcribe),
            (State::Transcribing, Event::Transcript { empty: false }) => Some(Effect::Paste),
            (State::Transcribing, Event::Transcript { empty: true }) => {
                self.state = State::Error;
                Some(Effect::ShowNoSpeech)
            }
            (State::Transcribing, Event::PasteInserted) => {
                self.state = State::Idle;
                Some(Effect::Complete)
            }
            (State::Error, Event::ErrorElapsed) => {
                self.state = State::Idle;
                Some(Effect::Complete)
            }
            _ => None,
        }
    }
}

/// GTK-main-thread owner of the one active dictation transaction. All slow
/// operations are started on workers and polled from the main loop.
pub struct DictationController {
    machine: Machine,
    settings: std::rc::Rc<std::cell::RefCell<Settings>>,
    shortcut: std::rc::Rc<ShortcutController>,
    paste: Option<PasteController>,
    status: gtk::Label,
    pressed_at: Option<Instant>,
    snapshot: Option<Settings>,
    recording: Option<Recording>,
    start_receiver: Option<Receiver<Result<Recording, audio::CaptureError>>>,
    finalize_receiver: Option<Receiver<Result<FinalizedRecording, audio::CaptureError>>>,
    transcription_receiver: Option<Receiver<Result<String, String>>>,
    paste_receiver: Option<Receiver<PasteResult>>,
    temporary_audio: Option<PathBuf>,
    release_requested: bool,
    error_deadline: Option<Instant>,
}

impl DictationController {
    pub fn new(
        settings: std::rc::Rc<std::cell::RefCell<Settings>>,
        shortcut: std::rc::Rc<ShortcutController>,
        paste: Option<PasteController>,
        status: gtk::Label,
    ) -> Self {
        Self {
            machine: Machine::new(),
            settings,
            shortcut,
            paste,
            status,
            pressed_at: None,
            snapshot: None,
            recording: None,
            start_receiver: None,
            finalize_receiver: None,
            transcription_receiver: None,
            paste_receiver: None,
            temporary_audio: None,
            release_requested: false,
            error_deadline: None,
        }
    }

    pub fn handle_shortcut_event(&mut self, event: ShortcutEvent) {
        match event {
            ShortcutEvent::Pressed => self.apply(Event::Pressed),
            ShortcutEvent::Released => {
                let long_enough = self
                    .pressed_at
                    .is_some_and(|pressed_at| pressed_at.elapsed() >= MINIMUM_RECORDING_DURATION);
                self.apply(Event::Released { long_enough });
            }
            ShortcutEvent::Escape => self.apply(Event::Escape),
            ShortcutEvent::Active | ShortcutEvent::Conflict | ShortcutEvent::Unavailable => {}
        }
    }

    pub fn tick(&mut self) {
        self.poll_recording_start();
        self.poll_finalization();
        self.poll_transcription();
        self.poll_paste();
        if self
            .error_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.apply(Event::ErrorElapsed);
        }
    }

    pub fn backend_unavailable(&mut self) {
        if self.machine.state == State::Recording {
            self.fail_with_message("Couldn't capture Escape; recording cancelled.");
        }
    }

    fn apply(&mut self, event: Event) {
        let Some(effect) = self.machine.apply(event) else {
            return;
        };
        match effect {
            Effect::StartRecording => self.start_recording(),
            Effect::CancelRecording => self.cancel_recording(),
            Effect::FinalizeRecording => self.request_finalization(),
            Effect::Transcribe => self.start_transcription(),
            Effect::Paste => self.start_paste(),
            Effect::ShowNoSpeech => self.fail("No speech detected."),
            Effect::ShowFailure => self.fail("Dictation failed."),
            Effect::Complete => self.complete(),
        }
    }

    fn start_recording(&mut self) {
        self.snapshot = Some(self.settings.borrow().clone());
        let microphone = self
            .snapshot
            .as_ref()
            .expect("settings snapshot set before recording")
            .microphone
            .clone();
        self.pressed_at = Some(Instant::now());
        self.release_requested = false;
        self.shortcut.set_recording(true);
        self.status.set_text("Recording…");
        self.start_receiver = Some(audio::start_recording(microphone));
        thread::spawn(|| {
            if let Ok(client) = groq::GroqClient::new() {
                client.prewarm();
            }
        });
    }

    fn cancel_recording(&mut self) {
        self.shortcut.set_recording(false);
        self.start_receiver = None;
        self.recording = None;
        self.release_requested = false;
        self.pressed_at = None;
        self.snapshot = None;
        self.remove_temporary_audio();
        self.status.set_text("Recording cancelled.");
    }

    fn request_finalization(&mut self) {
        self.shortcut.set_recording(false);
        self.pressed_at = None;
        self.status.set_text("Transcribing…");
        self.release_requested = true;
        self.start_finalization_if_ready();
    }

    fn start_finalization_if_ready(&mut self) {
        let Some(recording) = self.recording.take() else {
            return;
        };
        self.release_requested = false;
        let path = temporary_recording_path();
        self.temporary_audio = Some(path.clone());
        self.finalize_receiver = Some(recording.finish(path));
    }

    fn poll_recording_start(&mut self) {
        let result = self
            .start_receiver
            .as_ref()
            .and_then(|receiver| match receiver.try_recv() {
                Ok(result) => Some(Ok(result)),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(())),
                Err(mpsc::TryRecvError::Empty) => None,
            });
        let Some(result) = result else {
            return;
        };
        self.start_receiver = None;
        match result {
            Ok(Ok(recording)) if self.machine.state == State::Recording => {
                self.recording = Some(recording)
            }
            Ok(Ok(recording))
                if self.machine.state == State::Transcribing && self.release_requested =>
            {
                self.recording = Some(recording);
                self.start_finalization_if_ready();
            }
            Ok(Ok(recording)) => drop(recording),
            Ok(Err(error)) => self.fail_with_message(&error.to_string()),
            Err(()) => self.apply(Event::CaptureFailed),
        }
    }

    fn poll_finalization(&mut self) {
        let result =
            self.finalize_receiver
                .as_ref()
                .and_then(|receiver| match receiver.try_recv() {
                    Ok(result) => Some(Ok(result)),
                    Err(mpsc::TryRecvError::Disconnected) => Some(Err(())),
                    Err(mpsc::TryRecvError::Empty) => None,
                });
        let Some(result) = result else {
            return;
        };
        self.finalize_receiver = None;
        match result {
            Ok(Ok(recording)) if !contains_speech(&recording) => {
                self.apply(Event::Transcript { empty: true })
            }
            Ok(Ok(_recording)) => self.apply(Event::Finalized),
            Ok(Err(error)) => self.fail_with_message(&error.to_string()),
            Err(()) => self.apply(Event::FinalizeFailed),
        }
    }

    fn start_transcription(&mut self) {
        let Some(path) = self.temporary_audio.clone() else {
            self.apply(Event::TranscriptionFailed);
            return;
        };
        let Some(snapshot) = self.snapshot.clone() else {
            self.apply(Event::TranscriptionFailed);
            return;
        };
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let result = match secret::load_api_key() {
                Ok(api_key) => groq::GroqClient::new()
                    .and_then(|client| client.transcribe(&api_key, &path, &snapshot))
                    .map_err(|error| error.to_string()),
                Err(_) => Err(
                    "Couldn't access secure API-key storage. Check that your desktop keyring is running."
                        .into(),
                ),
            };
            let _ = sender.send(result);
        });
        self.transcription_receiver = Some(receiver);
    }

    fn poll_transcription(&mut self) {
        let result =
            self.transcription_receiver
                .as_ref()
                .and_then(|receiver| match receiver.try_recv() {
                    Ok(result) => Some(Ok(result)),
                    Err(mpsc::TryRecvError::Disconnected) => Some(Err(())),
                    Err(mpsc::TryRecvError::Empty) => None,
                });
        let Some(result) = result else {
            return;
        };
        self.transcription_receiver = None;
        match result {
            Ok(Ok(transcript)) => {
                let empty = transcript.is_empty();
                if !empty {
                    self.start_paste_with(transcript);
                }
                self.apply(Event::Transcript { empty });
            }
            Ok(Err(message)) => self.fail_with_message(&message),
            Err(()) => self.apply(Event::TranscriptionFailed),
        }
    }

    fn start_paste_with(&mut self, transcript: String) {
        if let Some(paste) = &self.paste {
            self.paste_receiver = Some(paste.insert(transcript));
        }
    }

    fn start_paste(&mut self) {
        if self.paste_receiver.is_none() {
            self.apply(Event::PasteFailed);
        }
    }

    fn poll_paste(&mut self) {
        let result = self
            .paste_receiver
            .as_ref()
            .and_then(|receiver| match receiver.try_recv() {
                Ok(result) => Some(Ok(result)),
                Err(mpsc::TryRecvError::Disconnected) => Some(Err(())),
                Err(mpsc::TryRecvError::Empty) => None,
            });
        let Some(result) = result else {
            return;
        };
        self.paste_receiver = None;
        match result {
            Ok(PasteResult::Inserted) => self.apply(Event::PasteInserted),
            Ok(PasteResult::ClipboardOnly) | Err(()) => {
                self.fail_with_message("Couldn't paste — transcript is on the clipboard.")
            }
        }
    }

    fn fail_with_message(&mut self, message: &str) {
        if self.machine.state != State::Error {
            self.machine.state = State::Error;
        }
        self.fail(message);
    }

    fn fail(&mut self, message: &str) {
        self.shortcut.set_recording(false);
        self.start_receiver = None;
        self.recording = None;
        self.finalize_receiver = None;
        self.transcription_receiver = None;
        self.paste_receiver = None;
        self.pressed_at = None;
        self.release_requested = false;
        self.snapshot = None;
        self.remove_temporary_audio();
        self.status.set_text(message);
        self.error_deadline = Some(Instant::now() + ERROR_DURATION);
    }

    fn complete(&mut self) {
        self.shortcut.set_recording(false);
        self.snapshot = None;
        self.error_deadline = None;
        self.remove_temporary_audio();
        self.status.set_text("Ready.");
    }

    fn remove_temporary_audio(&mut self) {
        remove_temporary_audio(&mut self.temporary_audio);
    }
}

fn temporary_recording_path() -> PathBuf {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let counter = RECORDING_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "echo-recording-{}-{timestamp}-{counter}.wav",
        std::process::id()
    ))
}

fn remove_temporary_audio(temporary_audio: &mut Option<PathBuf>) {
    if let Some(path) = temporary_audio.take() {
        let _ = std::fs::remove_file(path);
    }
}

fn contains_speech(recording: &FinalizedRecording) -> bool {
    recording.average_amplitude >= MINIMUM_SPEECH_AMPLITUDE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transitions_through_a_successful_hold_to_paste_transaction() {
        let mut machine = Machine::new();
        assert_eq!(machine.apply(Event::Pressed), Some(Effect::StartRecording));
        assert_eq!(machine.state, State::Recording);
        assert_eq!(
            machine.apply(Event::Released { long_enough: true }),
            Some(Effect::FinalizeRecording)
        );
        assert_eq!(machine.state, State::Transcribing);
        assert_eq!(machine.apply(Event::Finalized), Some(Effect::Transcribe));
        assert_eq!(
            machine.apply(Event::Transcript { empty: false }),
            Some(Effect::Paste)
        );
        assert_eq!(machine.apply(Event::PasteInserted), Some(Effect::Complete));
        assert_eq!(machine.state, State::Idle);
    }

    #[test]
    fn short_hold_and_escape_cancel_without_transcribing() {
        let mut machine = Machine::new();
        machine.apply(Event::Pressed);
        assert_eq!(
            machine.apply(Event::Released { long_enough: false }),
            Some(Effect::CancelRecording)
        );
        assert_eq!(machine.state, State::Idle);
        machine.apply(Event::Pressed);
        assert_eq!(machine.apply(Event::Escape), Some(Effect::CancelRecording));
        assert_eq!(machine.state, State::Idle);
    }

    #[test]
    fn failures_and_empty_speech_enter_error_until_the_timeout() {
        for event in [
            Event::CaptureFailed,
            Event::FinalizeFailed,
            Event::TranscriptionFailed,
            Event::PasteFailed,
        ] {
            let mut machine = Machine {
                state: if event == Event::CaptureFailed {
                    State::Recording
                } else {
                    State::Transcribing
                },
            };
            assert_eq!(machine.apply(event), Some(Effect::ShowFailure));
            assert_eq!(machine.state, State::Error);
            assert_eq!(machine.apply(Event::ErrorElapsed), Some(Effect::Complete));
            assert_eq!(machine.state, State::Idle);
        }

        let mut machine = Machine {
            state: State::Transcribing,
        };
        assert_eq!(
            machine.apply(Event::Transcript { empty: true }),
            Some(Effect::ShowNoSpeech)
        );
        assert_eq!(machine.state, State::Error);
    }

    #[test]
    fn invalid_events_and_overlapping_holds_do_not_change_state() {
        let invalid = [
            (State::Idle, Event::Released { long_enough: true }),
            (State::Idle, Event::Escape),
            (State::Recording, Event::Pressed),
            (State::Recording, Event::Finalized),
            (State::Transcribing, Event::Pressed),
            (State::Transcribing, Event::Escape),
            (State::Error, Event::Pressed),
            (State::Error, Event::Released { long_enough: true }),
        ];
        for (state, event) in invalid {
            let mut machine = Machine { state };
            assert_eq!(machine.apply(event), None);
            assert_eq!(machine.state, state);
        }
    }

    #[test]
    fn overlapping_hold_does_not_interrupt_the_first_transcription() {
        let mut machine = Machine::new();
        machine.apply(Event::Pressed);
        machine.apply(Event::Released { long_enough: true });
        assert_eq!(machine.state, State::Transcribing);
        assert_eq!(machine.apply(Event::Pressed), None);
        assert_eq!(machine.apply(Event::Released { long_enough: true }), None);
        assert_eq!(machine.apply(Event::Finalized), Some(Effect::Transcribe));
        assert_eq!(
            machine.apply(Event::Transcript { empty: false }),
            Some(Effect::Paste)
        );
    }

    #[test]
    fn silent_recordings_do_not_reach_groq() {
        let silent = FinalizedRecording {
            path: PathBuf::from("silent.wav"),
            frames: 16_000,
            average_amplitude: MINIMUM_SPEECH_AMPLITUDE - 1,
        };
        let speech = FinalizedRecording {
            average_amplitude: MINIMUM_SPEECH_AMPLITUDE,
            ..silent.clone()
        };
        assert!(!contains_speech(&silent));
        assert!(contains_speech(&speech));
    }

    #[test]
    fn temporary_audio_is_removed_after_every_terminal_exit() {
        for exit in ["success", "empty", "cancelled", "failure"] {
            let path = temporary_recording_path();
            std::fs::write(&path, b"temporary audio").expect("temporary recording is written");
            let mut temporary_audio = Some(path.clone());
            remove_temporary_audio(&mut temporary_audio);
            assert!(
                !path.exists(),
                "{exit} exit removes its temporary recording"
            );
        }
    }
}

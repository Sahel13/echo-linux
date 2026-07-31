use crate::settings::Microphone;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream,
};
use std::{
    collections::HashSet,
    fmt,
    fs::OpenOptions,
    os::unix::fs::OpenOptionsExt,
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver},
        Arc, Mutex,
    },
    thread,
};
use webrtc_vad::{SampleRate, Vad, VadMode};

const TARGET_SAMPLE_RATE: u32 = 16_000;
const TARGET_CHANNELS: u16 = 1;
const TARGET_BITS_PER_SAMPLE: u16 = 16;
/// A hard cap prevents a stuck global shortcut from retaining unbounded audio
/// in memory. Five minutes is deliberately well beyond a normal dictation.
pub const MAXIMUM_RECORDING_DURATION: std::time::Duration = std::time::Duration::from_secs(300);
const MAX_CAPTURED_SAMPLES: usize = TARGET_SAMPLE_RATE as usize * 300;
const VAD_FRAME_SAMPLES: usize = 320; // 20 ms at 16 kHz, required by WebRTC VAD.
/// Reject isolated WebRTC VAD positives from microphone noise. A 200 ms
/// contiguous run is still short enough for a brief spoken dictation.
pub const MINIMUM_VOICED_RUN_FRAMES: usize = 10;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputDevice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeviceListError;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureError {
    NoInputDevice,
    SelectedDeviceUnavailable,
    UnsupportedSampleFormat,
    StartFailed,
    DeviceLost,
    MaximumDurationExceeded,
    FinalizeFailed,
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::NoInputDevice => "No microphone is available.",
            Self::SelectedDeviceUnavailable => "The selected microphone is no longer available.",
            Self::UnsupportedSampleFormat => "The selected microphone uses an unsupported format.",
            Self::StartFailed => "Couldn't start microphone recording.",
            Self::DeviceLost => "Microphone recording stopped unexpectedly.",
            Self::MaximumDurationExceeded => "Recording reached the five-minute limit.",
            Self::FinalizeFailed => "Couldn't finalize microphone recording.",
        })
    }
}

impl std::error::Error for CaptureError {}

/// Metadata for a completed WAV file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FinalizedRecording {
    pub path: PathBuf,
    pub frames: usize,
    /// Number of fixed-duration frames classified as speech by WebRTC VAD.
    /// This is a count only; no sample or transcript data is retained.
    pub speech_frames: usize,
    /// Longest contiguous run of VAD speech frames, also only a count.
    pub longest_speech_run: usize,
}

/// A running CPAL capture. Its callback only converts and copies samples into
/// memory; WAV writing is deferred to [`Recording::finish`].
pub struct Recording {
    command_sender: mpsc::Sender<CaptureCommand>,
}

impl Recording {
    /// Stop capture and write a 16 kHz mono 16-bit PCM WAV on a worker thread.
    /// The returned receiver can be polled from the GTK main loop without
    /// blocking it.
    pub fn finish(self, path: PathBuf) -> Receiver<Result<FinalizedRecording, CaptureError>> {
        let (sender, receiver) = mpsc::channel();
        if self
            .command_sender
            .send(CaptureCommand::Finalize(path, sender.clone()))
            .is_err()
        {
            let _ = sender.send(Err(CaptureError::DeviceLost));
        }
        receiver
    }
}

enum CaptureCommand {
    Finalize(
        PathBuf,
        mpsc::Sender<Result<FinalizedRecording, CaptureError>>,
    ),
}

/// Start microphone capture on a worker thread. The receiver delivers an
/// active [`Recording`] only after the input stream is running.
pub fn start_recording(selection: Microphone) -> Receiver<Result<Recording, CaptureError>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = create_input_stream(&selection).map(|(stream, captured)| {
            let (command_sender, command_receiver) = mpsc::channel();
            (
                Recording { command_sender },
                stream,
                captured,
                command_receiver,
            )
        });
        match result {
            Ok((recording, stream, captured, command_receiver)) => {
                if sender.send(Ok(recording)).is_err() {
                    return;
                }
                if let Ok(CaptureCommand::Finalize(path, result_sender)) = command_receiver.recv() {
                    drop(stream);
                    let cleanup_path = path.clone();
                    let result = captured
                        .lock()
                        .map_err(|_| CaptureError::FinalizeFailed)
                        .and_then(|captured| {
                            if captured.device_lost {
                                Err(CaptureError::DeviceLost)
                            } else {
                                if captured.at_capacity() {
                                    Err(CaptureError::MaximumDurationExceeded)
                                } else {
                                    write_wav(path, &captured.samples)
                                }
                            }
                        });
                    send_finalization_result(result_sender, result, cleanup_path);
                }
            }
            Err(error) => {
                let _ = sender.send(Err(error));
            }
        }
    });
    receiver
}

fn send_finalization_result(
    result_sender: mpsc::Sender<Result<FinalizedRecording, CaptureError>>,
    result: Result<FinalizedRecording, CaptureError>,
    cleanup_path: PathBuf,
) {
    if result_sender.send(result).is_err() {
        let _ = std::fs::remove_file(cleanup_path);
    }
}

fn create_input_stream(
    selection: &Microphone,
) -> Result<(Stream, Arc<Mutex<CapturedAudio>>), CaptureError> {
    let device = selected_input_device(selection)?;
    let supported_config = device
        .default_input_config()
        .map_err(|_| CaptureError::StartFailed)?;
    let config = supported_config.config();
    let captured = Arc::new(Mutex::new(CapturedAudio::new(
        config.sample_rate.0,
        config.channels,
    )?));
    let failed_capture = captured.clone();
    let error_callback = move |_| {
        if let Ok(mut captured) = failed_capture.lock() {
            captured.device_lost = true;
        }
    };

    let stream = match supported_config.sample_format() {
        SampleFormat::I8 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            i8_to_i16,
        ),
        SampleFormat::I16 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            i16_to_i16,
        ),
        SampleFormat::I32 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            i32_to_i16,
        ),
        SampleFormat::I64 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            i64_to_i16,
        ),
        SampleFormat::U8 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            u8_to_i16,
        ),
        SampleFormat::U16 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            u16_to_i16,
        ),
        SampleFormat::U32 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            u32_to_i16,
        ),
        SampleFormat::U64 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            u64_to_i16,
        ),
        SampleFormat::F32 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            float_to_i16,
        ),
        SampleFormat::F64 => build_stream(
            &device,
            &config,
            captured.clone(),
            error_callback,
            f64_to_i16,
        ),
        _ => return Err(CaptureError::UnsupportedSampleFormat),
    }
    .map_err(|_| CaptureError::StartFailed)?;
    stream.play().map_err(|_| CaptureError::StartFailed)?;

    Ok((stream, captured))
}

fn build_stream<T: cpal::SizedSample + Send + 'static>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    captured: Arc<Mutex<CapturedAudio>>,
    error_callback: impl FnMut(cpal::StreamError) + Send + 'static,
    convert: impl Fn(T) -> i16 + Send + 'static,
) -> Result<Stream, cpal::BuildStreamError> {
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            if let Ok(mut captured) = captured.lock() {
                captured.append_interleaved(data.iter().copied().map(&convert));
            }
        },
        error_callback,
        None,
    )
}

fn selected_input_device(selection: &Microphone) -> Result<cpal::Device, CaptureError> {
    let host = cpal::default_host();
    match selection {
        Microphone::SystemDefault => host
            .default_input_device()
            .ok_or(CaptureError::NoInputDevice),
        Microphone::Device { id } => host
            .input_devices()
            .map_err(|_| CaptureError::NoInputDevice)?
            .find(|device| device.name().ok().as_deref() == Some(id))
            .ok_or(CaptureError::SelectedDeviceUnavailable),
    }
}

fn float_to_i16(value: f32) -> i16 {
    (value.clamp(-1.0, 1.0) * 32_767.0).round() as i16
}

fn i8_to_i16(value: i8) -> i16 {
    i16::from(value) << 8
}

fn i16_to_i16(value: i16) -> i16 {
    value
}

fn i32_to_i16(value: i32) -> i16 {
    (value >> 16) as i16
}

fn i64_to_i16(value: i64) -> i16 {
    (value >> 48) as i16
}

fn u8_to_i16(value: u8) -> i16 {
    (i16::from(value) - 128) << 8
}

fn u16_to_i16(value: u16) -> i16 {
    (i32::from(value) - 32_768) as i16
}

fn u32_to_i16(value: u32) -> i16 {
    ((i64::from(value) - 2_147_483_648) >> 16) as i16
}

fn u64_to_i16(value: u64) -> i16 {
    ((value as i128 - 9_223_372_036_854_775_808_i128) >> 48) as i16
}

fn f64_to_i16(value: f64) -> i16 {
    float_to_i16(value as f32)
}

/// Enumerate the inputs that are available from the active audio host.
///
/// CPAL does not expose a separate stable device identifier on Linux, so the
/// device name is retained as the selection identifier. This is sufficient to
/// restore a selection after a device is temporarily unavailable, while still
/// permitting the UI to fall back safely when that name disappears.
pub fn list_input_devices() -> Result<Vec<InputDevice>, DeviceListError> {
    let host = cpal::default_host();
    let devices = host.input_devices().map_err(|_| DeviceListError)?;
    let mut seen = HashSet::new();
    let mut inputs = devices
        .filter_map(|device| {
            let name = device.name().ok()?;
            seen.insert(name.clone()).then_some(InputDevice {
                id: name.clone(),
                name,
            })
        })
        .collect::<Vec<_>>();
    inputs.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(inputs)
}

pub fn selected_index(selection: &Microphone, devices: &[InputDevice]) -> Option<u32> {
    match selection {
        Microphone::SystemDefault => Some(0),
        Microphone::Device { id } => devices
            .iter()
            .position(|device| &device.id == id)
            .and_then(|index| u32::try_from(index + 1).ok()),
    }
}

/// Return the currently usable selection and whether a missing device forced a
/// fallback to the system default.
pub fn reconcile_selection(selection: &Microphone, devices: &[InputDevice]) -> (Microphone, bool) {
    if selected_index(selection, devices).is_some() {
        (selection.clone(), false)
    } else {
        (Microphone::SystemDefault, true)
    }
}

struct CapturedAudio {
    samples: Vec<i16>,
    sample_rate: u32,
    channels: usize,
    output_rate_remainder: u32,
    device_lost: bool,
    max_samples: usize,
}

impl CapturedAudio {
    fn new(sample_rate: u32, channels: u16) -> Result<Self, CaptureError> {
        if sample_rate == 0 || channels == 0 {
            return Err(CaptureError::UnsupportedSampleFormat);
        }
        Self::with_max_samples(sample_rate, channels, MAX_CAPTURED_SAMPLES)
    }

    fn with_max_samples(
        sample_rate: u32,
        channels: u16,
        max_samples: usize,
    ) -> Result<Self, CaptureError> {
        if sample_rate == 0 || channels == 0 || max_samples == 0 {
            return Err(CaptureError::UnsupportedSampleFormat);
        }
        Ok(Self {
            samples: Vec::new(),
            sample_rate,
            channels: usize::from(channels),
            output_rate_remainder: 0,
            device_lost: false,
            max_samples,
        })
    }

    fn append_interleaved(&mut self, samples: impl IntoIterator<Item = i16>) {
        let mut samples = samples.into_iter();
        loop {
            let mut total = 0_i32;
            for _ in 0..self.channels {
                let Some(sample) = samples.next() else {
                    return;
                };
                total += i32::from(sample);
            }
            let mono = (total / self.channels as i32) as i16;
            self.output_rate_remainder += TARGET_SAMPLE_RATE;
            while self.output_rate_remainder >= self.sample_rate {
                if self.at_capacity() {
                    return;
                }
                self.samples.push(mono);
                self.output_rate_remainder -= self.sample_rate;
            }
        }
    }

    fn at_capacity(&self) -> bool {
        self.samples.len() >= self.max_samples
    }
}

fn write_wav(path: PathBuf, samples: &[i16]) -> Result<FinalizedRecording, CaptureError> {
    let spec = hound::WavSpec {
        channels: TARGET_CHANNELS,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: TARGET_BITS_PER_SAMPLE,
        sample_format: hound::SampleFormat::Int,
    };
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .map_err(|_| CaptureError::FinalizeFailed)?;
    let mut writer = hound::WavWriter::new(file, spec).map_err(|_| CaptureError::FinalizeFailed)?;
    for sample in samples {
        writer
            .write_sample(*sample)
            .map_err(|_| CaptureError::FinalizeFailed)?;
    }
    writer
        .finalize()
        .map_err(|_| CaptureError::FinalizeFailed)?;
    let activity = voice_activity(samples);
    Ok(FinalizedRecording {
        path,
        frames: samples.len(),
        speech_frames: activity.speech_frames,
        longest_speech_run: activity.longest_run,
    })
}

struct VoiceActivity {
    speech_frames: usize,
    longest_run: usize,
}

fn voice_activity(samples: &[i16]) -> VoiceActivity {
    let mut vad = Vad::new_with_rate_and_mode(SampleRate::Rate16kHz, VadMode::Aggressive);
    let mut speech_frames = 0;
    let mut current_run = 0;
    let mut longest_run = 0;
    for frame in samples.chunks_exact(VAD_FRAME_SAMPLES) {
        if vad.is_voice_segment(frame).unwrap_or(false) {
            speech_frames += 1;
            current_run += 1;
            longest_run = longest_run.max(current_run);
        } else {
            current_run = 0;
        }
    }
    VoiceActivity {
        speech_frames,
        longest_run,
    }
}

#[cfg(test)]
fn longest_voiced_run(decisions: impl IntoIterator<Item = bool>) -> usize {
    let mut current_run = 0;
    let mut longest_run = 0;
    for speech in decisions {
        if speech {
            current_run += 1;
            longest_run = longest_run.max(current_run);
        } else {
            current_run = 0;
        }
    }
    longest_run
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn devices() -> Vec<InputDevice> {
        vec![
            InputDevice {
                id: "Built-in Microphone".into(),
                name: "Built-in Microphone".into(),
            },
            InputDevice {
                id: "USB Microphone".into(),
                name: "USB Microphone".into(),
            },
        ]
    }

    #[test]
    fn system_default_is_always_the_first_input_choice() {
        assert_eq!(
            selected_index(&Microphone::SystemDefault, &devices()),
            Some(0)
        );
    }

    #[test]
    fn connected_selected_microphone_keeps_its_choice() {
        let selection = Microphone::Device {
            id: "USB Microphone".into(),
        };
        assert_eq!(selected_index(&selection, &devices()), Some(2));
        assert_eq!(
            reconcile_selection(&selection, &devices()),
            (selection, false)
        );
    }

    #[test]
    fn disappeared_microphone_falls_back_to_system_default() {
        let selection = Microphone::Device {
            id: "USB Microphone".into(),
        };
        let connected = vec![devices()[0].clone()];
        assert_eq!(
            reconcile_selection(&selection, &connected),
            (Microphone::SystemDefault, true)
        );
    }

    #[test]
    fn converts_mono_integer_samples_to_pcm() {
        let mut captured = CapturedAudio::new(TARGET_SAMPLE_RATE, 1).unwrap();
        captured.append_interleaved([i16::MIN, 0, i16::MAX]);
        assert_eq!(captured.samples, [i16::MIN, 0, i16::MAX]);
    }

    #[test]
    fn downmixes_stereo_float_samples_to_pcm() {
        let mut captured = CapturedAudio::new(TARGET_SAMPLE_RATE, 2).unwrap();
        captured.append_interleaved([float_to_i16(1.0), float_to_i16(-1.0)]);
        assert_eq!(captured.samples, [0]);
    }

    #[test]
    fn resamples_native_audio_to_sixteen_khz() {
        let mut captured = CapturedAudio::new(48_000, 1).unwrap();
        captured.append_interleaved(std::iter::repeat_n(1_000, 48_000));
        assert_eq!(captured.samples.len(), 16_000);
    }

    #[test]
    fn capture_buffer_stops_at_its_configured_limit() {
        let mut captured = CapturedAudio::with_max_samples(TARGET_SAMPLE_RATE, 1, 2).unwrap();
        captured.append_interleaved([10, 20, 30]);

        assert_eq!(captured.samples, [10, 20]);
        assert!(captured.at_capacity());
    }

    #[test]
    fn abandoned_finalization_removes_its_temporary_wav() {
        let path = temporary_wav_path();
        std::fs::write(&path, b"temporary recording").unwrap();
        let (sender, receiver) = mpsc::channel();
        drop(receiver);

        send_finalization_result(sender, Err(CaptureError::FinalizeFailed), path.clone());

        assert!(!path.exists());
    }

    #[test]
    fn finalizes_a_valid_sixteen_khz_mono_pcm_wav() {
        let path = temporary_wav_path();
        let recording = write_wav(path.clone(), &[1, -2, 3]).unwrap();
        let reader = hound::WavReader::open(&path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.sample_rate, TARGET_SAMPLE_RATE);
        assert_eq!(spec.channels, TARGET_CHANNELS);
        assert_eq!(spec.bits_per_sample, TARGET_BITS_PER_SAMPLE);
        assert_eq!(spec.sample_format, hound::SampleFormat::Int);
        assert_eq!(recording.frames, 3);
        assert_eq!(recording.speech_frames, 0);
        assert_eq!(recording.longest_speech_run, 0);
        use std::os::unix::fs::PermissionsExt as _;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn web_rtc_vad_rejects_silent_frames() {
        let activity = voice_activity(&vec![0; VAD_FRAME_SAMPLES * 3]);
        assert_eq!(activity.speech_frames, 0);
        assert_eq!(activity.longest_run, 0);
    }

    #[test]
    fn requires_a_contiguous_vad_run_to_filter_sparse_noise_positives() {
        assert_eq!(
            longest_voiced_run([true, false, true, true, false, true]),
            2
        );
        assert!(longest_voiced_run([true; MINIMUM_VOICED_RUN_FRAMES]) >= MINIMUM_VOICED_RUN_FRAMES);
    }

    #[test]
    fn device_loss_does_not_poison_a_new_capture_buffer() {
        let mut failed = CapturedAudio::new(TARGET_SAMPLE_RATE, 1).unwrap();
        failed.device_lost = true;
        assert!(failed.device_lost);
        let next = CapturedAudio::new(TARGET_SAMPLE_RATE, 1).unwrap();
        assert!(!next.device_lost);
    }

    fn temporary_wav_path() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("echo-audio-{unique}.wav"))
    }
}

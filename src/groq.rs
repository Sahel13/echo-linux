#![allow(dead_code)] // FLOW-001 runs this client on its transcription worker.

use crate::settings::{Language, Model, Settings, Style};
use reqwest::{
    blocking::{
        multipart::{Form, Part},
        Client as HttpClient, Response,
    },
    StatusCode,
};
use serde::Deserialize;
use std::{error::Error as _, fmt, io::ErrorKind, path::Path, time::Duration};

const TRANSCRIPTIONS_ENDPOINT: &str = "https://api.groq.com/openai/v1/audio/transcriptions";
const NORMAL_STYLE_EXEMPLAR: &str = "The following is a professional transcript with proper capitalization, punctuation, and complete sentences. The meeting starts at 3pm, the budget is $12,500, and we are in room 204.";
const LOWER_CASE_STYLE_EXEMPLAR: &str = "here's a casual transcript with no capitalization and relaxed punctuation just lowercase text. i'll grab 2 coffees and meet you at 5";

/// A synchronous Groq client. Call it only from a worker thread; GTK callers
/// must communicate results back through the controller.
pub struct GroqClient {
    http: HttpClient,
    endpoint: String,
}

impl GroqClient {
    pub fn new() -> Result<Self, GroqError> {
        Self::for_endpoint(TRANSCRIPTIONS_ENDPOINT)
    }

    fn for_endpoint(endpoint: &str) -> Result<Self, GroqError> {
        Self::for_endpoint_with_timeout(endpoint, Duration::from_secs(10), Duration::from_secs(60))
    }

    fn for_endpoint_with_timeout(
        endpoint: &str,
        connect_timeout: Duration,
        timeout: Duration,
    ) -> Result<Self, GroqError> {
        let http = HttpClient::builder()
            .connect_timeout(connect_timeout)
            .timeout(timeout)
            .build()
            .map_err(|error| GroqError::Network(network_failure(&error)))?;
        Ok(Self {
            http,
            endpoint: endpoint.into(),
        })
    }

    /// Starts a best-effort connection before audio finalization. Its result is
    /// intentionally ignored: a failed prewarm must never prevent the actual
    /// transcription request.
    pub fn prewarm(&self) {
        let _ = self.http.head(&self.endpoint).send();
    }

    pub fn transcribe(
        &self,
        api_key: &str,
        wav_path: &Path,
        settings: &Settings,
    ) -> Result<String, GroqError> {
        if api_key.trim().is_empty() {
            return Err(GroqError::MissingApiKey);
        }

        let file = Part::file(wav_path)
            .map_err(|_| GroqError::AudioFileUnavailable)?
            .mime_str("audio/wav")
            .map_err(|_| GroqError::AudioFileUnavailable)?;
        let mut form = Form::new()
            .part("file", file)
            .text("model", model_id(&settings.model))
            .text(
                "prompt",
                transcription_prompt(&settings.vocabulary, &settings.style),
            )
            .text("response_format", "json");
        if let Some(language) = language_code(&settings.language) {
            form = form.text("language", language);
        }

        let response = self
            .http
            .post(&self.endpoint)
            .bearer_auth(api_key)
            .multipart(form)
            .send()
            .map_err(|error| GroqError::Network(network_failure(&error)))?;
        decode_response(response)
    }
}

fn decode_response(response: Response) -> Result<String, GroqError> {
    let status = response.status();
    let body = response
        .text()
        .map_err(|error| GroqError::Network(network_failure(&error)))?;
    if !status.is_success() {
        return Err(GroqError::Http {
            status,
            message: groq_error_message(&body),
        });
    }
    serde_json::from_str::<TranscriptionResponse>(&body)
        .map(|response| response.text.trim().into())
        .map_err(|_| GroqError::UnreadableResponse)
}

fn groq_error_message(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct ErrorResponse {
        error: ErrorBody,
    }

    #[derive(Deserialize)]
    struct ErrorBody {
        message: String,
    }

    serde_json::from_str::<ErrorResponse>(body)
        .ok()
        .and_then(|response| {
            let message = response.error.message.trim();
            (!message.is_empty()).then(|| message.chars().take(80).collect())
        })
}

fn network_failure(error: &reqwest::Error) -> NetworkFailure {
    if error.is_timeout() {
        return NetworkFailure::Timeout;
    }

    let mut source = error.source();
    let mut has_offline_io_error = false;
    while let Some(current) = source {
        if current
            .downcast_ref::<std::io::Error>()
            .is_some_and(|error| {
                matches!(
                    error.kind(),
                    ErrorKind::NetworkDown
                        | ErrorKind::NetworkUnreachable
                        | ErrorKind::HostUnreachable
                        | ErrorKind::NotConnected
                        | ErrorKind::ConnectionAborted
                        | ErrorKind::ConnectionReset
                )
            })
        {
            has_offline_io_error = true;
            break;
        }
        source = current.source();
    }
    if has_offline_io_error {
        NetworkFailure::Offline
    } else if error.is_connect() {
        // DNS, connection, and TLS failures all happen before an HTTP response.
        NetworkFailure::Unreachable
    } else {
        NetworkFailure::Other
    }
}

fn model_id(model: &Model) -> &'static str {
    match model {
        Model::WhisperLargeV3Turbo => "whisper-large-v3-turbo",
        Model::WhisperLargeV3 => "whisper-large-v3",
    }
}

fn language_code(language: &Language) -> Option<&'static str> {
    match language {
        Language::AutoDetect => None,
        Language::English => Some("en"),
        Language::Spanish => Some("es"),
        Language::French => Some("fr"),
        Language::German => Some("de"),
        Language::Italian => Some("it"),
        Language::Portuguese => Some("pt"),
        Language::Dutch => Some("nl"),
        Language::Hindi => Some("hi"),
        Language::Arabic => Some("ar"),
        Language::Chinese => Some("zh"),
        Language::Japanese => Some("ja"),
        Language::Korean => Some("ko"),
        Language::Russian => Some("ru"),
    }
}

fn transcription_prompt(vocabulary: &str, style: &Style) -> String {
    let exemplar = match style {
        Style::Normal => NORMAL_STYLE_EXEMPLAR,
        Style::LowerCase => LOWER_CASE_STYLE_EXEMPLAR,
    };
    let vocabulary = vocabulary.trim();
    if vocabulary.is_empty() {
        exemplar.into()
    } else {
        format!("{vocabulary}\n\n{exemplar}")
    }
}

#[derive(Debug)]
pub enum GroqError {
    MissingApiKey,
    AudioFileUnavailable,
    Network(NetworkFailure),
    Http {
        status: StatusCode,
        message: Option<String>,
    },
    UnreadableResponse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NetworkFailure {
    Offline,
    Timeout,
    Unreachable,
    Other,
}

impl fmt::Display for GroqError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingApiKey => "No API key — add one in Settings",
            Self::AudioFileUnavailable => "Couldn't read the recording.",
            Self::Network(NetworkFailure::Offline) => "No internet connection",
            Self::Network(NetworkFailure::Timeout) => "Request timed out",
            Self::Network(NetworkFailure::Unreachable) => "Can't reach Groq",
            Self::Network(NetworkFailure::Other) => "Network error — try again",
            Self::Http {
                message: Some(message),
                ..
            } => message,
            Self::Http { status, .. }
                if *status == StatusCode::UNAUTHORIZED || *status == StatusCode::FORBIDDEN =>
            {
                "Invalid API key — check Settings"
            }
            Self::Http { status, .. } if *status == StatusCode::PAYLOAD_TOO_LARGE => {
                "Recording too large for Groq"
            }
            Self::Http { status, .. } if *status == StatusCode::TOO_MANY_REQUESTS => {
                "Rate limited by Groq — try again shortly"
            }
            Self::Http { status, .. } if status.is_server_error() => {
                return write!(formatter, "Groq server error (HTTP {})", status.as_u16());
            }
            Self::Http { status, .. } => {
                return write!(formatter, "Groq request failed (HTTP {})", status.as_u16());
            }
            Self::UnreadableResponse => "Unreadable response from Groq",
        })
    }
}

impl std::error::Error for GroqError {}

#[derive(Deserialize)]
struct TranscriptionResponse {
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::{Read, Write},
        net::{TcpListener, TcpStream},
        path::PathBuf,
        sync::mpsc,
        thread,
        time::{SystemTime, UNIX_EPOCH},
    };

    const TEST_KEY: &str = "test-groq-api-key";

    #[test]
    fn transcription_request_contains_the_selected_groq_fields() {
        let server = MockServer::start(vec![MockResponse::json("{\"text\":\"  Hello Echo.  \"}")]);
        let settings = Settings {
            model: Model::WhisperLargeV3,
            language: Language::Japanese,
            style: Style::LowerCase,
            vocabulary: "Echo, Sahel".into(),
            ..Settings::default()
        };
        let wav = TestWav::create();

        let transcript = server
            .client()
            .transcribe(TEST_KEY, wav.path(), &settings)
            .expect("mock transcription succeeds");

        assert_eq!(transcript, "Hello Echo.");
        let request = server.next_request();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/openai/v1/audio/transcriptions");
        assert!(request
            .headers
            .to_ascii_lowercase()
            .contains("authorization: bearer test-groq-api-key"));
        assert!(request
            .headers
            .to_ascii_lowercase()
            .contains("multipart/form-data"));
        assert!(request.body.contains("name=\"file\""));
        assert!(request.body.contains("filename=\"echo-groq-test"));
        assert!(request.body.contains("Content-Type: audio/wav"));
        assert_field(&request.body, "model", "whisper-large-v3");
        assert_field(&request.body, "language", "ja");
        assert_field(&request.body, "response_format", "json");
        assert!(request
            .body
            .contains(&transcription_prompt("Echo, Sahel", &Style::LowerCase)));
    }

    #[test]
    fn auto_detect_omits_the_language_multipart_field() {
        let server = MockServer::start(vec![MockResponse::json("{\"text\":\"hello\"}")]);
        let settings = Settings {
            language: Language::AutoDetect,
            ..Settings::default()
        };
        let wav = TestWav::create();

        server
            .client()
            .transcribe(TEST_KEY, wav.path(), &settings)
            .expect("mock transcription succeeds");

        assert!(!server.next_request().body.contains("name=\"language\""));
    }

    #[test]
    fn empty_vocabulary_still_sends_the_selected_style_exemplar() {
        let server = MockServer::start(vec![MockResponse::json("{\"text\":\"hello\"}")]);
        let settings = Settings {
            vocabulary: "  \n ".into(),
            ..Settings::default()
        };
        let wav = TestWav::create();

        server
            .client()
            .transcribe(TEST_KEY, wav.path(), &settings)
            .expect("mock transcription succeeds");

        assert!(server.next_request().body.contains(NORMAL_STYLE_EXEMPLAR));
    }

    #[test]
    fn failed_prewarm_does_not_prevent_a_later_transcription() {
        let server = MockServer::start(vec![
            MockResponse::connection_failure(),
            MockResponse::json("{\"text\":\"hello\"}"),
        ]);
        let client = server.client();
        let wav = TestWav::create();

        client.prewarm();
        let transcript = client
            .transcribe(TEST_KEY, wav.path(), &Settings::default())
            .expect("transcription remains independent from prewarm");

        assert_eq!(transcript, "hello");
        assert_eq!(server.next_request().method, "HEAD");
        assert_eq!(server.next_request().method, "POST");
    }

    #[test]
    fn failure_statuses_map_to_actionable_messages_without_retrying() {
        assert_response_error(401, "{}", "Invalid API key — check Settings");
        assert_response_error(403, "{}", "Invalid API key — check Settings");
        assert_response_error(413, "{}", "Recording too large for Groq");
        assert_response_error(429, "{}", "Rate limited by Groq — try again shortly");
        assert_response_error(503, "{}", "Groq server error (HTTP 503)");
        assert_response_error(400, "{}", "Groq request failed (HTTP 400)");
    }

    #[test]
    fn malformed_success_response_is_not_exposed_to_the_user() {
        let server = MockServer::start(vec![MockResponse::json("not json")]);
        let wav = TestWav::create();

        let error = server
            .client()
            .transcribe(TEST_KEY, wav.path(), &Settings::default())
            .expect_err("malformed response is rejected");

        assert_eq!(error.to_string(), "Unreadable response from Groq");
    }

    #[test]
    fn groq_error_message_is_trimmed_bounded_and_preferred() {
        let server = MockServer::start(vec![MockResponse::status(
            400,
            "{\"error\":{\"message\":\"  a useful message that is deliberately long enough to exceed the eighty-character user-facing limit safely  \"}}",
        )]);
        let wav = TestWav::create();

        let error = server
            .client()
            .transcribe(TEST_KEY, wav.path(), &Settings::default())
            .expect_err("error response is returned");
        let message = error.to_string();

        assert_eq!(message.chars().count(), 80);
        assert!(message.starts_with("a useful message"));
        assert!(!message.contains(TEST_KEY));
        assert!(!message.contains("transcript text"));
    }

    #[test]
    fn missing_key_and_network_failures_have_short_private_messages() {
        let wav = TestWav::create();
        let missing_key = GroqClient::for_endpoint("http://127.0.0.1:1")
            .expect("client builds")
            .transcribe("  ", wav.path(), &Settings::default())
            .expect_err("blank key is rejected before a request");
        assert_eq!(missing_key.to_string(), "No API key — add one in Settings");

        assert_eq!(
            GroqError::Network(NetworkFailure::Offline).to_string(),
            "No internet connection"
        );
        assert_eq!(
            GroqError::Network(NetworkFailure::Unreachable).to_string(),
            "Can't reach Groq"
        );
        assert_eq!(
            GroqError::Network(NetworkFailure::Other).to_string(),
            "Network error — try again"
        );
    }

    #[test]
    fn a_timed_out_request_maps_to_the_timeout_message() {
        let server = MockServer::start(vec![MockResponse::status_after(
            200,
            "{\"text\":\"never returned\"}",
            Duration::from_millis(50),
        )]);
        let wav = TestWav::create();

        let error = server
            .client_with_timeout(Duration::from_millis(10))
            .transcribe(TEST_KEY, wav.path(), &Settings::default())
            .expect_err("slow response times out");

        assert_eq!(error.to_string(), "Request timed out");
    }

    fn assert_response_error(status: u16, body: &'static str, expected: &str) {
        let server = MockServer::start(vec![MockResponse::status(status, body)]);
        let wav = TestWav::create();

        let error = server
            .client()
            .transcribe(TEST_KEY, wav.path(), &Settings::default())
            .expect_err("error response does not retry");

        assert_eq!(error.to_string(), expected);
        assert_eq!(server.next_request().method, "POST");
    }

    fn assert_field(body: &str, name: &str, value: &str) {
        assert!(
            body.contains(&format!("name=\"{name}\"\r\n\r\n{value}\r\n")),
            "missing {name}={value} in multipart body"
        );
    }

    struct TestWav {
        path: PathBuf,
    }

    impl TestWav {
        fn create() -> Self {
            let suffix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock is after Unix epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "echo-groq-test-{}-{suffix}.wav",
                std::process::id()
            ));
            std::fs::write(&path, b"RIFFtestWAVEfmt ").expect("test recording writes");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TestWav {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.path);
        }
    }

    struct MockServer {
        endpoint: String,
        requests: mpsc::Receiver<RecordedRequest>,
        worker: Option<thread::JoinHandle<()>>,
    }

    impl MockServer {
        fn start(responses: Vec<MockResponse>) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("mock server binds");
            let endpoint = format!(
                "http://{}/openai/v1/audio/transcriptions",
                listener.local_addr().expect("mock address is available")
            );
            let (sender, requests) = mpsc::channel();
            let worker = thread::spawn(move || {
                for response in responses {
                    let (mut stream, _) = listener.accept().expect("mock request arrives");
                    let request = read_request(&mut stream).expect("mock request reads");
                    sender.send(request).expect("test receives request");
                    response.write_to(&mut stream);
                }
            });
            Self {
                endpoint,
                requests,
                worker: Some(worker),
            }
        }

        fn client(&self) -> GroqClient {
            GroqClient::for_endpoint(&self.endpoint).expect("client builds")
        }

        fn client_with_timeout(&self, timeout: Duration) -> GroqClient {
            GroqClient::for_endpoint_with_timeout(&self.endpoint, timeout, timeout)
                .expect("client builds")
        }

        fn next_request(&self) -> RecordedRequest {
            self.requests.recv().expect("captured request is available")
        }
    }

    impl Drop for MockServer {
        fn drop(&mut self) {
            if let Some(worker) = self.worker.take() {
                worker.join().expect("mock server exits");
            }
        }
    }

    struct MockResponse {
        status: Option<u16>,
        body: &'static str,
        delay: Duration,
    }

    impl MockResponse {
        fn json(body: &'static str) -> Self {
            Self {
                status: Some(200),
                body,
                delay: Duration::ZERO,
            }
        }

        fn status(status: u16, body: &'static str) -> Self {
            Self {
                status: Some(status),
                body,
                delay: Duration::ZERO,
            }
        }

        fn status_after(status: u16, body: &'static str, delay: Duration) -> Self {
            Self {
                status: Some(status),
                body,
                delay,
            }
        }

        fn connection_failure() -> Self {
            Self {
                status: None,
                body: "",
                delay: Duration::ZERO,
            }
        }

        fn write_to(&self, stream: &mut TcpStream) {
            let Some(status) = self.status else {
                return;
            };
            thread::sleep(self.delay);
            let _ = write!(
                stream,
                "HTTP/1.1 {} Test\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                status,
                self.body.len(),
                self.body
            );
        }
    }

    struct RecordedRequest {
        method: String,
        path: String,
        headers: String,
        body: String,
    }

    fn read_request(stream: &mut TcpStream) -> std::io::Result<RecordedRequest> {
        let mut bytes = Vec::new();
        let mut read_buffer = [0; 4096];
        let header_end = loop {
            let read = stream.read(&mut read_buffer)?;
            bytes.extend_from_slice(&read_buffer[..read]);
            if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                break end + 4;
            }
        };
        let headers = String::from_utf8_lossy(&bytes[..header_end]).into_owned();
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("content-length")
                    .then_some(value.trim())
            })
            .and_then(|length| length.parse::<usize>().ok())
            .unwrap_or(0);
        while bytes.len() < header_end + content_length {
            let read = stream.read(&mut read_buffer)?;
            bytes.extend_from_slice(&read_buffer[..read]);
        }
        let request_line = headers.lines().next().expect("request line is present");
        let mut request_parts = request_line.split_whitespace();
        Ok(RecordedRequest {
            method: request_parts.next().unwrap_or_default().into(),
            path: request_parts.next().unwrap_or_default().into(),
            headers,
            body: String::from_utf8_lossy(&bytes[header_end..header_end + content_length])
                .into_owned(),
        })
    }
}

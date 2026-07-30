use serde::{Deserialize, Serialize};
use std::{
    env, fmt,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const SETTINGS_VERSION: u32 = 1;
const APPLICATION_DIRECTORY: &str = "echo";
const SETTINGS_FILENAME: &str = "settings.json";

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Settings {
    pub shortcut: Shortcut,
    pub model: Model,
    pub language: Language,
    pub style: Style,
    pub microphone: Microphone,
    pub vocabulary: String,
    pub launch_at_login: bool,
    pub total_words: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            shortcut: Shortcut::default(),
            model: Model::WhisperLargeV3Turbo,
            language: Language::English,
            style: Style::Normal,
            microphone: Microphone::SystemDefault,
            vocabulary: String::new(),
            launch_at_login: false,
            total_words: 0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shortcut {
    pub key: String,
    pub modifiers: Vec<Modifier>,
}

impl Default for Shortcut {
    fn default() -> Self {
        Self {
            key: "XF86Favorites".into(),
            modifiers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Modifier {
    Control,
    Alt,
    Shift,
    Super,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Model {
    #[serde(rename = "whisper-large-v3-turbo")]
    WhisperLargeV3Turbo,
    #[serde(rename = "whisper-large-v3")]
    WhisperLargeV3,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Language {
    #[serde(rename = "")]
    AutoDetect,
    #[serde(rename = "en")]
    English,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "fr")]
    French,
    #[serde(rename = "de")]
    German,
    #[serde(rename = "it")]
    Italian,
    #[serde(rename = "pt")]
    Portuguese,
    #[serde(rename = "nl")]
    Dutch,
    #[serde(rename = "hi")]
    Hindi,
    #[serde(rename = "ar")]
    Arabic,
    #[serde(rename = "zh")]
    Chinese,
    #[serde(rename = "ja")]
    Japanese,
    #[serde(rename = "ko")]
    Korean,
    #[serde(rename = "ru")]
    Russian,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Style {
    Normal,
    #[serde(rename = "lower_case")]
    LowerCase,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum Microphone {
    #[serde(rename = "system_default")]
    SystemDefault,
    Device {
        id: String,
    },
}

#[derive(Debug)]
pub enum SettingsError {
    ConfigHomeUnavailable,
    Read {
        path: PathBuf,
        source: io::Error,
    },
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    UnsupportedVersion {
        found: u32,
    },
    Serialize(serde_json::Error),
    Write {
        path: PathBuf,
        source: io::Error,
    },
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigHomeUnavailable => {
                formatter.write_str("XDG configuration directory is unavailable")
            }
            Self::Read { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
            Self::Parse { path, source } => {
                write!(formatter, "could not parse {}: {source}", path.display())
            }
            Self::UnsupportedVersion { found } => {
                write!(formatter, "settings version {found} is not supported")
            }
            Self::Serialize(source) => write!(formatter, "could not serialize settings: {source}"),
            Self::Write { path, source } => {
                write!(formatter, "could not write {}: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for SettingsError {}

#[derive(Clone, Debug)]
pub struct SettingsStore {
    path: PathBuf,
}

impl SettingsStore {
    pub fn for_current_user() -> Result<Self, SettingsError> {
        Ok(Self::at(settings_path_for(
            env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            env::var_os("HOME").map(PathBuf::from),
        )?))
    }

    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Settings, SettingsError> {
        match fs::read(&self.path) {
            Ok(contents) => decode_settings(&self.path, &contents),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Settings::default()),
            Err(source) => Err(SettingsError::Read {
                path: self.path.clone(),
                source,
            }),
        }
    }

    pub fn save(&self, settings: &Settings) -> Result<(), SettingsError> {
        let contents = encode_settings(settings)?;
        write_atomically(&self.path, contents.as_bytes()).map_err(|source| SettingsError::Write {
            path: self.path.clone(),
            source,
        })
    }
}

fn settings_path_for(
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf, SettingsError> {
    let config_home = if let Some(path) = xdg_config_home {
        if path.is_absolute() {
            path
        } else {
            home.filter(|path| path.is_absolute())
                .map(|path| path.join(".config"))
                .ok_or(SettingsError::ConfigHomeUnavailable)?
        }
    } else {
        home.filter(|path| path.is_absolute())
            .map(|path| path.join(".config"))
            .ok_or(SettingsError::ConfigHomeUnavailable)?
    };

    Ok(config_home
        .join(APPLICATION_DIRECTORY)
        .join(SETTINGS_FILENAME))
}

fn encode_settings(settings: &Settings) -> Result<String, SettingsError> {
    serde_json::to_string_pretty(&SettingsDocument {
        version: SETTINGS_VERSION,
        settings: settings.clone(),
    })
    .map(|contents| format!("{contents}\n"))
    .map_err(SettingsError::Serialize)
}

fn decode_settings(path: &Path, contents: &[u8]) -> Result<Settings, SettingsError> {
    let document = serde_json::from_slice::<SettingsDocument>(contents).map_err(|source| {
        SettingsError::Parse {
            path: path.to_path_buf(),
            source,
        }
    })?;

    if document.version != SETTINGS_VERSION {
        return Err(SettingsError::UnsupportedVersion {
            found: document.version,
        });
    }

    Ok(document.settings)
}

fn write_atomically(path: &Path, contents: &[u8]) -> io::Result<()> {
    write_atomically_after_sync(path, contents, |_| Ok(()))
}

fn write_atomically_after_sync(
    path: &Path,
    contents: &[u8],
    after_sync: impl FnOnce(&Path) -> io::Result<()>,
) -> io::Result<()> {
    let directory = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "settings path does not have a parent directory",
        )
    })?;
    fs::create_dir_all(directory)?;

    let temporary_path = temporary_path(path);
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        file.write_all(contents)?;
        file.sync_all()?;
        after_sync(&temporary_path)?;
        replace_file(&temporary_path, path)?;
        sync_directory(directory)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn temporary_path(path: &Path) -> PathBuf {
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("tmp-{}-{counter}", std::process::id()))
}

#[cfg(unix)]
fn replace_file(temporary_path: &Path, path: &Path) -> io::Result<()> {
    fs::rename(temporary_path, path)
}

#[cfg(not(unix))]
fn replace_file(temporary_path: &Path, path: &Path) -> io::Result<()> {
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(temporary_path, path)
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<()> {
    File::open(directory)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_: &Path) -> io::Result<()> {
    Ok(())
}

#[derive(Deserialize, Serialize)]
struct SettingsDocument {
    version: u32,
    #[serde(flatten)]
    settings: Settings,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_settings_path() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before Unix epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!(
                "echo-settings-test-{}-{unique}",
                std::process::id()
            ))
            .join(SETTINGS_FILENAME)
    }

    fn changed_settings() -> Settings {
        Settings {
            shortcut: Shortcut {
                key: "F9".into(),
                modifiers: vec![Modifier::Control, Modifier::Shift],
            },
            model: Model::WhisperLargeV3,
            language: Language::Japanese,
            style: Style::LowerCase,
            microphone: Microphone::Device {
                id: "test-device".into(),
            },
            vocabulary: "Echo, Groq".into(),
            launch_at_login: true,
            total_words: 42,
        }
    }

    #[test]
    fn defaults_match_the_configured_values() {
        assert_eq!(
            Settings::default(),
            Settings {
                shortcut: Shortcut {
                    key: "XF86Favorites".into(),
                    modifiers: vec![],
                },
                model: Model::WhisperLargeV3Turbo,
                language: Language::English,
                style: Style::Normal,
                microphone: Microphone::SystemDefault,
                vocabulary: String::new(),
                launch_at_login: false,
                total_words: 0,
            }
        );
    }

    #[test]
    fn current_user_store_uses_the_xdg_configuration_directory() {
        assert_eq!(
            settings_path_for(Some(PathBuf::from("/tmp/echo-config")), None)
                .expect("absolute XDG configuration directory is accepted"),
            PathBuf::from("/tmp/echo-config/echo/settings.json")
        );
    }

    #[test]
    fn changed_settings_reload_from_a_new_store() {
        let path = temporary_settings_path();
        let store = SettingsStore::at(path.clone());
        let settings = changed_settings();

        store.save(&settings).expect("settings save succeeds");

        let restarted_store = SettingsStore::at(path.clone());
        assert_eq!(
            restarted_store.load().expect("settings reload succeeds"),
            settings
        );

        fs::remove_dir_all(path.parent().expect("settings parent directory"))
            .expect("test settings directory removal succeeds");
    }

    #[test]
    fn interrupted_write_keeps_the_last_valid_settings_readable() {
        let path = temporary_settings_path();
        let store = SettingsStore::at(path.clone());
        let previous = changed_settings();
        store
            .save(&previous)
            .expect("initial settings save succeeds");

        let interrupted = write_atomically_after_sync(&path, b"{ interrupted", |_| {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "simulated process interruption",
            ))
        });
        assert_eq!(
            interrupted
                .expect_err("simulated write is interrupted")
                .kind(),
            io::ErrorKind::Interrupted
        );

        assert_eq!(
            store.load().expect("previous settings remain readable"),
            previous
        );

        fs::remove_dir_all(path.parent().expect("settings parent directory"))
            .expect("test settings directory removal succeeds");
    }

    #[test]
    fn saved_file_never_contains_api_key_or_transcript() {
        let path = temporary_settings_path();
        let store = SettingsStore::at(path.clone());
        let settings = changed_settings();
        store.save(&settings).expect("settings save succeeds");

        let contents = fs::read_to_string(&path).expect("settings file is readable");
        assert!(!contents.contains("api_key"));
        assert!(!contents.contains("transcript"));

        fs::remove_dir_all(path.parent().expect("settings parent directory"))
            .expect("test settings directory removal succeeds");
    }
}

use std::{
    env, fmt, fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const AUTOSTART_DIRECTORY: &str = "autostart";
const DESKTOP_FILENAME: &str = "io.github.sahel.Echo.desktop";
static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub struct Autostart {
    desktop_path: PathBuf,
    executable_path: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Enabled,
    Missing,
    ExecutableChanged,
    Invalid,
}

#[derive(Debug)]
pub enum AutostartError {
    ConfigHomeUnavailable,
    CurrentExecutable(io::Error),
    CurrentExecutableNotAbsolute(PathBuf),
    Read(io::Error),
    Write(io::Error),
    Remove(io::Error),
}

impl fmt::Display for AutostartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigHomeUnavailable => {
                formatter.write_str("XDG configuration directory is unavailable")
            }
            Self::CurrentExecutable(source) => {
                write!(
                    formatter,
                    "current executable path is unavailable: {source}"
                )
            }
            Self::CurrentExecutableNotAbsolute(path) => {
                write!(
                    formatter,
                    "current executable path is not absolute: {}",
                    path.display()
                )
            }
            Self::Read(source) => write!(formatter, "could not read autostart entry: {source}"),
            Self::Write(source) => write!(formatter, "could not write autostart entry: {source}"),
            Self::Remove(source) => write!(formatter, "could not remove autostart entry: {source}"),
        }
    }
}

impl std::error::Error for AutostartError {}

impl Autostart {
    pub fn for_current_user() -> Result<Self, AutostartError> {
        let executable_path = env::current_exe().map_err(AutostartError::CurrentExecutable)?;
        let desktop_path = autostart_path_for(
            env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            env::var_os("HOME").map(PathBuf::from),
        )?;
        Self::at(desktop_path, executable_path)
    }

    fn at(desktop_path: PathBuf, executable_path: PathBuf) -> Result<Self, AutostartError> {
        if !executable_path.is_absolute() {
            return Err(AutostartError::CurrentExecutableNotAbsolute(
                executable_path,
            ));
        }
        Ok(Self {
            desktop_path,
            executable_path,
        })
    }

    pub fn enable(&self) -> Result<(), AutostartError> {
        write_atomically(
            &self.desktop_path,
            desktop_entry(&self.executable_path).as_bytes(),
        )
        .map_err(AutostartError::Write)
    }

    pub fn disable(&self) -> Result<(), AutostartError> {
        match fs::remove_file(&self.desktop_path) {
            Ok(()) => {
                if let Some(directory) = self.desktop_path.parent() {
                    sync_directory(directory).map_err(AutostartError::Remove)?;
                }
                Ok(())
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(AutostartError::Remove(error)),
        }
    }

    pub fn status(&self) -> Result<Status, AutostartError> {
        let contents = match fs::read_to_string(&self.desktop_path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Status::Missing),
            Err(error) => return Err(AutostartError::Read(error)),
        };
        let Some(exec) = desktop_value(&contents, "Exec") else {
            return Ok(Status::Invalid);
        };
        let Some(path) = parse_quoted_exec(exec) else {
            return Ok(Status::Invalid);
        };
        if path == self.executable_path {
            Ok(Status::Enabled)
        } else {
            Ok(Status::ExecutableChanged)
        }
    }
}

fn autostart_path_for(
    xdg_config_home: Option<PathBuf>,
    home: Option<PathBuf>,
) -> Result<PathBuf, AutostartError> {
    let config_home = if let Some(path) = xdg_config_home {
        if path.is_absolute() {
            path
        } else {
            home.filter(|path| path.is_absolute())
                .map(|path| path.join(".config"))
                .ok_or(AutostartError::ConfigHomeUnavailable)?
        }
    } else {
        home.filter(|path| path.is_absolute())
            .map(|path| path.join(".config"))
            .ok_or(AutostartError::ConfigHomeUnavailable)?
    };
    Ok(config_home.join(AUTOSTART_DIRECTORY).join(DESKTOP_FILENAME))
}

fn desktop_entry(executable_path: &Path) -> String {
    format!(
        "[Desktop Entry]\nType=Application\nName=Echo\nComment=Hold a shortcut to dictate text\nExec=\"{}\"\nTerminal=false\nX-GNOME-Autostart-enabled=true\n",
        escape_exec_path(executable_path)
    )
}

fn escape_exec_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
}

fn desktop_value<'a>(contents: &'a str, key: &str) -> Option<&'a str> {
    contents.lines().find_map(|line| {
        line.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
    })
}

fn parse_quoted_exec(value: &str) -> Option<PathBuf> {
    let value = value.strip_prefix('"')?.strip_suffix('"')?;
    let mut parsed = String::new();
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character == '\\' {
            match characters.next()? {
                '\\' => parsed.push('\\'),
                '"' => parsed.push('"'),
                '`' => parsed.push('`'),
                '$' => parsed.push('$'),
                _ => return None,
            }
        } else {
            parsed.push(character);
        }
    }
    Some(PathBuf::from(parsed))
}

fn write_atomically(path: &Path, contents: &[u8]) -> io::Result<()> {
    let directory = path.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "autostart path does not have a parent directory",
        )
    })?;
    fs::create_dir_all(directory)?;
    let counter = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temporary_path = path.with_extension(format!("tmp-{}-{counter}", std::process::id()));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temporary_path, path)?;
        sync_directory(directory)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary_path);
    }
    result
}

#[cfg(unix)]
fn sync_directory(directory: &Path) -> io::Result<()> {
    fs::File::open(directory)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temporary_path() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before Unix epoch")
            .as_nanos();
        env::temp_dir()
            .join(format!(
                "echo-autostart-test-{}-{unique}",
                std::process::id()
            ))
            .join(DESKTOP_FILENAME)
    }

    #[test]
    fn xdg_path_uses_the_autostart_directory() {
        assert_eq!(
            autostart_path_for(Some(PathBuf::from("/tmp/echo-config")), None)
                .expect("absolute XDG configuration directory is accepted"),
            PathBuf::from("/tmp/echo-config/autostart/io.github.sahel.Echo.desktop")
        );
    }

    #[test]
    fn enable_writes_a_valid_entry_for_the_current_path_and_disable_removes_it() {
        let desktop_path = temporary_path();
        let executable_path = PathBuf::from("/opt/Echo Folder/echo");
        let autostart = Autostart::at(desktop_path.clone(), executable_path.clone())
            .expect("absolute executable path is accepted");

        autostart.enable().expect("autostart enable succeeds");
        let contents = fs::read_to_string(&desktop_path).expect("desktop entry is readable");
        assert!(contents.starts_with("[Desktop Entry]\n"));
        assert!(contents.contains("Type=Application\n"));
        assert!(contents.contains("Name=Echo\n"));
        assert!(contents.contains("Exec=\"/opt/Echo Folder/echo\"\n"));
        assert!(contents.contains("Terminal=false\n"));
        assert_eq!(
            autostart.status().expect("entry is inspectable"),
            Status::Enabled
        );

        autostart.disable().expect("autostart disable succeeds");
        assert!(!desktop_path.exists());
        fs::remove_dir_all(desktop_path.parent().expect("desktop entry has a parent"))
            .expect("temporary directory removal succeeds");
    }

    #[test]
    fn a_binary_move_requires_retoggling_instead_of_rewriting_the_entry() {
        let desktop_path = temporary_path();
        let original = Autostart::at(desktop_path.clone(), PathBuf::from("/opt/echo-old/echo"))
            .expect("original path is absolute");
        original.enable().expect("original entry is written");
        let original_contents =
            fs::read_to_string(&desktop_path).expect("original entry is readable");

        let moved = Autostart::at(desktop_path.clone(), PathBuf::from("/opt/echo-new/echo"))
            .expect("moved path is absolute");
        assert_eq!(
            moved.status().expect("moved entry is inspectable"),
            Status::ExecutableChanged
        );
        assert_eq!(
            fs::read_to_string(&desktop_path).expect("entry remains readable"),
            original_contents
        );

        fs::remove_dir_all(desktop_path.parent().expect("desktop entry has a parent"))
            .expect("temporary directory removal succeeds");
    }
}

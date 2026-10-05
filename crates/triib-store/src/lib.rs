//! Where triib keeps its files, and saving them so a crash never leaves a
//! half written one.

pub mod atomic;
pub mod paths;

use std::fmt;
use std::io;
use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

/// Why a settings file could not be used.
#[derive(Debug)]
pub enum LoadError {
    Io(io::Error),
    Parse(toml::de::Error),
}

impl fmt::Display for LoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "cannot read the settings: {error}"),
            Self::Parse(error) => write!(formatter, "the settings file is not valid: {error}"),
        }
    }
}

impl std::error::Error for LoadError {}

/// Loads the settings at `path`, or when there are none yet the defaults,
/// then writes `path` so it lists every setting: a file from before a
/// setting existed gets it at its default.
pub fn load_or_create<T>(path: &Path) -> Result<T, LoadError>
where
    T: Serialize + DeserializeOwned + Default,
{
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let settings = T::default();
            save(path, &settings).map_err(LoadError::Io)?;
            return Ok(settings);
        }
        Err(error) => return Err(LoadError::Io(error)),
    };
    let settings: T = toml::from_str(&text).map_err(LoadError::Parse)?;
    let present: toml::Table = toml::from_str(&text).map_err(LoadError::Parse)?;
    let all =
        toml::Table::try_from(&settings).map_err(|error| LoadError::Io(io::Error::other(error)))?;
    if all.keys().any(|key| !present.contains_key(key)) {
        save(path, &settings).map_err(LoadError::Io)?;
    }
    Ok(settings)
}

/// Writes `value` as TOML, creating the folder first.
pub fn save<T: Serialize>(path: &Path, value: &T) -> io::Result<()> {
    let text = toml::to_string_pretty(value).map_err(io::Error::other)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    atomic::write(path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    struct Sample {
        name: String,
        count: u32,
    }

    impl Default for Sample {
        fn default() -> Self {
            Self {
                name: "triib".to_owned(),
                count: 3,
            }
        }
    }

    #[test]
    fn creates_the_file_with_every_setting() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested/sample.toml");
        assert_eq!(load_or_create::<Sample>(&path).unwrap(), Sample::default());
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("name = \"triib\"") && text.contains("count = 3"));
    }

    #[test]
    fn adds_settings_a_file_lacks_and_keeps_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.toml");
        std::fs::write(&path, "count = 7\n").unwrap();
        let loaded = load_or_create::<Sample>(&path).unwrap();
        assert_eq!(loaded.count, 7);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("name = \"triib\"") && text.contains("count = 7"));
    }

    #[test]
    fn a_file_that_does_not_parse_is_an_error_and_left_alone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample.toml");
        std::fs::write(&path, "count = \"many\"\n").unwrap();
        assert!(matches!(
            load_or_create::<Sample>(&path),
            Err(LoadError::Parse(_))
        ));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "count = \"many\"\n"
        );
    }
}

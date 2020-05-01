use std::error::Error;
use std::fmt::{Display, Formatter, Result};
use std::io::Error as ioError;
use toml::de::Error as deError;

/// Error wrapper allowing for the Io and deserialization to be simply handled at once.
#[derive(Debug)]
pub enum ConfigError {
    Io(ioError),
    Deserialize(deError),
    DirNotFound(String),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            ConfigError::Io(ref err) => write!(f, "An error occurred while handling file: {}", err),
            ConfigError::Deserialize(ref err) => write!(f, "An error occurred while parsing file: {}", err),
            ConfigError::DirNotFound(s) => write!(f, "Could not determine system: {}", s),
        }
    }
}

impl Error for ConfigError {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            ConfigError::Io(ref err) => Some(err),
            ConfigError::Deserialize(ref err) => Some(err),
            ConfigError::DirNotFound(_) => None,
        }
    }
}

impl From<ioError> for ConfigError {
    fn from(err: ioError) -> ConfigError {
        ConfigError::Io(err)
    }
}

impl From<deError> for ConfigError {
    fn from(err: deError) -> ConfigError {
        ConfigError::Deserialize(err)
    }
}

impl From<String> for ConfigError {
    fn from(dir: String) -> ConfigError {
        ConfigError::DirNotFound(dir)
    }
}

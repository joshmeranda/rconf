use std::error::{self, Error};
use std::fmt::{Display, Formatter, Result};
use std::io::{Error as ioError};
use toml::de::Error as deError;

/// Error wrapper allowing for the Io and deserialization to be simply handled at once.
#[derive(Debug)]
pub enum ConfigError {
    Io(ioError),
    Deserialize(deError),
}

impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            ConfigError::Io(ref err) => write!(f, "An error occurred while handling file: {}", err),
            ConfigError::Deserialize(ref err) => write!(f, "An error occurred while parsing file: {}", err),
        }
    }
}

impl error::Error for ConfigError {
    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            ConfigError::Io(ref err) => Some(err),
            ConfigError::Deserialize(ref err) => Some(err),
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

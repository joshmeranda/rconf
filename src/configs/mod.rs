pub mod error;
pub mod path;

use self::error::*;
use self::path::*;
use serde_derive::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Error as ioError, ErrorKind};
use std::path::Path;
use tar::{Archive, Builder, Entries};

/// Describes package manager commands to install and uninstall packages as well as upgrade the
/// system.
#[derive(Deserialize, Serialize)]
pub struct Manager {
    pub name: String,
    packages: Vec<String>,
    install_args: String,
    un_install_args: Option<String>,
    upgrade_args: Option<String>,
    reboot_after_upgrade: Option<bool>,
}

//impl Manager {
//    /// Install the specified packages.
//    fn run_install_cmd(&self) -> Command {
//        unimplemented!();
//    }
//
//    /// Uninstall the specified packages.
//    fn run_uninstall_cmd(&self) -> Option<Command> {
//        unimplemented!();
//    }
//
//    /// Upgrade the system, and optionally reboot after system upgrade.
//    fn run_upgrade_cmd(&self) -> Option<Command> {
//        unimplemented!();
//    }
//}

/// Container for all of the configuration options.
#[derive(Deserialize, Serialize)]
pub struct Configs {
    pub path_specifier: Option<PathSpecifier>,
    pub manager: Option<Manager>,
}

impl Configs {
    /// Create a new Configs instantiation from specified configuration file.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be raised on any error opening, reading, or deserializing the contents.
    pub fn with_file(path: &Path) -> Result<Configs, ConfigError> {
        let contents = fs::read_to_string(path)?;
        let cfg: Configs = toml::from_str(&contents[..])?;

        Ok(cfg)
    }

    /// Create a new Configs instantiation from the contents of a tar archive.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be raised on any error opening, reading, or deserializing the contents.
    pub fn with_archive(path: &Path) -> Result<Configs, ConfigError> {
        let mut archive = Archive::new(File::open(path)?);
        let entries: Entries<File> = archive.entries()?;

        for entry in entries {
            let mut entry = entry?;
            let name = entry.path()?;

            if *name == *Path::new(".rconf") {
                // attempt to unpack the file to the platforms temp directory
                let tmp_dir = std::env::temp_dir();

                if entry.unpack_in(&tmp_dir)? {
                    // obtain path to tmp dir
                    let mut path = tmp_dir;
                    path.push(".rconf");

                    let cfg = Configs::with_file(&path);

                    // remove the unpacked file
                    fs::remove_file(path)?;

                    return cfg;
                } else {
                    return Err(ConfigError::Io(ioError::new(
                        ErrorKind::Other,
                        "Cannot unpack to parent directory.s",
                    )));
                }
            }
        }

        Err(ConfigError::Io(ioError::new(
            ErrorKind::NotFound,
            "The archive did not contain the required '.rconf' file",
        )))
    }

    /// Package configuration files into a tar archive and write to the system.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be raised on any error opening, reading, or deserializing the contents.
    pub fn write_archive(&self, path: &Path) -> Result<File, ConfigError> {
        // TODO: add '.tar' if no extension is specified in path
        // TODO: use GzEncoder if the extensions ends in '.gz'
        let file = File::create(path)?;
        let mut builder = Builder::new(file);

        match &self.path_specifier {
            None => println!("Writing empty archive!"),
            Some(specifier) => builder.append_path_specifier(specifier)?
        };

        Ok(builder.into_inner()?)
    }
}

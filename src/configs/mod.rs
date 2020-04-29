pub mod error;
pub mod path;

use self::error::*;
use self::path::*;
use serde_derive::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, BufReader};
use std::path::Path;
use tar::{Archive, Builder, Header};

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

/// Container for all of the configuration options.
#[derive(Deserialize, Serialize)]
pub struct Configs {
    pub path_specifier: Option<PathSpecifier>,
    pub manager: Option<Manager>
}

pub struct ConfigArchive {
    cfg: Configs,
    archive: Archive<File>
}

impl ConfigArchive {
    fn retrieve_configs<P: AsRef<Path>>(path: P) -> Result<Configs, ConfigError> {
    // read archive
        let file = File::open(path)?;
        let mut archive = Archive::new(file);
        let entries = archive.entries()?;
        let mut cfg: Option<Configs> = None;

        // find configuration file
        for entry in entries {
            let mut entry = entry?;
            if entry.path()?.to_str() == Some(".rconf") {
                let mut content = String::new();
                entry.read_to_string(&mut content)?;

                cfg = Some(toml::from_str(content.as_str())?);
            }
        }

        Ok(cfg.unwrap())
    }

    pub fn with_archive<P: AsRef<Path>>(path: P) -> Result<ConfigArchive, ConfigError> {
        let cfg = ConfigArchive::retrieve_configs(&path)?;
        let file = File::open(path)?;
        let archive = Archive::new(file);

        Ok(ConfigArchive {
            cfg,
            archive
        })
    }

    pub fn install(&mut self) -> Result<(), ConfigError>{
        let entries = self.archive.entries()?;

        for entry in entries {
            if let Ok(e) = entry {
                println!("{}", e.path()?.to_str().unwrap());
            }
        }

        Ok(())
    }
}

impl Configs {
    /// Create a new Configs instantiation from specified configuration file.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be raised on any error opening,
    /// reading, or deserializing the contents.
    pub fn with_file(path: &Path) -> Result<Configs, ConfigError> {
        let contents = fs::read_to_string(path)?;
        let cfg: Configs = toml::from_str(&contents[..])?;

        Ok(cfg)
    }

    /// Package configuration files into a tar archive and write to the system.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be raised on any error opening,
    /// reading, or deserializing the contents.
    pub fn write_archive(&self, path: &Path) -> Result<File, ConfigError> {
        // TODO: add rconf file
        let file = File::create(path)?;
        let mut builder = Builder::new(file);

        // append the the data from this configuration struct
        let content = toml::to_string_pretty(self).unwrap();
        let path  = Path::new(".rconf");
        let mut header = Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_cksum();

        builder.append_data(&mut header, path, content.as_bytes());

        // add the files from the specifier into the archive
        if self.path_specifier.is_some() {
            builder.append_path_specifier(self.path_specifier.as_ref().unwrap())?;
        }

        Ok(builder.into_inner()?)
    }
}

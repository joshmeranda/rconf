#[macro_use]
pub mod path;
pub mod error;

use self::error::*;
use self::path::*;
use serde_derive::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use tar::{Archive, Builder, Header};

/// Describes package manager commands to install and uninstall packages as well as upgrade the
/// system.
#[derive(Deserialize, Serialize)]
pub struct Manager {
    /// The name of the package manager (pacman, yum, apt, etc)
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
    pub manager: Option<Manager>,
}

impl Configs {
    /// Create a new Configs instantiation from specified configuration file.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error reading from
    /// the specified file, or parsing the contents.
    pub fn with_file(path: &Path) -> Result<Configs, ConfigError> {
        let contents = fs::read_to_string(path)?;
        let cfg: Configs = toml::from_str(&contents[..])?;

        Ok(cfg)
    }

    /// Package configuration files into a tar archive and write to the system. See
    /// [append_path_specifier]
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error creating the
    /// archive, or adding files and their contents to it.
    pub fn write_archive(&self, path: &Path) -> Result<File, ConfigError> {
        let file = File::create(path)?;
        let mut builder = Builder::new(file);

        // append the the data from this configuration struct
        let content = toml::to_string_pretty(self).unwrap();
        let path = Path::new(".rconf");
        let mut header = Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_cksum();

        builder.append_data(&mut header, path, content.as_bytes())?;

        // add the files from the specifier into the archive
        if self.path_specifier.is_some() {
            builder.append_path_specifier(self.path_specifier.as_ref().unwrap())?;
        }

        Ok(builder.into_inner()?)
    }
}

/// A container struct for a [Configs](struct.Configs.html) and the archive which describes it.
pub struct ConfigArchive {
    cfg: Configs,
    archive: Archive<File>,
}

impl ConfigArchive {
    /// Parse the archive's '.rconf' file as a [Configs](struct.Configs.html).
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error reading from
    /// and parsing the configuration file.
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

    /// Install all archived files to their intended locations on the file system.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error reading from
    /// the archive or unpacking a contained file to the specified location.
    fn install_configs(&mut self) -> Result<(), ConfigError> {
        let entries = self.archive.entries()?;

        for entry in entries {
            let mut entry = entry?;

            // extract the path from the archive entry
            let path = entry.path()?;
            let path = match ArchivePath::from_tar_path(path.as_ref()) {
                None => continue,
                Some(p) => p
            };

            // determine the target destination for the archive entry
            let dst = match path.kind {
                PathKind::ABSOLUTE => {
                    Path::new(&std::path::MAIN_SEPARATOR.to_string()).to_path_buf()
                }
                PathKind::HOME => try_dir!(dirs::home_dir, PathKind::HOME),
                PathKind::CONFIG => try_dir!(dirs::config_dir, PathKind::CONFIG),
            };

            // catch errors unpacking the archive files
            entry.unpack_in(&dst)?;
        }

        Ok(())
    }

    /// Construct a new [ConfigArchive](struct.ConfigArchive.html) from a tar archive.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error reading from the archive, or
    /// parsing the contained archived file.
    pub fn with_archive<P: AsRef<Path>>(path: P) -> Result<ConfigArchive, ConfigError> {
        let cfg = ConfigArchive::retrieve_configs(&path)?;
        let file = File::open(path)?;
        let archive = Archive::new(file);

        Ok(ConfigArchive { cfg, archive })
    }

    /// Install the configurations stored in the archive.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error installing the
    /// archived configurations.
    pub fn install(&mut self) -> Result<(), ConfigError> {
        self.install_configs()?;

        Ok(())
    }
}

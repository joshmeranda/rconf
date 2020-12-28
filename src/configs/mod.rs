#[macro_use]
pub mod path;
pub mod error;
pub mod manager;

use self::error::{ConfigError, Result};
use self::manager::*;
use self::path::*;
use super::script::build_script;
use serde_derive::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use tar::{Archive, Builder, Header};

/// Simple macro for generating a header for project files to be including in the configuration tar.
macro_rules! basic_header {
    ($data: expr) => {{
        let mut header = Header::new_gnu();
        header.set_size($data.len() as u64);
        header.set_mode(420); // 644 (rw- r-- r--)
        header.set_cksum();

        header
    }};
}

/// A container struct for a [ConfigArchive](struct.ConfigArchive.html) and the archive which describes it.
#[derive(Deserialize, Serialize)]
pub struct ConfigArchive {
    pub paths: Option<PathSpecifier>,

    pub manager: Option<Manager>,

    #[serde(skip)]
    archive: Option<Archive<File>>,
}

impl ConfigArchive {
    /// Parse the archive's '.rconf' file as a [ConfigArchive](struct.ConfigArchive.html).
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error reading from
    /// and parsing the configuration file.
    fn retrieve_configs<P: AsRef<Path>>(path: P) -> Result<ConfigArchive> {
        // read archive
        let file = File::open(path)?;
        let mut archive = Archive::new(file);
        let entries = archive.entries()?;
        let mut cfg: Option<ConfigArchive> = None;

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
    fn install_configs(&mut self) -> Result<()> {
        if let Some(archive) = &mut self.archive {
            let entries = archive.entries()?;

            for entry in entries {
                let mut entry = entry?;

                // extract the path from the archive entry
                let path = entry.path()?;
                let path = match ArchivePath::from_tar_path(path.as_ref()) {
                    None => continue,
                    Some(p) => p,
                };

                // retrieve the path's local location
                let dst = path.to_local_path()?;

                entry.unpack(dst)?;
            }

            Ok(())
        } else {
            Ok(())
        }
    }

    /// Uninstall and remove all specified configuration files.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned if a config file /
    /// directory could not be remove or found.
    fn uninstall_configs(&mut self) -> Result<()> {
        if let Some(archive) = &mut self.archive {
            let entries = archive.entries()?;

            for entry in entries {
                let entry = entry?;

                let path = entry.path()?;
                let path = match ArchivePath::from_tar_path(path.as_ref()) {
                    None => continue,
                    Some(p) => p,
                };

                let target = path.to_local_path()?;

                match if target.is_file() {
                    fs::remove_file(target)
                } else if target.is_dir() {
                    fs::remove_dir_all(target)
                } else {
                    Ok(())
                } {
                    Err(err) => return Err(ConfigError::from(err)),
                    Ok(_) => (),
                }
            }

            Ok(())
        } else {
            Ok(())
        }
    }

    /// Construct a new [ConfigArchive](struct.ConfigArchive.html) from a tar archive.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error reading from the archive, or
    /// parsing the contained archived file.
    pub fn with_archive<P: AsRef<Path>>(path: P) -> Result<ConfigArchive> {
        let cfg = ConfigArchive::retrieve_configs(&path)?;
        let file = File::open(path)?;
        let archive = Some(Archive::new(file));

        Ok(ConfigArchive { archive, ..cfg })
    }

    /// Create a new ConfigArchive instantiation from specified configuration file.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error reading from
    /// the specified file, or parsing the contents.
    pub fn with_file(path: &Path) -> Result<ConfigArchive> {
        let contents = fs::read_to_string(path)?;
        let cfg: ConfigArchive = toml::from_str(&contents[..])?;

        Ok(cfg)
    }

    /// Package configuration files into a tar archive and write to the system. See
    /// [append_path_specifier]
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error creating the
    /// archive, or adding files and their contents to it.
    pub fn write_archive(&self, path: &Path) -> Result<File> {
        let file = File::create(path)?;
        let mut builder = Builder::new(file);

        // generate content and header for rconf file
        let content = toml::to_string_pretty(self).unwrap();
        let script = build_script(self);

        builder.append_data(
            &mut basic_header!(content),
            Path::new(".rconf"),
            content.as_bytes(),
        )?;
        builder.append_data(
            &mut basic_header!(script),
            Path::new("install.sh"),
            script.as_bytes(),
        )?;

        // add the files from the specifier into the archive
        if self.paths.is_some() {
            builder.append_path_specifier(self.paths.as_ref().unwrap())?;
        }

        Ok(builder.into_inner()?)
    }

    /// Install the configurations stored in the archive.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error installing the
    /// archived configurations.
    pub fn install(&mut self) -> Result<()> {
        if let Some(manager) = &self.manager {
            let status = manager.install_packages();

            if !status.success() {
                return Err(ConfigError::Manager(
                    manager.name.clone(),
                    manager.install_args.clone(),
                ));
            }
        }

        self.install_configs()?;

        Ok(())
    }

    /// Uninstall the archive configurations.
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) will be returned on an error uninstalling
    /// the archived configurations.
    pub fn uninstall(&mut self) -> Result<()> {
        if let Some(manager) = &self.manager {
            if let Err(err) = manager.un_install_packages() {
                return Err(err);
            }
        }

        self.uninstall_configs()?;

        Ok(())
    }
}

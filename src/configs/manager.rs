use crate::configs::error::{ConfigError, Result};
use std::process::{Command, ExitStatus};

/// Describes package manager commands to install and uninstall packages as well as upgrade the
/// system.
#[derive(Deserialize, Serialize)]
pub struct Manager {
    /// The name of the package manager (pacman, yum, apt, etc)
    name: String,
    packages: Vec<String>,
    install_args: Vec<String>,
    un_install_args: Option<Vec<String>>,
    upgrade_args: Option<Vec<String>>,
}

impl Manager {
    /// Install the packages specified using the specified package manager.
    pub fn install_packages(&self) -> ExitStatus {
        Command::new(&self.name)
            .args(&self.install_args)
            .args(&self.packages)
            .spawn()
            .expect("Could not run the package manager with the given args")
            .wait()
            .expect("Issue waiting for the child installing process")
    }

    /// Uninstall the packages specified using the  specified package manager.
    pub fn un_install_packages(&self) -> Result<ExitStatus> {
        if let Some(args) = &self.un_install_args {
            Ok(Command::new(&self.name)
                .args(args)
                .args(&self.packages)
                .spawn()
                .expect("Could not run the package manager with the given args")
                .wait()
                .expect("Issue waiting for the child installing process"))
        } else {
            Err(ConfigError::FieldNotFound("un_install_args".to_string()))
        }
    }

    /// Upgrade the current machine, it is suggested that the user reboots their computer after this
    /// is executed but it is not enforced.
    pub fn system_upgrade(&self) -> Result<ExitStatus> {
        if let Some(args) = &self.upgrade_args {
            Ok(Command::new(&self.name)
                .args(args)
                .spawn()
                .expect("Could not run the package manager with the given args")
                .wait()
                .expect("Issue waiting for the child installing process"))
        } else {
            Err(ConfigError::FieldNotFound("upgrade_args".to_string()))
        }
    }
}

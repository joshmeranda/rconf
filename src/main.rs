//! Command line tool intended to ease the burden of a system setup and configuration and allow
//! users to hit the ground running.
mod configs;
mod script;

#[macro_use]
extern crate serde_derive;
extern crate toml;

use clap::{crate_version, App, AppSettings, Arg, ArgMatches, SubCommand};
use configs::{error::ConfigError, ConfigArchive};
use std::path::{Path, PathBuf};

/// Create a tar archive of existing system config files specified in the given toml file. Defaults
/// to a '.rconf' file in the home directory.
fn archive(archive_matches: &ArgMatches) -> Result<(), ConfigError> {
    // determine the path to the configuration file
    let mut path = PathBuf::new();

    match archive_matches.value_of("config_file") {
        Some(p) => path.push(p),
        // add the default configuration path
        None => match dirs::config_dir() {
            Some(p) => {
                path.push(p);
                path.push(".rconf");
            }
            None => {
                eprintln!("Could not determine default configuration directory, and no config file was given.");
                std::process::exit(1);
            }
        },
    };

    // print error message and exit
    let cfg = ConfigArchive::with_file(&path)?;

    // determine the destination path
    let mut path = PathBuf::new();

    // add the parent directory
    match archive_matches.value_of("destination") {
        Some(dst) => path.push(dst),
        None => path = std::env::current_dir()?,
    }

    // add the '.tar' extension if necessary to the given archive name
    let mut title = String::from(match archive_matches.value_of("title") {
        Some(title) => title,
        None => "rconf.tar",
    });

    if !title.ends_with(".tar") {
        title.push_str(".tar");
    }

    // add tile to the given path
    path.push(title);

    cfg.write_archive(path.as_path())?;

    Ok(())
}

fn install(install_matches: &ArgMatches) -> Result<(), ConfigError> {
    let tar_path = Path::new(install_matches.value_of("archive").unwrap());
    let mut archive_cfg = ConfigArchive::with_archive(tar_path)?;

    if install_matches.is_present("upgrade") {
        let is_upgraded: Result<(), ConfigError> = match &archive_cfg.manager {
            Some(manager) => {
                if let Err(err) = manager.system_upgrade() {
                    Err(err)
                } else {
                    Ok(())
                }
            }
            None => Err(ConfigError::FieldNotFound(String::from("manager"))),
        };

        if let Err(err) = is_upgraded {
            return Err(err);
        }
    }

    archive_cfg.install()
}

fn remove(remove_matches: &ArgMatches) -> Result<(), ConfigError> {
    let tar_path = Path::new(remove_matches.value_of("archive").unwrap());
    let mut archive_cfg = ConfigArchive::with_archive(tar_path)?;

    archive_cfg.uninstall()
}

fn main() -> Result<(), ConfigError> {
    let matches = App::new("rconf")
        .about("backup and deploy configuration files.")
        .version(crate_version!())
        // create an archive according to specifications contained in a file
        .subcommand(SubCommand::with_name("archive")
            .about("create an archive as specified by the config file.")
            .arg(Arg::with_name("config_file")
                .short("f")
                .long("file")
                .value_name("FILE")
                .help("the file to utilize for creating the archive"))
            .arg(Arg::with_name("destination")
                .short("d")
                .long("dest")
                .value_name("DIR")
                .help("the parent directory in which to store the resulting archive (defaults to the current working directory)"))
            .arg(Arg::with_name("title")
                 // .hidden(true)
                .required(true)
                .value_name("TITLE")
                .help("the name of the resulting archive, if the .tar extension is missig it will be added"))
                .setting(AppSettings::ArgRequiredElseHelp))
        // install system configurations and packages
        .subcommand(SubCommand::with_name("install")
            .about("attempt to install configurations from a given archive")
            .arg(Arg::with_name("archive")
                .hidden(true)
                .required(true)
                .value_name("ARCHIVE")
                .help("the path to the archive to be unpacked"))
            .arg(Arg::with_name("upgrade")
                .long("upgrade")
                .takes_value(false)
                .help("if available upgrade the system using the package manger before installing"))
                .setting(AppSettings::ArgRequiredElseHelp))
        // uninstall system configurations and packages
        .subcommand(SubCommand::with_name("remove")
            .about("attempt to uninstall configurations from a given archive")
            .arg(Arg::with_name("archive")
                .hidden(true)
                .required(true)
                .value_name("ARCHIVE")
                .help("the path to the archive to be unpacked"))
            .setting(AppSettings::ArgRequiredElseHelp))
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .get_matches();

    let result = match matches.subcommand_name() {
        Some("install") => install(matches.subcommand_matches("install").unwrap()),
        Some("archive") => archive(matches.subcommand_matches("archive").unwrap()),
        Some("remove") => remove(matches.subcommand_matches("remove").unwrap()),
        _ => Ok(()), // unrecognized SubCommand handled ^^^ by get_matches
    };

    // nicely print any errors to the console
    match result {
        Ok(_) => Ok(()),
        Err(err) => {
            eprintln!("{}", err.to_string());
            std::process::exit(1);
        }
    }
}

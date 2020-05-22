use self::super::error::ConfigError;
use std::fs::File;
use std::path::{self, Path, PathBuf};
use tar::Builder;

macro_rules! archive_path_vec {
    ($property:expr, $kind:expr) => {{
        match $property {
            None => vec![],
            Some(v) => v
                .iter()
                .map(|path| -> ArchivePath {
                    ArchivePath {
                        kind: $kind,
                        path: Path::new(path),
                    }
                })
                .collect(),
        }
    }};
}

#[macro_export]
macro_rules! try_dir {
    ($dir_fn:expr, $kind:expr) => {
        match $dir_fn() {
            Some(dir) => dir,
            None => {
                return Err(ConfigError::DirNotFound(match $kind {
                    PathKind::ABSOLUTE => "absolute".to_string(),
                    PathKind::HOME => "Home".to_string(),
                    PathKind::CONFIG => "Config".to_string(),
                }));
            }
        }
    };
}

/// Custom trait allowing for appending a [PathSpecifier](struct.PathSpecifier.html) to the type.
pub trait AppendSpecifier {
    fn append_path_specifier(&mut self, specifier: &PathSpecifier) -> Result<(), ConfigError>;
}

/// Extension for [Builder](../../../tar/builder/struct.Builder.html) allowing for adding all paths
/// in a [PathSpecifier](struct.PathSpecifier.html) to be appended.
///
/// This implementation handles appending both files and directories.
impl AppendSpecifier for Builder<File> {
    /// Append the configuration files specified by the [PathSpecifier](struct.PathSpecifier.html)
    ///
    /// All absolute paths are stored with their root at the archive root (ex /etc/gitconfig =>
    /// archive.tar/etc/gitconfig). System dependent config locations will be stored in a
    /// representative top level directory in the archive (ex $HOME/.basrhc => archive.tar/home).
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) is returned on an error adding a file from
    /// the specifier into the builder.
    fn append_path_specifier(&mut self, specifier: &PathSpecifier) -> Result<(), ConfigError> {
        // retrieve vector of archive paths for all config paths
        let mut all_paths = specifier.get_archiveable_paths(PathKind::ABSOLUTE);
        all_paths.append(&mut specifier.get_archiveable_paths(PathKind::HOME));
        all_paths.append(&mut specifier.get_archiveable_paths(PathKind::CONFIG));

        // add all paths to the archive builder
        for path in all_paths {
            let path_buf: PathBuf = path.to_local_path()?;

            if path_buf.is_file() {
                self.append_path_with_name(path_buf, path.to_tar_path())?
            } else if path_buf.is_dir() {
                self.append_dir_all(path.to_tar_path(), path_buf)?
            }
        }

        Ok(())
    }
}

/// Used to specify the type of path when retrieving the vectors from
/// [ConfigPathSpecifier](struct.ConfigPathSpecifier.html).
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum PathKind {
    ABSOLUTE,
    HOME,
    CONFIG,
}

impl<P> From<P> for PathKind
where
    P: AsRef<Path>,
{
    /// Determine the path type from the path in relation to the archive.
    fn from(path: P) -> PathKind {
        let path = path.as_ref();

        if path.is_absolute() {
            PathKind::ABSOLUTE
        } else if path.starts_with("home") {
            PathKind::HOME
        } else if path.starts_with("config") {
            PathKind::CONFIG
        } else {
            PathKind::ABSOLUTE
        }
    }
}

/// Intermediate type for adding the paths of a
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ArchivePath<'a> {
    pub kind: PathKind,
    pub path: &'a Path,
}

impl<'a> ArchivePath<'a> {
    /// Construct an [ArchivePath](struct.ArchivePath.html) from a file's relative path inside a
    /// config archive.
    pub fn from_tar_path(path: &'a Path) -> Option<ArchivePath<'a>> {
        if path.to_str().unwrap() == ".rconf" {
            None
        } else if path.starts_with("home") {
            Some(ArchivePath {
                kind: PathKind::HOME,
                path: path.strip_prefix("home").unwrap(),
            })
        } else if path.starts_with("config") {
            Some(ArchivePath {
                kind: PathKind::CONFIG,
                path: path.strip_prefix("config").unwrap(),
            })
        } else if path.is_relative() {
            // absolute paths are stored in a relative path of the same name without the leading '/'
            Some(ArchivePath {
                kind: PathKind::ABSOLUTE,
                path,
            })
        } else {
            None
        }
    }

    /// Retrieve the relative path for a config file inside an archive.
    pub fn to_tar_path(&self) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(match self.kind {
            PathKind::ABSOLUTE => "",
            PathKind::HOME => "home",
            PathKind::CONFIG => "config",
        });

        path.push(match self.kind {
            PathKind::ABSOLUTE => match self.path.strip_prefix("/") {
                Ok(p) => return p.to_path_buf(),
                Err(_) => return self.path.to_path_buf(),
            },
            _ => self.path,
        });

        path
    }

    /// Retrieve the path on the local system corresponding to the
    /// [ArchivePath](struct.ArchivePath.html).
    ///
    /// # Errors
    /// A [ConfigError](../error/enum.ConfigError.html) on an error determining a system directory
    /// such as the home or config directories.
    pub fn to_local_path(&self) -> Result<PathBuf, ConfigError> {
        let mut buf = PathBuf::new();

        match &self.kind {
            PathKind::ABSOLUTE => buf.push(path::MAIN_SEPARATOR.to_string()),
            PathKind::HOME => buf.push(try_dir!(dirs::home_dir, PathKind::HOME)),
            PathKind::CONFIG => buf.push(try_dir!(dirs::config_dir, PathKind::CONFIG)),
        };

        buf.push(self.path);

        Ok(buf)
    }
}

/// Container for all configuration files specified in the configuration.
#[derive(Deserialize, Serialize)]
#[serde(rename(deserialize = ""))]
pub struct PathSpecifier {
    pub absolute: Option<Vec<String>>,
    pub home: Option<Vec<String>>,
    pub config: Option<Vec<String>>,
}

impl PathSpecifier {
    /// Retrieve a vector of paths as [ArchivePath](struct.ArchivePath.html) which can be easier
    /// stored in an archive.
    fn get_archiveable_paths(&self, kind: PathKind) -> Vec<ArchivePath> {
        match kind {
            PathKind::ABSOLUTE => archive_path_vec!(&self.absolute, PathKind::ABSOLUTE),
            PathKind::HOME => archive_path_vec!(&self.home, PathKind::HOME),
            PathKind::CONFIG => archive_path_vec!(&self.config, PathKind::CONFIG),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ArchivePath, PathKind};
    use crate::configs::path::PathSpecifier;
    use std::path::Path;

    #[test]
    fn test_path_kind() {
        assert_eq!(PathKind::ABSOLUTE, PathKind::from("/etc/rconf"));
        assert_eq!(PathKind::HOME, PathKind::from("home/rconf"));
        assert_eq!(PathKind::CONFIG, PathKind::from("config/rconf"));
    }

    #[test]
    fn test_from_tar_path_skip_rconf() {
        assert!(ArchivePath::from_tar_path(Path::new(".rconf")).is_none());
    }

    #[test]
    fn test_from_tar_path_absolute() {
        let home = ArchivePath::from_tar_path(Path::new("etc/rconf"));
        assert_eq!(
            ArchivePath {
                kind: PathKind::ABSOLUTE,
                path: Path::new("etc/rconf")
            },
            home.unwrap()
        );
    }

    #[test]
    fn test_from_tar_path_home() {
        let home = ArchivePath::from_tar_path(Path::new("home/rconf"));
        assert_eq!(
            ArchivePath {
                kind: PathKind::HOME,
                path: Path::new("rconf")
            },
            home.unwrap()
        );
    }

    #[test]
    fn test_from_tar_path_config() {
        let config = ArchivePath::from_tar_path(Path::new("config/rconf"));
        assert_eq!(
            ArchivePath {
                kind: PathKind::CONFIG,
                path: Path::new("rconf")
            },
            config.unwrap()
        );
    }

    #[test]
    fn test_to_tar_path_absolute() {
        let absolute = ArchivePath {
            kind: PathKind::ABSOLUTE,
            path: Path::new("etc/rconf"),
        };

        assert_eq!(Path::new("etc/rconf"), absolute.to_tar_path());
    }

    #[test]
    fn test_to_tar_path_home() {
        let home = ArchivePath {
            kind: PathKind::HOME,
            path: Path::new("rconf"),
        };

        assert_eq!(Path::new("home/rconf"), home.to_tar_path());
    }

    #[test]
    fn test_to_tar_path_config() {
        let config = ArchivePath {
            kind: PathKind::CONFIG,
            path: Path::new("rconf"),
        };

        assert_eq!(Path::new("config/rconf"), config.to_tar_path());
    }

    #[test]
    fn test_archiveable_paths() {
        let specifier = PathSpecifier {
            absolute: Some(vec!["/etc/rconf".to_string()]),
            home: Some(vec!["rconf".to_string()]),
            config: Some(vec!["rconf".to_string()]),
        };

        let expected_absolute = vec![ArchivePath {
            kind: PathKind::ABSOLUTE,
            path: Path::new("/etc/rconf"),
        }];
        let expected_home = vec![ArchivePath {
            kind: PathKind::HOME,
            path: Path::new("rconf"),
        }];
        let expected_config = vec![ArchivePath {
            kind: PathKind::CONFIG,
            path: Path::new("rconf"),
        }];

        assert_eq!(
            expected_absolute,
            specifier.get_archiveable_paths(PathKind::ABSOLUTE)
        );
        assert_eq!(
            expected_home,
            specifier.get_archiveable_paths(PathKind::HOME)
        );
        assert_eq!(
            expected_config,
            specifier.get_archiveable_paths(PathKind::CONFIG)
        );
    }

    #[test]
    fn test_empty_archiveable_paths() {
        let specifier = PathSpecifier {
            absolute: None,
            home: None,
            config: None,
        };

        assert!(specifier
            .get_archiveable_paths(PathKind::ABSOLUTE)
            .is_empty());
        assert!(specifier.get_archiveable_paths(PathKind::HOME).is_empty());
        assert!(specifier.get_archiveable_paths(PathKind::CONFIG).is_empty());
    }
}

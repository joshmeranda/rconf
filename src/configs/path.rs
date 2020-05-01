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
#[derive(Copy, Clone)]
pub enum PathKind {
    ABSOLUTE,
    HOME,
    CONFIG,
}

impl From<&Path> for PathKind {
    /// Determine the path type from the path in relation to the archive.
    fn from(path: &Path) -> PathKind {
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
#[derive(Copy, Clone)]
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
        } else if path.is_absolute() {
            Some(ArchivePath {
                kind: PathKind::ABSOLUTE,
                path,
            })
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
        } else {
            Some(ArchivePath {
                kind: PathKind::ABSOLUTE,
                path,
            })
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

        return path;
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
    absolute: Option<Vec<String>>,
    home: Option<Vec<String>>,
    config: Option<Vec<String>>,
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

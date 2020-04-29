use std::path::{self, Path, PathBuf};
use std::io::{Result as ioResult};
use std::fs::{self, File};
use tar::Builder;

macro_rules! path_vec {
    ($property:expr) => {{
        match $property {
            None => vec![],
            Some(v) => v.iter().map(|str| Path::new(str) ).collect(),
        }
    }};
}

macro_rules! archive_path_vec {
    ($property:expr, $kind:expr) => {{
        match $property {
            None => vec![],
            Some(v) => v.iter().map(|path| -> ArchivePath { ArchivePath { kind: $kind, path: Path::new(path) }}).collect()
        }
    }}
}

/// Custom trait allowing for appending a [PathSpecifier](struct.PathSpecifier.html) to the type.
pub trait AppendSpecifier {
    fn append_path_specifier(&mut self, specifier: &PathSpecifier) -> ioResult<()>;
}

/// Extension for [Builder](../../../tar/builder/struct.Builder.html) allowing for adding all paths
/// in a [PathSpecifier](struct.PathSpecifier.html) to be appended.
///
/// This implementation handles appending both files and directories.
impl AppendSpecifier for Builder<File> {
    fn append_path_specifier(&mut self, specifier: &PathSpecifier) -> ioResult<()> {
        // retrieve vector of archive paths for all config paths
        let mut all_paths = specifier.get_archiveable_paths(PathKind::ABSOLUTE);
        all_paths.append(&mut specifier.get_archiveable_paths(PathKind::HOME));
        all_paths.append(&mut specifier.get_archiveable_paths(PathKind::CONFIG));

        // add all paths to the archive builder
        for path in all_paths {
            let path_buf: PathBuf = path.into();
            println!("{} => {}", path.to_tar_path().to_str().unwrap(), path_buf.to_str().unwrap());

            if path_buf.is_file() {
                self.append_path_with_name(path_buf, path.to_tar_path())?
            } else if path_buf.is_dir() {
                self.append_dir_all(path.to_tar_path(), path_buf)?
            }
        }

        Ok(())
    }
}

/// Used to specify the type of path when retrieving the vectors from [ConfigPathSpecifier](struct.ConfigPathSpecifier.html).
#[derive(Copy, Clone)]
pub enum PathKind {
    ABSOLUTE,
    HOME,
    CONFIG
}

/// Intermediate type for adding the paths of a
#[derive(Copy, Clone)]
pub struct ArchivePath<'a> {
    pub kind: PathKind,
    pub path: &'a Path
}

impl ArchivePath<'_> {
    /// Retrieve path for the files inside the tar archive.
    pub fn to_tar_path(&self) -> PathBuf {
        let mut path = PathBuf::new();

        path.push(match self.kind {
            PathKind::ABSOLUTE => "",
            PathKind::HOME => "home",
            PathKind::CONFIG => "config",
        });

        path.push(match self.kind {
            PathKind::ABSOLUTE => {
                match self.path.strip_prefix("/") {
                    Ok(p) => return p.to_path_buf(),
                    Err(_) => return self.path.to_path_buf()
                }
            },
            _ => self.path
        });


        return path;
    }
}

impl Into<PathBuf> for ArchivePath<'_> {
    /// Converts an [ArchivePath](struct.ArchivePath.html) into the (PathBuf) which represents the
    /// file's location on the local system.
    fn into(self) -> PathBuf {
        let mut path = match &self.kind {
            PathKind::ABSOLUTE => self.path.to_path_buf(),
            PathKind::HOME => dirs::home_dir().unwrap(),
            PathKind::CONFIG => dirs::config_dir().unwrap()
        };

        path.push(self.path);

        path.to_path_buf()
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
    /// Retrieve vector of paths as specified by [PathKind](enum.PathKind.html).
    ///
    /// It should be noted that no error will be raised if an absolute path leads too the current
    /// users home or config directories; however, it would make far more sense to list those paths
    /// in the appropriate field.
    fn get_paths(&self, kind: PathKind) -> Vec<&Path> {
        match kind {
            PathKind::ABSOLUTE => path_vec!(&self.absolute),
            PathKind::HOME => path_vec!(&self.home),
            PathKind::CONFIG => path_vec!(&self.config)
        }
    }

    /// Retrieve a vector of paths as [ArchivePath](struct.ArchivePath.html) which can be easier
    /// stored in an archive.
    fn get_archiveable_paths(&self, kind: PathKind) -> Vec<ArchivePath> {
        match kind {
            PathKind::ABSOLUTE => archive_path_vec!(&self.absolute, PathKind::ABSOLUTE),
            PathKind::HOME => archive_path_vec!(&self.home, PathKind::HOME),
            PathKind::CONFIG=> archive_path_vec!(&self.config, PathKind::CONFIG)
        }
    }

    pub fn install_paths(&self) -> Result<(), crate::configs::error::ConfigError> {
        let mut all_paths = self.get_archiveable_paths(PathKind::ABSOLUTE);
        all_paths.append(self.get_archiveable_paths(PathKind::HOME).as_mut());
        all_paths.append(self.get_archiveable_paths(PathKind::CONFIG).as_mut());

        for path in all_paths {
            let archived_path: PathBuf = path.into();
            let mut new_path = PathBuf::new();

            // build the new path
            match path.kind {
                PathKind::ABSOLUTE => new_path.push(path::MAIN_SEPARATOR.to_string()),
                PathKind::HOME => new_path.push(dirs::home_dir().unwrap()),
                PathKind::CONFIG => new_path.push(dirs::config_dir().unwrap())
            };

            fs::rename(archived_path, new_path)?;
        }

        Ok(())
    }
}
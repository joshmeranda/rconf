use std::path::{Path, PathBuf};
use std::io::{Result as ioResult};
use std::fs::File;
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

            if path_buf.is_file() {
                self.append_path_with_name(path_buf, path.path)?
            } else if path_buf.is_dir() {
                self.append_dir_all(path.path, path_buf)?
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
    kind: PathKind,
    pub path: &'a Path
}

impl Into<PathBuf> for ArchivePath<'_> {
    fn into(self) -> PathBuf {
        let mut path= match &self.kind {
            PathKind::ABSOLUTE => PathBuf::new(),
            PathKind::HOME => dirs::home_dir().unwrap(),
            PathKind::CONFIG => dirs::config_dir().unwrap()
        };

        path.push(self.path);

        path
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
    pub fn get_paths(&self, kind: PathKind) -> Vec<&Path> {
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
}
use crate::prelude::IoResult;
use std::fs;
use std::fs::Metadata;
use std::path::Path;

pub trait PathExt {
    /// Returns the nearest existing ancestor of the path.
    fn find_nearest_existing_ancestor(&self) -> Option<&Path>;

    fn get_kind(&self) -> IoResult<FileKind>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    /// A regular file.
    File,
    /// A directory.
    Directory,
    /// A symbolic link, hard link, junction, NTFS mount, etc.
    Link,
    /// Some unrecognized kind of file.
    Other,
}

impl PathExt for Path {
    fn find_nearest_existing_ancestor(&self) -> Option<&Path> {
        let mut path = self;
        while !path.exists() {
            path = match path.parent() {
                Some(p) => p,
                None => return None,
            };
        }
        Some(path)
    }

    fn get_kind(&self) -> IoResult<FileKind> {
        let metadata = fs::symlink_metadata(self)?;
        Ok(metadata.into())
    }
}

impl From<Metadata> for FileKind {
    fn from(metadata: Metadata) -> Self {
        Self::from(&metadata)
    }
}

impl From<&Metadata> for FileKind {
    fn from(metadata: &Metadata) -> Self {
        if metadata.is_file() {
            FileKind::File
        } else if metadata.is_dir() {
            FileKind::Directory
        } else if metadata.file_type().is_symlink() {
            FileKind::Link
        } else {
            FileKind::Other
        }
    }
}

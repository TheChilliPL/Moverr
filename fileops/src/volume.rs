//! Volume operations. Currently only supports Windows.

use crate::prelude::*;
use crate::PathExt;
use flagset::{flags, FlagSet};
use log_error::LogError;
use std::borrow::Cow;
use std::ffi::OsString;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{absolute, Path};
use widestring::{U16CStr, U16CString, Utf16String};
use windows::core::PCWSTR;
use windows::Win32::Foundation::MAX_PATH;
use windows::Win32::Storage::FileSystem::GetVolumeInformationW;

/// Information about a volume.
///
/// See [GetVolumeInformationW](https://docs.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getvolumeinformationw)
#[derive(Debug, Clone)]
pub struct VolumeInformation {
    pub name: String,
    pub serial_number: u32,
    pub max_component_length: u32,
    pub fs_flags: FlagSet<FsFlag>,
    pub fs_name: String,
}

flags! {
    /// Bit flags for the `fs_flags` field of `VolumeInformation`.
    ///
    /// The enum represents single flags, use [FlagSet] for multiple flags.
    pub enum FsFlag: u32 {
        None = 0x00000000,
        FileCaseSensitiveSearch = 0x00000001,
        FileCasePreservedNames = 0x00000002,
        FileUnicodeOnDisk = 0x00000004,
        FilePersistentAcls = 0x00000008,
        FileFileCompression = 0x00000010,
        FileVolumeQuotas = 0x00000020,
        FileSupportsSparseFiles = 0x00000040,
        FileSupportsReparsePoints = 0x00000080,
        FileSupportsRemoteStorage = 0x00000100,
        FileReturnsCleanupResultInfo = 0x00000200,
        FileSupportsPosixUnlinkRename = 0x00000400,
        FileVolumeIsCompressed = 0x00000800,
        FileSupportsObjectIds = 0x00001000,
        FileSupportsEncryption = 0x00002000,
        FileNamedStreams = 0x00004000,
        FileReadOnlyVolume = 0x00008000,
        FileSequentialWriteOnce = 0x00010000,
        FileSupportsTransactions = 0x00020000,
        FileSupportsHardLinks = 0x00040000,
        FileSupportsExtendedAttributes = 0x00080000,
        FileSupportsOpenByFileId = 0x00100000,
        FileSupportsUsnJournal = 0x00200000,
        FileSupportsIntegrityStreams = 0x00400000,
        FileSupportsBlockRefcounting = 0x00800000,
        FileSupportsSparseVdl = 0x01000000,
        FileDaxVolume = 0x02000000,
        FileSupportsGhosting = 0x04000000,
    }
}

impl VolumeInformation {
    /// Returns information about the volume of the specified root path.
    pub fn of_root(root: &Path) -> Result<VolumeInformation> {
        let path_utf16 = U16CString::from_str(root.to_str().ok_or(Error::InvalidPath)?)
            .map_err(|_| Error::InvalidPath)?;

        // Max path length + 1 for null terminator
        const BUFFER_SIZE: usize = MAX_PATH as usize + 1;

        let mut name_utf16 = [0u16; BUFFER_SIZE];
        let mut serial_number = 0u32;
        let mut max_component_length = 0u32;
        let mut flags_int = 0u32;
        let mut fs_name_utf16 = [0u16; BUFFER_SIZE];

        unsafe {
            GetVolumeInformationW(
                PCWSTR(path_utf16.as_ptr()),
                Some(&mut name_utf16),
                Some(&mut serial_number),
                Some(&mut max_component_length),
                Some(&mut flags_int),
                Some(&mut fs_name_utf16),
            )
        }
        .map_err(|e| Error::WinApi(e))?;

        let name = U16CStr::from_slice_truncate(&name_utf16)
            .map_err(|_| Error::InvalidPath)?
            .to_string()
            .map_err(|_| Error::InvalidPath)?;
        let fs_name = U16CStr::from_slice_truncate(&fs_name_utf16)
            .map_err(|_| Error::InvalidPath)?
            .to_string()
            .map_err(|_| Error::InvalidPath)?;
        let flags = FlagSet::new_truncated(flags_int);

        Ok(VolumeInformation {
            name,
            serial_number,
            max_component_length,
            fs_flags: flags,
            fs_name,
        })
    }

    /// Returns information about the volume that the path is on.
    pub fn of_path(path: &Path) -> Result<VolumeInformation> {
        let root = find_volume_root(path).ok_or(Error::NotFound)?;
        VolumeInformation::of_root(&root)
    }
}

/// Returns the root of the volume that the path is on.
/// Follows symlinks, mounts, etc.
pub fn find_volume_root(path: &Path) -> Option<Cow<Path>> {
    let mut path = Cow::Borrowed(find_nearest_anchor(path)?);

    if path.is_relative() {
        path = Cow::Owned(
            absolute(&path)
                .inspect_err(|e| warn!("Failed to get absolute path for {}: {}", path.display(), e))
                .ok()?,
        );
    }

    loop {
        let meta = path.symlink_metadata();
        if let Err(e) = meta {
            error!("Failed to get metadata for {}: {}", path.display(), e);
            return None;
        }
        let meta = meta.unwrap();
        if meta.file_type().is_symlink() {
            path = Cow::Owned(path.read_link().unwrap());
        } else {
            return Some(path);
        }
    }
}

/// Returns the nearest anchor of the path.
///
/// An anchor is a directory that requires special handling, such as a volume
/// root or a directory symlink.
///
/// Probably wouldn't work correctly on Unix systems due to mount points.
pub fn find_nearest_anchor(path: &Path) -> Option<&Path> {
    // We start at the nearest existing ancestor.
    let ancestor = path.find_nearest_existing_ancestor()?;

    // Then we iterate over the parents checking for anchors.
    let mut path = ancestor;
    loop {
        let meta = path.symlink_metadata();
        if let Err(e) = meta {
            error!("Failed to get metadata for {}: {}", path.display(), e);
            return None;
        }
        let meta = meta.unwrap();
        if meta.file_type().is_symlink() {
            return Some(path);
        }
        path = match path.parent() {
            Some(p) => p,
            None => return Some(path),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::MAIN_SEPARATOR_STR;
    use test_log::test;
    use uuid::Uuid;

    // We use MAIN_SEPARATOR(_STR) in order not to make assumptions about the platform.
    // Should return the root of the volume on Windows and the root on Unix.
    //
    // Tests should be compatible with both Windows and Unix once volume supports Unix.
    // Tests that are Windows-specific should be marked with #[cfg(windows)].

    #[test]
    fn test_find_nearest_anchor() {
        // Virtually no chance of this path existing
        let path = Path::new(MAIN_SEPARATOR_STR).join(Uuid::new_v4().to_string());

        let anchor = find_nearest_anchor(&path);

        assert!(anchor.is_some());
        assert_eq!(anchor.unwrap(), Path::new(MAIN_SEPARATOR_STR));
    }

    #[test]
    fn test_find_volume_root() {
        // Virtually no chance of this path existing
        let path = Path::new(MAIN_SEPARATOR_STR).join(Uuid::new_v4().to_string());

        let root_path = find_volume_root(&path);

        assert!(root_path.is_some());
        #[cfg(windows)]
        assert_eq!(root_path.unwrap(), Path::new(r"C:\"));
        #[cfg(unix)]
        assert_eq!(root_path.unwrap(), Path::new("/"));
    }

    #[test]
    fn test_get_volume_info() {
        let vol_info = VolumeInformation::of_root(Path::new("C:\\"));

        assert!(
            vol_info.is_ok(),
            "Failed to get volume information: {:?}",
            vol_info.unwrap_err()
        );

        let vol_info = vol_info.unwrap();

        debug!("Volume information: {:?}", vol_info);
    }
}

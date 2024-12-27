use crate::file_size::num_ext::AsBytes;
use crate::file_size::FileSize;
use crate::fraction::{Fraction, FromRatio};
use crate::sync::CancellationToken;
use crate::volume_information::VolumeInformation;
use futures_lite::StreamExt;
use log::error;
use smol::fs;
use std::sync::{Arc, Mutex};
use std::{borrow::Cow, io, os::windows::ffi::OsStrExt, path::Path};
use windows::{
    core::PCWSTR,
    Win32::{Foundation::MAX_PATH, Storage::FileSystem::GetVolumeInformationW},
};

pub trait PathExt {
    /// Find the nearest existing ancestor path.
    fn find_nearest_existing_ancestor(self: &Self) -> Option<&Path>;
    /// Find the nearest anchor path, which is the first existing ancestor that is either a symlink or a volume root.
    fn find_nearest_anchor(self: &Self) -> Option<Cow<Path>>;
    /// Find the real volume root path.
    fn find_volume_root(self: &Self) -> Option<Cow<Path>>;
    fn get_volume_information(&self) -> Result<VolumeInformation, Option<windows::core::Error>>;
    async fn calc_directory_stats(
        &self,
        cancellation_token: Option<&CancellationToken>,
    ) -> Result<DirectoryStats, DirectoryStatsError>;
    async fn copy_directory(
        &self,
        dest: &Path,
        progress: Option<Arc<Mutex<ProcessDirectoryProgress>>>,
        cancellation_token: Option<Arc<CancellationToken>>,
    ) -> Result<(), CopyDirectoryError>;
    async fn verify_copy(
        &self,
        dest: &Path,
        progress: Option<Arc<Mutex<ProcessDirectoryProgress>>>,
        cancellation_token: Option<Arc<CancellationToken>>,
    ) -> Result<(), VerifyDirectoryError>;
    async fn move_and_symlink(
        &self,
        dest: &Path,
        progress: Option<Arc<Mutex<MoveAndSymlinkProgress>>>,
        cancellation_token: Option<Arc<CancellationToken>>,
    ) -> Result<(), MoveAndSymlinkError>;
    async fn move_back(
        &self,
        dest: &Path,
        progress: Option<Arc<Mutex<MoveBackProgress>>>,
        cancellation_token: Option<Arc<CancellationToken>>,
    ) -> Result<(), MoveBackError>;
}

impl PathExt for Path {
    fn find_nearest_existing_ancestor(self: &Path) -> Option<&Path> {
        let mut path: &Path = self;
        loop {
            if path.exists() {
                return Some(path);
            }
            path = path.parent()?;
        }
    }

    fn find_nearest_anchor(self: &Path) -> Option<Cow<Path>> {
        let mut path = Cow::Borrowed(self.find_nearest_existing_ancestor()?);
        if path.is_relative() {
            path = Cow::Owned(path.canonicalize().ok()?);
        }

        loop {
            let metadata = path.symlink_metadata();
            if metadata.is_err() {
                panic!(
                    "Failed to get metadata for {}. {:?}",
                    path.display(),
                    metadata.unwrap_err()
                );
            }
            let metadata = metadata.unwrap();
            if metadata.file_type().is_symlink() {
                return Some(path);
            }
            {
                let parent = path.parent();
                if parent.is_none() {
                    return Some(path);
                }
                path = match path {
                    Cow::Borrowed(p) => Cow::Borrowed(p.parent()?),
                    Cow::Owned(p) => Cow::Owned(p.parent()?.to_path_buf()),
                }
            }
            // path = Cow::Borrowed(parent.unwrap());
        }
    }

    fn find_volume_root(self: &Self) -> Option<Cow<Path>> {
        let mut anchor: Cow<Path> = self.find_nearest_anchor()?;

        loop {
            let metadata = anchor.symlink_metadata().ok()?;
            if metadata.file_type().is_symlink() {
                let target = anchor.read_link().ok()?;
                anchor = Cow::Owned(target);
            } else {
                return Some(anchor);
            }
        }
    }

    fn get_volume_information(
        self: &Self,
    ) -> Result<VolumeInformation, Option<windows::core::Error>> {
        let path_utf16: Vec<u16> = self
            .find_volume_root()
            .ok_or(None)?
            .as_ref()
            .as_os_str()
            .encode_wide()
            .collect();
        const BUFFER_SIZE: usize = MAX_PATH as usize + 1;

        let mut volume_name_utf16 = [0u16; BUFFER_SIZE];
        let mut serial_number = 0u32;
        let mut max_component_length = 0u32;
        let mut flags = 0u32;
        let mut fs_type_utf16 = [0u16; BUFFER_SIZE];

        unsafe {
            GetVolumeInformationW(
                PCWSTR(path_utf16.as_ptr()),
                Some(&mut volume_name_utf16),
                Some(&mut serial_number),
                Some(&mut max_component_length),
                Some(&mut flags),
                Some(&mut fs_type_utf16),
            )
        }?;

        Ok(VolumeInformation {
            volume_name: String::from_utf16_lossy(&volume_name_utf16),
            volume_serial_number: serial_number,
            maximum_component_length: max_component_length,
            file_system_flags: flags,
            file_system_name: String::from_utf16_lossy(&fs_type_utf16),
        })
    }

    async fn calc_directory_stats(
        &self,
        cancellation_token: Option<&CancellationToken>,
    ) -> Result<DirectoryStats, DirectoryStatsError> {
        let mut stats = DirectoryStats::default();

        let mut children = async_fs::read_dir(self)
            .await
            .map_err(|e| DirectoryStatsError::Io(e.kind()))?;

        while let Some(child) = children
            .try_next()
            .await
            .map_err(|e| DirectoryStatsError::Io(e.kind()))?
        {
            if let Some(cancellation_token) = cancellation_token {
                if cancellation_token.is_cancelled() {
                    return Err(DirectoryStatsError::Cancelled);
                }
            }

            let metadata = async_fs::symlink_metadata(child.path())
                .await
                .map_err(|e| DirectoryStatsError::Io(e.kind()))?;
            if metadata.is_symlink() {
                stats.symlink_count += 1;
            } else if metadata.is_dir() {
                stats.subfolder_count += 1;
                let child_stats =
                    Box::pin(child.path().calc_directory_stats(cancellation_token)).await?;
                stats.subfolder_count += child_stats.subfolder_count;
                stats.file_count += child_stats.file_count;
                stats.symlink_count += child_stats.symlink_count;
                stats.size += child_stats.size;
            } else {
                stats.file_count += 1;
                stats.size += metadata.len().bytes();
            }
        }

        Ok(stats)
    }

    async fn copy_directory(
        &self,
        dest: &Path,
        progress: Option<Arc<Mutex<ProcessDirectoryProgress>>>,
        cancellation_token: Option<Arc<CancellationToken>>,
    ) -> Result<(), CopyDirectoryError> {
        if dest.exists() {
            if let Some(progress) = progress {
                progress.lock().unwrap().state = ProcessDirectoryState::Aborted;
            }

            return Err(CopyDirectoryError::DestinationExists);
        }

        async_fs::create_dir_all(dest)
            .await
            .map_err(|e| CopyDirectoryError::Io(e.kind()))?;

        /// Inner function that doesn't clean up the destination directory on error/cancellation
        ///
        /// `dest` must be an existing directory here.
        async fn _copy_directory(
            source: &Path,
            dest: &Path,
            progress: &Option<Arc<Mutex<ProcessDirectoryProgress>>>,
            cancellation_token: Option<Arc<CancellationToken>>,
        ) -> Result<(), CopyDirectoryError> {
            let mut children = async_fs::read_dir(source)
                .await
                .map_err(|e| CopyDirectoryError::Io(e.kind()))?;

            // Processing children consecutively, as file system operations aren't fully
            // parallelizable anyway
            while let Some(child) = children
                .try_next()
                .await
                .map_err(|e| CopyDirectoryError::Io(e.kind()))?
            {
                if let Some(cancellation_token) = cancellation_token.clone() {
                    if cancellation_token.is_cancelled() {
                        return Err(CopyDirectoryError::Cancelled);
                    }
                }

                let child_path = child.path();
                let child_dest = dest.join(child.file_name());

                let metadata = async_fs::symlink_metadata(&child_path)
                    .await
                    .map_err(|e| CopyDirectoryError::Io(e.kind()))?;
                if metadata.is_symlink() {
                    return Err(CopyDirectoryError::SymlinkEncountered);
                } else if metadata.is_dir() {
                    async_fs::create_dir(&child_dest)
                        .await
                        .map_err(|e| CopyDirectoryError::Io(e.kind()))?;
                    Box::pin(_copy_directory(
                        &child_path,
                        &child_dest,
                        &progress.clone(),
                        cancellation_token.clone(),
                    ))
                    .await?;
                } else {
                    async_fs::copy(&child_path, &child_dest)
                        .await
                        .map_err(|e| CopyDirectoryError::Io(e.kind()))?;

                    if let Some(progress) = progress.as_ref() {
                        progress
                            .lock()
                            .unwrap()
                            .process_file(metadata.len().bytes());
                    }
                }
            }

            Ok(())
        }

        let result = _copy_directory(self, dest, &progress, cancellation_token).await;

        if result.is_err() {
            // Error/cancellation occurred, clean up the destination directory
            // Ignore any errors that occur during cleanup
            let res = async_fs::remove_dir_all(dest).await;
            if res.is_err() {
                error!(target: "copy_directory", "Failed to clean up destination directory {:?}. {:?}", dest, res);
            }

            if let Some(progress) = progress {
                progress.lock().unwrap().state = ProcessDirectoryState::Aborted;
            }
        } else if let Some(progress) = progress {
            progress.lock().unwrap().state = ProcessDirectoryState::Finished;
        }

        result
    }

    async fn verify_copy(
        &self,
        dest: &Path,
        progress: Option<Arc<Mutex<ProcessDirectoryProgress>>>,
        cancellation_token: Option<Arc<CancellationToken>>,
    ) -> Result<(), VerifyDirectoryError> {
        let mut children = async_fs::read_dir(self)
            .await
            .map_err(|e| VerifyDirectoryError::Io(e.kind()))?;

        while let Some(child) = children
            .try_next()
            .await
            .map_err(|e| VerifyDirectoryError::Io(e.kind()))?
        {
            if let Some(cancellation_token) = cancellation_token.clone() {
                if cancellation_token.is_cancelled() {
                    return Err(VerifyDirectoryError::Cancelled);
                }
            }

            let child_path = child.path();
            let child_dest = dest.join(child.file_name());

            let source_metadata = async_fs::symlink_metadata(&child_path)
                .await
                .map_err(|e| VerifyDirectoryError::Io(e.kind()))?;
            let dest_metadata = async_fs::symlink_metadata(&child_dest)
                .await
                .map_err(|e| VerifyDirectoryError::Io(e.kind()))?;

            if source_metadata.is_symlink() {
                if !dest_metadata.is_symlink() {
                    return Err(VerifyDirectoryError::InvalidData);
                }
            } else if source_metadata.is_dir() {
                if !dest_metadata.is_dir() {
                    return Err(VerifyDirectoryError::InvalidData);
                }
                Box::pin(child_path.verify_copy(
                    &child_dest,
                    progress.clone(),
                    cancellation_token.clone(),
                ))
                .await?;
            } else {
                if !dest_metadata.is_file() {
                    return Err(VerifyDirectoryError::InvalidData);
                }
                if source_metadata.len() != dest_metadata.len() {
                    return Err(VerifyDirectoryError::InvalidData);
                }

                if let Some(progress) = progress.as_ref() {
                    progress
                        .lock()
                        .unwrap()
                        .process_file(source_metadata.len().bytes());
                }
            }
        }

        Ok(())
    }

    async fn move_and_symlink(
        &self,
        dest: &Path,
        progress: Option<Arc<Mutex<MoveAndSymlinkProgress>>>,
        cancellation_token: Option<Arc<CancellationToken>>,
    ) -> Result<(), MoveAndSymlinkError> {
        let inner_progress = if let Some(progress) = progress.as_ref() {
            let mut progress = progress.lock().unwrap();
            progress.stage = MoveAndSymlinkStage::Copying;
            Some(progress.progress.clone())
        } else {
            None
        };

        let copy_res = self
            .copy_directory(dest, inner_progress.clone(), cancellation_token.clone())
            .await;

        if let Err(err) = copy_res {
            return match err {
                CopyDirectoryError::DestinationExists => {
                    Err(MoveAndSymlinkError::DestinationExists)
                }
                CopyDirectoryError::SymlinkEncountered => {
                    Err(MoveAndSymlinkError::SymlinkEncountered)
                }
                CopyDirectoryError::Cancelled => Err(MoveAndSymlinkError::Cancelled),
                CopyDirectoryError::Io(e) => Err(MoveAndSymlinkError::Io(e)),
            };
        }

        if let Some(progress) = progress.as_ref() {
            inner_progress.as_ref().unwrap().lock().unwrap().zero();
            progress.lock().unwrap().stage = MoveAndSymlinkStage::Verifying;
        }

        let verify_res = self
            .verify_copy(dest, inner_progress.clone(), cancellation_token.clone())
            .await;

        if let Err(err) = verify_res {
            return match err {
                VerifyDirectoryError::Cancelled => Err(MoveAndSymlinkError::Cancelled),
                VerifyDirectoryError::InvalidData => Err(MoveAndSymlinkError::VerificationFailed),
                VerifyDirectoryError::Io(e) => Err(MoveAndSymlinkError::Io(e)),
            };
        }

        if let Some(progress) = progress.as_ref() {
            progress.lock().unwrap().stage = MoveAndSymlinkStage::Symlinking;
        }

        let remove_res = async_fs::remove_dir_all(self).await;

        if let Err(err) = remove_res {
            return Err(MoveAndSymlinkError::Io(err.kind()));
        }

        let symlink_res = async_fs::windows::symlink_dir(dest, self).await;

        if let Err(err) = symlink_res {
            return Err(MoveAndSymlinkError::Io(err.kind()));
        }

        if let Some(progress) = progress.as_ref() {
            progress.lock().unwrap().stage = MoveAndSymlinkStage::Finished;
        }

        Ok(())
    }

    async fn move_back(
        &self,
        dest: &Path,
        progress: Option<Arc<Mutex<MoveBackProgress>>>,
        cancellation_token: Option<Arc<CancellationToken>>,
    ) -> Result<(), MoveBackError> {
        let inner_progress = if let Some(progress) = progress.as_ref() {
            let mut progress = progress.lock().unwrap();
            progress.stage = MoveBackStage::RemovingSymlink;
            Some(progress.progress.clone())
        } else {
            None
        };

        let remove_symlink_res = async_fs::remove_dir(self).await;

        if let Err(err) = remove_symlink_res {
            return Err(MoveBackError::Io(err.kind()));
        }

        if let Some(progress) = progress.as_ref() {
            inner_progress.as_ref().unwrap().lock().unwrap().zero();
            progress.lock().unwrap().stage = MoveBackStage::Copying;
        }

        let copy_res = dest
            .copy_directory(self, inner_progress.clone(), cancellation_token.clone())
            .await;

        if let Err(err) = copy_res {
            return match err {
                CopyDirectoryError::DestinationExists => {
                    panic!("This should never happen. Destination should be the symlink.")
                }
                CopyDirectoryError::SymlinkEncountered => Err(MoveBackError::SymlinkEncountered),
                CopyDirectoryError::Cancelled => Err(MoveBackError::Cancelled),
                CopyDirectoryError::Io(e) => Err(MoveBackError::Io(e)),
            };
        }

        if let Some(progress) = progress.as_ref() {
            inner_progress.as_ref().unwrap().lock().unwrap().zero();
            progress.lock().unwrap().stage = MoveBackStage::Verifying;
        }

        let verify_res = self
            .verify_copy(dest, inner_progress.clone(), cancellation_token.clone())
            .await;

        if let Err(err) = verify_res {
            return match err {
                VerifyDirectoryError::Cancelled => Err(MoveBackError::Cancelled),
                VerifyDirectoryError::InvalidData => Err(MoveBackError::VerificationFailed),
                VerifyDirectoryError::Io(e) => Err(MoveBackError::Io(e)),
            };
        }

        let remove_res = async_fs::remove_dir_all(dest).await;

        if let Err(err) = remove_res {
            return Err(MoveBackError::Io(err.kind()));
        }

        if let Some(progress) = progress.as_ref() {
            progress.lock().unwrap().stage = MoveBackStage::Finished;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DirectoryStatsError {
    Io(io::ErrorKind),
    Cancelled,
}

#[derive(Debug, Default, Clone)]
pub struct DirectoryStats {
    pub subfolder_count: u32,
    pub file_count: u32,
    pub symlink_count: u32,
    pub size: FileSize,
}

#[derive(Debug, Clone, Copy)]
pub enum CopyDirectoryError {
    Io(io::ErrorKind),
    DestinationExists,
    SymlinkEncountered,
    Cancelled,
}

#[derive(Debug, Clone, Copy)]
pub enum VerifyDirectoryError {
    Io(io::ErrorKind),
    Cancelled,
    InvalidData,
}

#[derive(Debug, Default, Clone, Copy)]
pub enum ProcessDirectoryState {
    #[default]
    InProgress,
    Finished,
    Aborted,
}

#[derive(Debug, Default, Clone)]
pub struct ProcessDirectoryProgress {
    pub state: ProcessDirectoryState,
    pub total_files: u32,
    pub processed_files: u32,
    pub total_size: FileSize,
    pub processed_size: FileSize,
}

impl From<&DirectoryStats> for ProcessDirectoryProgress {
    fn from(value: &DirectoryStats) -> Self {
        ProcessDirectoryProgress::new(value.file_count, value.size)
    }
}

impl ProcessDirectoryProgress {
    pub fn new(total_files: u32, total_size: FileSize) -> Self {
        Self {
            state: ProcessDirectoryState::InProgress,
            total_files,
            processed_files: 0,
            total_size,
            processed_size: FileSize::ZERO,
        }
    }

    pub fn copied_files_frac(&self) -> Fraction {
        Fraction::from_ratio(self.processed_files, self.total_files).unwrap()
    }

    pub fn copied_size_frac(&self) -> Fraction {
        Fraction::from_ratio(self.processed_size, self.total_size).unwrap()
    }

    pub fn zero(&mut self) {
        self.state = ProcessDirectoryState::InProgress;
        self.processed_files = 0;
        self.processed_size = FileSize::ZERO;
    }

    pub fn process_file(&mut self, size: FileSize) {
        self.processed_files += 1;
        self.processed_size += size;
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MoveAndSymlinkStage {
    #[default]
    Copying,
    Verifying,
    Symlinking,
    Finished,
}

#[derive(Debug, Clone, Copy)]
pub enum MoveAndSymlinkError {
    Io(io::ErrorKind),
    DestinationExists,
    SymlinkEncountered,
    VerificationFailed,
    Cancelled,
}

#[derive(Debug, Default, Clone)]
pub struct MoveAndSymlinkProgress {
    pub stage: MoveAndSymlinkStage,
    pub progress: Arc<Mutex<ProcessDirectoryProgress>>,
}

impl From<&DirectoryStats> for MoveAndSymlinkProgress {
    fn from(value: &DirectoryStats) -> Self {
        Self::new(value.file_count, value.size)
    }
}

impl MoveAndSymlinkProgress {
    pub fn new(total_files: u32, total_size: FileSize) -> Self {
        Self {
            stage: MoveAndSymlinkStage::Copying,
            progress: Arc::new(Mutex::new(ProcessDirectoryProgress::new(
                total_files,
                total_size,
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum MoveBackStage {
    #[default]
    RemovingSymlink,
    Copying,
    Verifying,
    Finished,
}

#[derive(Debug, Clone, Copy)]
pub enum MoveBackError {
    Io(io::ErrorKind),
    SymlinkEncountered,
    VerificationFailed,
    Cancelled,
}

#[derive(Debug, Default, Clone)]
pub struct MoveBackProgress {
    pub stage: MoveBackStage,
    pub progress: Arc<Mutex<ProcessDirectoryProgress>>,
}

impl From<&DirectoryStats> for MoveBackProgress {
    fn from(value: &DirectoryStats) -> Self {
        Self::new(value.file_count, value.size)
    }
}

impl MoveBackProgress {
    pub fn new(total_files: u32, total_size: FileSize) -> Self {
        Self {
            stage: MoveBackStage::RemovingSymlink,
            progress: Arc::new(Mutex::new(ProcessDirectoryProgress::new(
                total_files,
                total_size,
            ))),
        }
    }
}

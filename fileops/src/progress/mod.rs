mod copy_file;
mod stage;

pub use copy_file::copy_file_with_progress;

use crate::dirstats::DirectoryStats;
use crate::prelude::{AsBytes, FileSize};
use atomiq::{Atomic, Atomize, Ordering};
use unifrac::Primant;

#[derive(Debug, Clone)]
pub struct DirectoryProgress {
    pub total_files: u64,
    pub total_size: FileSize,
    pub processed_files: u64,
    pub processed_size: FileSize,
}

impl From<DirectoryStats> for DirectoryProgress {
    fn from(stats: DirectoryStats) -> Self {
        DirectoryProgress::new_zeroed(stats.total_files, stats.total_size)
    }
}

impl DirectoryProgress {
    pub fn new_zeroed(total_files: u64, total_size: FileSize) -> Self {
        Self {
            total_files,
            total_size,
            processed_files: 0,
            processed_size: 0.bytes(),
        }
    }

    pub fn files_to_primant(&self) -> Primant {
        Primant::from_ratio_saturating(self.processed_files, self.total_files)
    }

    pub fn size_to_primant(&self) -> Primant {
        Primant::from_ratio_saturating(self.processed_size.as_bytes(), self.total_size.as_bytes())
    }
}

pub struct AtomicDirectoryProgress {
    pub total_files: u64,
    pub total_size: FileSize,
    pub processed_files: Atomic<u64>,
    pub processed_size: Atomic<FileSize>,
}

impl From<DirectoryStats> for AtomicDirectoryProgress {
    fn from(stats: DirectoryStats) -> Self {
        AtomicDirectoryProgress::new_zeroed(stats.total_files, stats.total_size)
    }
}

impl From<DirectoryProgress> for AtomicDirectoryProgress {
    fn from(progress: DirectoryProgress) -> Self {
        AtomicDirectoryProgress::new(
            progress.total_files,
            progress.total_size,
            progress.processed_files,
            progress.processed_size,
        )
    }
}

impl AtomicDirectoryProgress {
    pub fn new_zeroed(file_count: u64, total_size: FileSize) -> Self {
        Self::new(file_count, total_size, 0, 0.bytes())
    }

    pub fn new(
        file_count: u64,
        total_size: FileSize,
        processed_files: u64,
        processed_size: FileSize,
    ) -> Self {
        Self {
            total_files: file_count,
            total_size,
            processed_files: processed_files.atomize(),
            processed_size: processed_size.atomize(),
        }
    }

    pub fn load(&self) -> DirectoryProgress {
        DirectoryProgress {
            total_files: self.total_files,
            total_size: self.total_size,
            processed_files: self.processed_files.load(Ordering::Relaxed),
            processed_size: self.processed_size.load(Ordering::Relaxed),
        }
    }

    pub fn store(&self, progress: DirectoryProgress) {
        assert_eq!(
            self.total_files, progress.total_files,
            "total_files mismatch"
        );
        assert_eq!(self.total_size, progress.total_size, "total_size mismatch");
        self.processed_files
            .store(progress.processed_files, Ordering::Relaxed);
        self.processed_size
            .store(progress.processed_size, Ordering::Relaxed);
    }

    pub fn add_file(&self, size: FileSize) {
        self.processed_files.fetch_add(1, Ordering::Relaxed);
        self.processed_size.fetch_add(size, Ordering::Relaxed);
    }

    pub fn reset(&self) {
        self.processed_files.store(0, Ordering::Relaxed);
        self.processed_size.store(0.bytes(), Ordering::Relaxed);
    }
}

#[cfg(test)]
#[cfg(feature = "loom")]
mod tests {
    use super::*;
    use loom::sync::Arc;
    use loom::{model, thread};
    use test_log::test;

    #[test]
    fn test_atomic_directory_progress() {
        model(move || {
            let progress: Arc<AtomicDirectoryProgress> =
                Arc::new(AtomicDirectoryProgress::new_zeroed(10, 100.bytes()));

            let t1 = thread::spawn({
                let progress = progress.clone();
                move || {
                    for _ in 0..3 {
                        // info!("add_file");
                        progress.add_file(10.bytes());
                    }
                }
            });

            let t2 = thread::spawn({
                let progress = progress.clone();
                move || {
                    let p = progress.load();
                    assert!((0..=30).contains(&p.processed_size.as_bytes()));
                    assert!((0..=3).contains(&p.processed_files));
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();

            let p = progress.load();
            assert_eq!(p.processed_files, 3);
            assert_eq!(p.processed_size.as_bytes(), 30);
        });
    }
}

mod copy_file;
mod stage;

use crate::dirstats::DirectoryStats;
use crate::prelude::{AsBytes, FileSize};
use atomiq::compat::IntAtomic;
use atomiq::Ordering;

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
}

pub struct AtomicDirectoryProgress<T, U>
where
    T: IntAtomic<Value = u64>,
    U: IntAtomic<Value = FileSize>,
{
    pub total_files: u64,
    pub total_size: FileSize,
    pub processed_files: T,
    pub processed_size: U,
}

impl<T, U> From<DirectoryStats> for AtomicDirectoryProgress<T, U>
where
    T: IntAtomic<Value = u64>,
    U: IntAtomic<Value = FileSize>,
{
    fn from(stats: DirectoryStats) -> Self {
        AtomicDirectoryProgress::new_zeroed(stats.total_files, stats.total_size)
    }
}

impl<T, U> From<DirectoryProgress> for AtomicDirectoryProgress<T, U>
where
    T: IntAtomic<Value = u64>,
    U: IntAtomic<Value = FileSize>,
{
    fn from(progress: DirectoryProgress) -> Self {
        AtomicDirectoryProgress::new(
            progress.total_files,
            progress.total_size,
            T::new(progress.processed_files),
            U::new(progress.processed_size),
        )
    }
}

impl<T, U> AtomicDirectoryProgress<T, U>
where
    T: IntAtomic<Value = u64>,
    U: IntAtomic<Value = FileSize>,
{
    pub fn new_zeroed(file_count: u64, total_size: FileSize) -> Self {
        Self {
            total_files: file_count,
            total_size,
            processed_files: T::new(0),
            processed_size: U::new(0.bytes()),
        }
    }

    pub fn new(
        file_count: u64,
        total_size: FileSize,
        processed_files: T,
        processed_size: U,
    ) -> Self {
        Self {
            total_files: file_count,
            total_size,
            processed_files,
            processed_size,
        }
    }

    pub fn load(&self, ordering: Ordering) -> DirectoryProgress {
        DirectoryProgress {
            total_files: self.total_files,
            total_size: self.total_size,
            processed_files: self.processed_files.load(ordering),
            processed_size: self.processed_size.load(ordering),
        }
    }

    pub fn store(&self, progress: DirectoryProgress, ordering: Ordering) {
        assert_eq!(
            self.total_files, progress.total_files,
            "total_files mismatch"
        );
        assert_eq!(self.total_size, progress.total_size, "total_size mismatch");
        self.processed_files
            .store(progress.processed_files, ordering);
        self.processed_size.store(progress.processed_size, ordering);
    }

    pub fn add_file(&self, size: FileSize, ordering: Ordering) {
        self.processed_files.fetch_add(1, ordering);
        self.processed_size.fetch_add(size, ordering);
    }

    pub fn reset(&self, ordering: Ordering) {
        self.processed_files.store(0, ordering);
        self.processed_size.store(0.bytes(), ordering);
    }
}

#[cfg(test)]
#[cfg(feature = "loom")]
mod tests {
    use super::*;
    use crate::file_size::AtomicFileSize;
    use atomiq::compat::loom::AtomicU64;
    use atomiq::OrderingExt;
    use loom::sync::Arc;
    use loom::{model, thread};
    use test_log::test;

    fn test_atomic_directory_progress(ordering: Ordering) {
        model(move || {
            let progress: Arc<AtomicDirectoryProgress<AtomicU64, AtomicFileSize<AtomicU64>>> =
                Arc::new(AtomicDirectoryProgress::new_zeroed(10, 100.bytes()));

            let t1 = thread::spawn({
                let progress = progress.clone();
                move || {
                    for _ in 0..3 {
                        // info!("add_file");
                        progress.add_file(10.bytes(), ordering.for_store());
                    }
                }
            });

            let t2 = thread::spawn({
                let progress = progress.clone();
                move || {
                    let p = progress.load(ordering.for_load());
                    assert!((0..=30).contains(&p.processed_size.as_bytes()));
                    assert!((0..=3).contains(&p.processed_files));
                }
            });

            t1.join().unwrap();
            t2.join().unwrap();

            let p = progress.load(ordering.for_load());
            assert_eq!(p.processed_files, 3);
            assert_eq!(p.processed_size.as_bytes(), 30);
        });
    }

    // #[test]
    // #[should_panic]
    // fn test_atomic_directory_progress_relaxed() {
    //     test_atomic_directory_progress(Ordering::Relaxed);
    // }

    #[test]
    fn test_atomic_directory_progress_acq_rel() {
        test_atomic_directory_progress(Ordering::AcqRel);
    }
}

use crate::file_size::num_ext::{AsBytes, AsBytesMult};
use crate::pathext::FileKind;
use crate::prelude::fs::windows::OpenOptionsExt;
use crate::prelude::*;
use crate::progress::AtomicDirectoryProgress;
use crate::Result;
use smol::io::{AsyncReadExt, AsyncWriteExt};
use std::path::Path;
use std::sync::atomic::Ordering;
use windows::Win32::Storage::FileSystem::{FILE_SHARE_NONE, FILE_SHARE_READ};

async fn copy_file_with_progress<'a>(
    src: &'a Path,
    dst: &'a Path,
    progress: &'a AtomicDirectoryProgress,
) -> Result<()> {
    let src_meta = fs::symlink_metadata(src)
        .await
        .map_err(|e| Error::Io(e.kind()))?;

    let src_kind = FileKind::from(&src_meta);

    if src_kind != FileKind::File {
        return Err(Error::UnexpectedFileKind(src_kind));
    }

    let dst_exists = dst.try_exists().map_err(|e| Error::Io(e.kind()))?;

    if dst_exists {
        return Err(Error::AlreadyExists);
    }

    let file_size = src_meta.len().bytes();

    if file_size < 10.mb() {
        // Small files are copied without progress.
        let copied_bytes = fs::copy(src, dst).await.map_err(|e| Error::Io(e.kind()))?;

        progress.add_file(copied_bytes.bytes());

        return Ok(());
    }

    let mut src_file = fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ.0) // Allow other processes to read the file but not modify it.
        .open(src)
        .await
        .map_err(|e| Error::Io(e.kind()))?;
    let mut dst_file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .share_mode(FILE_SHARE_NONE.0) // Don't allow other processes to access the file.
        .open(dst)
        .await
        .map_err(|e| Error::Io(e.kind()))?;

    // First we allocate the file size.
    // TODO Avoid filling the file with zeros. (May require platform-specific implementation.)
    dst_file
        .set_len(file_size.as_bytes())
        .await
        .map_err(|e| Error::Io(e.kind()))?;

    // Then we copy the file bit by bit.
    const BUFFER_SIZE: usize = 16 * 1024; // 16 KiB
    let mut buffer = [0u8; BUFFER_SIZE];

    loop {
        let read_bytes = src_file
            .read(&mut buffer)
            .await
            .map_err(|e| Error::Io(e.kind()))?;

        if read_bytes == 0 {
            break;
        }

        dst_file
            .write_all(&buffer[..read_bytes])
            .await
            .map_err(|e| Error::Io(e.kind()))?;

        progress
            .processed_size
            .fetch_add((read_bytes as u64).bytes(), Ordering::Relaxed);
    }

    progress.processed_files.fetch_add(1, Ordering::Relaxed);

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::prelude::{AsBytes, AsBytesMult, FileSize};
    use crate::progress::copy_file::copy_file_with_progress;
    use crate::progress::AtomicDirectoryProgress;
    use atomiq::Ordering;
    use log::{debug, info};
    use rand::Rng;
    use std::io::Write;
    use std::ops::Rem;
    use std::path::PathBuf;
    use std::time::Duration;
    use std::{env, thread};
    use test_log::test;
    use uuid::Uuid;

    fn create_temp_file(min_size: FileSize) -> std::io::Result<PathBuf> {
        info!("Creating temporary file of size >={}...", min_size);
        let temp_dir = env::temp_dir();
        let uuid = Uuid::new_v4().to_string();
        let file_name = format!("moverr_test_{}", uuid);
        let file_path = temp_dir.join(file_name);
        let mut file = std::fs::File::create_new(&file_path)?;
        let mut rng = rand::thread_rng();

        const BUFFER_SIZE: u64 = 1024;
        let rem_some = min_size.as_bytes().rem(BUFFER_SIZE) > 0;
        let buffer_writes = min_size.as_bytes() / BUFFER_SIZE + rem_some as u64;
        let real_size = (buffer_writes * BUFFER_SIZE).bytes();
        debug!("Real size: {}", real_size);
        file.set_len(real_size.as_bytes())?;
        let mut written_size = 0.bytes();
        let mut buffer = [0u8; 1024];
        rng.fill(&mut buffer);
        while written_size < real_size {
            file.write_all(&buffer)?;

            written_size += (buffer.len() as u64).bytes();
        }

        info!("Created temporary file: {}", file_path.display());
        Ok(file_path)
    }

    #[test]
    fn test() {
        let source_path = create_temp_file(500.mb()).unwrap();

        let dest_path = source_path.with_file_name(format!(
            "{}_copy",
            source_path.file_name().unwrap().to_str().unwrap()
        ));

        let size = std::fs::metadata(&source_path).unwrap().len().bytes();

        let progress: AtomicDirectoryProgress = AtomicDirectoryProgress::new_zeroed(1, size);

        info!("Spawning threads...");
        thread::scope(|s| {
            let a = s.spawn(|| {
                smol::block_on(async {
                    copy_file_with_progress(&source_path, &dest_path, &progress)
                        .await
                        .unwrap();
                });
            });

            let b = s.spawn(|| {
                smol::block_on(async {
                    loop {
                        let progress = progress.load();

                        debug!(
                            "Processed files: {}/{} ({:.2}%)",
                            progress.processed_files,
                            progress.total_files,
                            (progress.processed_files as f64 / progress.total_files as f64) * 100.0
                        );
                        debug!(
                            "Processed size: {}/{} ({:.2}%)",
                            progress.processed_size,
                            progress.total_size,
                            (progress.processed_size.as_bytes() as f64
                                / progress.total_size.as_bytes() as f64)
                                * 100.0
                        );

                        if progress.processed_files == progress.total_files {
                            break;
                        }

                        smol::Timer::after(Duration::from_millis(1000)).await;
                    }
                });
            });

            a.join().unwrap();
            b.join().unwrap();
        });

        info!("Done");

        // Cleanup
        std::fs::remove_file(&source_path).unwrap();
        std::fs::remove_file(&dest_path).unwrap();

        info!("Cleaned up");

        let progress = progress.load();

        assert_eq!(progress.processed_files, 1);
        assert_eq!(progress.processed_size, size);

        debug!(
            "Processed files: {}/{} ({:.2}%)",
            progress.processed_files,
            progress.total_files,
            (progress.processed_files as f64 / progress.total_files as f64) * 100.0
        );
        debug!(
            "Processed size: {}/{} ({:.2}%)",
            progress.processed_size,
            progress.total_size,
            (progress.processed_size.as_bytes() as f64 / progress.total_size.as_bytes() as f64)
                * 100.0
        );
    }
}

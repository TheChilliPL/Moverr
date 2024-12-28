use crate::file_size::FileSize;

#[derive(Debug, Clone, Default)]
pub struct DirectoryStats {
    pub total_files: u64,
    pub total_size: FileSize,
}

impl DirectoryStats {
    pub fn new(file_count: u64, total_size: FileSize) -> Self {
        Self {
            total_files: file_count,
            total_size,
        }
    }
}

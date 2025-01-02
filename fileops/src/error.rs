use crate::pathext::FileKind;
use thiserror::Error;

#[repr(u8)]
#[derive(Debug, Clone, Error, Eq, PartialEq)]
pub enum Error {
    // 0 unused for Option<Error>::None.
    // General errors. (1-127)
    #[error("I/O error: {0}")]
    Io(std::io::ErrorKind) = 1,
    #[error("File not found")]
    NotFound,
    #[error("File already exists")]
    AlreadyExists,
    #[error("Unexpected file kind: {0:?}")]
    UnexpectedFileKind(FileKind),
    #[error("Invalid path")]
    InvalidPath,

    // Platform-specific errors. (128-255)
    #[cfg(windows)]
    #[error("Windows API error: {0}")]
    WinApi(#[from] windows::core::Error) = 128,
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            std::io::ErrorKind::IsADirectory => Self::UnexpectedFileKind(FileKind::Directory),
            std::io::ErrorKind::NotADirectory => Self::UnexpectedFileKind(FileKind::File),
            _ => Self::Io(err.kind()),
        }
    }
}

impl Error {
    fn discriminant(&self) -> u8 {
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }

    pub fn is_platform_specific(&self) -> bool {
        self.discriminant() >= 128
    }
}

pub type Result<T> = std::result::Result<T, Error>;

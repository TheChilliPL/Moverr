use crate::pathext::FileKind;

#[repr(u8)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    // 0 unused for Option<Error>::None.
    // General errors. (1-127)
    Io(std::io::ErrorKind) = 1,
    NotFound,
    AlreadyExists,
    UnexpectedFileKind(FileKind),
    InvalidPath,

    // Platform-specific errors. (128-255)
    #[cfg(windows)]
    WinApi(windows::core::Error) = 128,
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

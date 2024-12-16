use crate::file_size::FileSize;

pub trait AsBytes {
    fn bytes(self) -> FileSize;
}

pub trait AsBytesMult {
    fn kb(self) -> FileSize;
    fn mb(self) -> FileSize;
    fn gb(self) -> FileSize;
    fn tb(self) -> FileSize;
}

impl AsBytes for u64 {
    fn bytes(self) -> FileSize {
        FileSize(self)
    }
}

impl AsBytesMult for u64 {
    fn kb(self) -> FileSize {
        FileSize(self * 1024)
    }

    fn mb(self) -> FileSize {
        FileSize(self * 1024 * 1024)
    }

    fn gb(self) -> FileSize {
        FileSize(self * 1024 * 1024 * 1024)
    }

    fn tb(self) -> FileSize {
        FileSize(self * 1024 * 1024 * 1024 * 1024)
    }
}

impl AsBytesMult for f64 {
    fn kb(self) -> FileSize {
        FileSize((self * 1024.0) as u64)
    }

    fn mb(self) -> FileSize {
        FileSize((self * 1024.0 * 1024.0) as u64)
    }

    fn gb(self) -> FileSize {
        FileSize((self * 1024.0 * 1024.0 * 1024.0) as u64)
    }

    fn tb(self) -> FileSize {
        FileSize((self * 1024.0 * 1024.0 * 1024.0 * 1024.0) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_bytes() {
        assert_eq!(1.bytes(), FileSize(1));
        assert_eq!(1.kb(), FileSize(1024));
        assert_eq!(1.mb(), FileSize(1024 * 1024));
        assert_eq!(1.gb(), FileSize(1024 * 1024 * 1024));
        assert_eq!(1.tb(), FileSize(1024 * 1024 * 1024 * 1024));
    }

    #[test]
    fn test_as_bytes_mult() {
        assert_eq!(1.0.kb(), FileSize(1024));
        assert_eq!(1.0.mb(), FileSize(1024 * 1024));
        assert_eq!(1.0.gb(), FileSize(1024 * 1024 * 1024));
        assert_eq!(1.0.tb(), FileSize(1024 * 1024 * 1024 * 1024));
    }
}

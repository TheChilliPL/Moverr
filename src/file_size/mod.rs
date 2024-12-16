use crate::file_size::units::FileSizeUnit;
use num_format::{CustomFormat, Grouping, ToFormattedString};
use std::fmt::{Debug, Display};
use std::ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

pub mod num_ext;
pub mod units;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct FileSize(u64);

impl FileSize {
    fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }
}

impl From<u64> for FileSize {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl Add for FileSize {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl AddAssign for FileSize {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl Sub for FileSize {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl SubAssign for FileSize {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Mul<u64> for FileSize {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<f64> for FileSize {
    type Output = Self;

    fn mul(self, rhs: f64) -> Self::Output {
        Self((self.0 as f64 * rhs) as u64)
    }
}

impl MulAssign<u64> for FileSize {
    fn mul_assign(&mut self, rhs: u64) {
        self.0 *= rhs;
    }
}

impl MulAssign<f64> for FileSize {
    fn mul_assign(&mut self, rhs: f64) {
        self.0 = (self.0 as f64 * rhs) as u64;
    }
}

impl Div for FileSize {
    type Output = f64;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 as f64 / rhs.0 as f64
    }
}

impl Div<u64> for FileSize {
    type Output = Self;

    fn div(self, rhs: u64) -> Self::Output {
        Self(self.0 / rhs)
    }
}

impl DivAssign<u64> for FileSize {
    fn div_assign(&mut self, rhs: u64) {
        self.0 /= rhs;
    }
}

impl FileSize {
    pub fn choose_unit(file_size: FileSize) -> (f64, FileSizeUnit) {
        const THRESHOLD: f64 = 1024.0; // * 1.25;

        let mut value = file_size.0 as f64;
        let mut unit = FileSizeUnit::Byte;

        while value >= THRESHOLD {
            value /= 1024.0;
            unit = unit.next().unwrap();
        }

        (value, unit)
    }

    pub fn to_string_unit(&self) -> String {
        if self.0 == 0 {
            return "0 B".to_string();
        }

        let format = CustomFormat::builder()
            .grouping(Grouping::Standard)
            .separator("")
            .decimal(".")
            .build()
            .unwrap();
        let (value, unit) = Self::choose_unit(*self);

        const SKIP_FRAC_THRESHOLD: f64 = 10.0;

        let floor = value.floor();
        let frac = value - floor;

        let floor = floor as u64;
        let frac = (frac * 10.0).floor() as u64;

        if value >= SKIP_FRAC_THRESHOLD {
            format!(
                "{} {}",
                floor.to_formatted_string(&format),
                unit.to_acronym()
            )
        } else {
            format!(
                "{}.{} {}",
                floor.to_formatted_string(&format),
                frac,
                unit.to_acronym()
            )
        }
    }
}

impl Display for FileSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_string_unit())
    }
}

impl Debug for FileSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FileSize({})", self.to_string_unit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_size() {
        let file_size = FileSize::from_bytes(0);
        assert_eq!(file_size.to_string_unit(), "0 B");

        let file_size = FileSize::from_bytes(1000);
        assert_eq!(file_size.to_string_unit(), "1000 B");

        let file_size = FileSize::from_bytes(1024);
        assert_eq!(file_size.to_string_unit(), "1.0 KiB");

        let file_size = FileSize::from_bytes(1024 * 1024);
        assert_eq!(file_size.to_string_unit(), "1.0 MiB");

        let file_size = FileSize::from_bytes(1024 * 1024 * 1024);
        assert_eq!(file_size.to_string_unit(), "1.0 GiB");

        let file_size = FileSize::from_bytes(1536 * 1024 * 1024);
        assert_eq!(file_size.to_string_unit(), "1.5 GiB");
    }

    #[test]
    fn test_debug() {
        let file_size = FileSize::from_bytes(1536);
        assert_eq!(format!("{:?}", file_size), "FileSize(1.5 KiB)");
    }
}

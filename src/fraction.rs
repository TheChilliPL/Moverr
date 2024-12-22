use crate::file_size::FileSize;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Add, Mul, Sub};

/// A simple struct representing a fraction from 0 to 1.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fraction(u32);

impl Fraction {
    const MIN: Fraction = Fraction(0);
    const ZERO: Fraction = Fraction(0);
    const MAX: Fraction = Fraction(u32::MAX);
}

impl TryFrom<f64> for Fraction {
    type Error = ();

    fn try_from(value: f64) -> Result<Fraction, ()> {
        if (0.0..=1.0).contains(&value) {
            Ok(Fraction((value * (u32::MAX as f64)) as u32))
        } else {
            Err(())
        }
    }
}

impl TryFrom<f32> for Fraction {
    type Error = ();

    fn try_from(value: f32) -> Result<Fraction, ()> {
        if (0.0..=1.0).contains(&value) {
            Ok(Fraction((value * (u32::MAX as f32)) as u32))
        } else {
            Err(())
        }
    }
}

impl Fraction {
    pub fn into_f64(self) -> f64 {
        self.0 as f64 / u32::MAX as f64
    }

    pub fn into_f32(self) -> f32 {
        self.0 as f32 / u32::MAX as f32
    }

    pub fn into_percent(self) -> f64 {
        self.into_f64() * 100.0
    }

    pub fn from_percent(percent: f64) -> Result<Fraction, ()> {
        if (0.0..=100.0).contains(&percent) {
            Ok(Fraction::try_from(percent / 100.0)?)
        } else {
            Err(())
        }
    }
}

pub trait FromRatio<T> {
    fn from_ratio(numerator: T, denominator: T) -> Result<Fraction, ()>;
}

impl FromRatio<u32> for Fraction {
    fn from_ratio(numerator: u32, denominator: u32) -> Result<Fraction, ()> {
        if denominator == 0 {
            return Err(());
        }
        Fraction::try_from(numerator as f64 / denominator as f64)
    }
}

impl FromRatio<u64> for Fraction {
    fn from_ratio(numerator: u64, denominator: u64) -> Result<Fraction, ()> {
        if denominator == 0 {
            return Err(());
        }
        Fraction::try_from(numerator as f64 / denominator as f64)
    }
}

impl FromRatio<FileSize> for Fraction {
    fn from_ratio(numerator: FileSize, denominator: FileSize) -> Result<Fraction, ()> {
        if denominator == FileSize::ZERO {
            return Err(());
        }
        Fraction::try_from((numerator.as_bytes() as f64) / (denominator.as_bytes() as f64))
    }
}

impl Into<f64> for Fraction {
    fn into(self) -> f64 {
        self.into_f64()
    }
}

impl Into<f32> for Fraction {
    fn into(self) -> f32 {
        self.into_f32()
    }
}

impl Debug for Fraction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.into_f64())
    }
}

impl Display for Fraction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.2}%", self.into_percent())
    }
}

impl Add for Fraction {
    type Output = Fraction;

    fn add(self, rhs: Self) -> Self::Output {
        Fraction(self.0 + rhs.0)
    }
}

impl Sub for Fraction {
    type Output = Fraction;

    fn sub(self, rhs: Self) -> Self::Output {
        Fraction(self.0 - rhs.0)
    }
}

impl Mul<f64> for Fraction {
    type Output = f64;

    fn mul(self, rhs: f64) -> Self::Output {
        self.into_f64() * rhs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const EPSILON: f64 = 1e-6;

    fn assert_approx_eq<T: Into<f64>>(a: T, b: T) {
        assert!((a.into() - b.into()).abs() < EPSILON);
    }

    #[test]
    fn test_f64_conversion() {
        let fraction = Fraction::try_from(0.5f64).unwrap();
        let f: f64 = fraction.into();
        assert_approx_eq(f, 0.5);
    }

    #[test]
    fn test_f32_conversion() {
        let fraction = Fraction::try_from(0.5f32).unwrap();
        let f: f32 = fraction.into();
        assert_approx_eq(f, 0.5);
    }

    #[test]
    fn test_into_percent() {
        let fraction = Fraction::try_from(0.5f64).unwrap();
        let percent = fraction.into_percent();
        assert_approx_eq(percent, 50.0);
    }

    #[test]
    fn test_from_percent() {
        let fraction = Fraction::from_percent(50.0).unwrap();
        assert_approx_eq(fraction.into_f64(), 0.5);
    }

    #[test]
    fn test_from_ratio() {
        let fraction = Fraction::from_ratio(1u32, 2u32).unwrap();
        assert_approx_eq(fraction.into_f64(), 0.5);
    }

    #[test]
    fn test_debug() {
        let fraction = Fraction::try_from(0.5f64).unwrap();
        assert_approx_eq(f64::from_str(&format!("{:?}", fraction)).unwrap(), 0.5);
    }

    #[test]
    fn test_display() {
        let fraction = Fraction::try_from(0.5f64).unwrap();
        assert_eq!(format!("{}", fraction), "50.00%");
    }
}

use std::any::Any;
use std::ops::Bound;
use std::ops::{Add, Div, Mul, RangeBounds, RangeInclusive, Sub};

pub trait Scalar:
    Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Sized
    + Copy
    + Clone
    + PartialOrd
    + PartialEq
{
    const SIGNED: bool;
    const MIN: Self;
    const ZERO: Self;
    const ONE: Self;
    const MAX: Self;
}

pub trait Integer: Scalar + Ord + Eq {}

macro_rules! impl_float {
    ($type:ident) => {
        impl Scalar for $type {
            const SIGNED: bool = true;
            const MIN: Self = Self::MIN;
            const ZERO: Self = 0.0;
            const ONE: Self = 1.0;
            const MAX: Self = Self::MAX;
        }
    };
}

macro_rules! impl_integer {
    (unsigned $type:ident) => {
        impl Scalar for $type {
            const SIGNED: bool = false;
            const MIN: Self = Self::MIN;
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const MAX: Self = Self::MAX;
        }
        impl Integer for $type {}
    };
    (signed $type:ident) => {
        impl Scalar for $type {
            const SIGNED: bool = true;
            const MIN: Self = Self::MIN;
            const ZERO: Self = 0;
            const ONE: Self = 1;
            const MAX: Self = Self::MAX;
        }
        impl Integer for $type {}
    };
}

impl_integer!(unsigned u8);
impl_integer!(unsigned u16);
impl_integer!(unsigned u32);
impl_integer!(unsigned u64);
impl_integer!(unsigned u128);
impl_integer!(unsigned usize);
impl_integer!(signed i8);
impl_integer!(signed i16);
impl_integer!(signed i32);
impl_integer!(signed i64);
impl_integer!(signed i128);
impl_integer!(signed isize);
impl_float!(f32);
impl_float!(f64);

pub trait FromRangeBounds<T> {
    fn from_range_bounds(range: impl RangeBounds<T>) -> Self;
}

macro_rules! impl_from_range_bounds {
    ($type:ident) => {
        impl FromRangeBounds<$type> for RangeInclusive<$type> {
            fn from_range_bounds(range: impl RangeBounds<$type>) -> Self {
                let start = match range.start_bound() {
                    Bound::Included(&start) => start,
                    Bound::Excluded(&start) => start + $type::ONE,
                    Bound::Unbounded => $type::MIN,
                };
                let end = match range.end_bound() {
                    Bound::Included(&end) => end,
                    Bound::Excluded(&end) => end - $type::ONE,
                    Bound::Unbounded => $type::MAX,
                };
                start..=end
            }
        }
    };
}

impl_from_range_bounds!(u8);
impl_from_range_bounds!(u16);
impl_from_range_bounds!(u32);
impl_from_range_bounds!(u64);
impl_from_range_bounds!(u128);
impl_from_range_bounds!(usize);
impl_from_range_bounds!(i8);
impl_from_range_bounds!(i16);
impl_from_range_bounds!(i32);
impl_from_range_bounds!(i64);
impl_from_range_bounds!(i128);
impl_from_range_bounds!(isize);

pub trait ClipToRange<T> {
    fn clip_to_range(self, range: RangeInclusive<T>) -> T;
}

pub trait ClipToBounds<T> {
    // Panics when range is empty.
    fn clip_to(self, bounds: impl RangeBounds<T>) -> T;
}

macro_rules! impl_clip_to_range {
    ($type:ident) => {
        impl ClipToRange<$type> for $type {
            fn clip_to_range(self, range: RangeInclusive<$type>) -> $type {
                if range.is_empty() {
                    panic!("Couldn't clip value to range");
                }
                self.min(*range.end()).max(*range.start())
            }
        }
    };
}

macro_rules! impl_clip_to_bounds {
    ($type:ident) => {
        impl_clip_to_range!($type);
        impl ClipToBounds<$type> for $type {
            fn clip_to(self, bounds: impl RangeBounds<$type>) -> $type {
                let range = RangeInclusive::from_range_bounds(bounds);
                self.clip_to_range(range)
            }
        }
    };
}

impl_clip_to_bounds!(u8);
impl_clip_to_bounds!(u16);
impl_clip_to_bounds!(u32);
impl_clip_to_bounds!(u64);
impl_clip_to_bounds!(u128);
impl_clip_to_bounds!(usize);
impl_clip_to_bounds!(i8);
impl_clip_to_bounds!(i16);
impl_clip_to_bounds!(i32);
impl_clip_to_bounds!(i64);
impl_clip_to_bounds!(i128);
impl_clip_to_bounds!(isize);
impl_clip_to_range!(f32);
impl_clip_to_range!(f64);

pub trait AsAny {
    fn as_any(&self) -> &dyn Any;
}

pub trait AsAnyMut: AsAny {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

macro_rules! impl_as_any {
    ($type:ty) => {
        impl AsAny for $type {
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
        }
    };
}

macro_rules! impl_as_any_mut {
    ($type:ty) => {
        crate::utils::impl_as_any!($type);
        impl AsAnyMut for $type {
            fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                self
            }
        }
    };
}

pub(crate) use impl_as_any;
pub(crate) use impl_as_any_mut;

pub trait Pad {
    fn pad_left(self, width: usize) -> String;
    fn pad_center(self, width: usize) -> String;
    fn pad_right(self, width: usize) -> String;
}

impl Pad for &str {
    fn pad_left(self, width: usize) -> String {
        format!("{:>width$}", self, width = width)
    }

    fn pad_center(self, width: usize) -> String {
        format!("{:^width$}", self, width = width)
    }

    fn pad_right(self, width: usize) -> String {
        format!("{:<width$}", self, width = width)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Not;

    #[test]
    fn test_clip_int() {
        assert_eq!(0.clip_to(0..=0), 0);
        assert_eq!(0.clip_to(0..=10), 0);
        assert_eq!(0.clip_to(0..10), 0);
        assert_eq!(0.clip_to(1..10), 1);
    }

    #[test]
    #[should_panic(expected = "Couldn't clip value to range")]
    fn test_clip_empty() {
        0.clip_to(0..0);
    }

    #[test]
    fn test_clip_float() {
        assert_eq!(0.0.clip_to_range(0.0..=0.0), 0.0);
        assert_eq!(0.0.clip_to_range(0.0..=10.0), 0.0);
        // assert_eq!(0.0.clip_to(0.0..10.0), 0.0);
        // assert_eq!(0.0.clip_to(1.0..10.0), 1.0);
    }

    #[test]
    fn as_any_test() {
        struct TestStruct;

        impl AsAny for TestStruct {
            fn as_any(&self) -> &dyn Any {
                self
            }
        }

        let test_struct = TestStruct;

        assert!(test_struct.as_any().is::<TestStruct>());

        assert!(test_struct.as_any().is::<String>().not());
    }

    #[test]
    fn pad_test() {
        let str = "test";
        assert_eq!(str.pad_left(10), "      test");
        assert_eq!(str.pad_center(10), "   test   ");
        assert_eq!(str.pad_right(10), "test      ");
    }
}

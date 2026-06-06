//! Data point type for chart coordinates.
//!
//! A Datum represents a single data point with x and y coordinates in data space (f64).
//! This is separate from pixel-space coordinates which use iced's Point (f32).

/// Trait for types that can be converted to f64.
/// Used for generic datum construction from any numeric type.
pub trait Numeric: Copy {
    fn to_f64(self) -> f64;
}

macro_rules! impl_numeric {
    ($($t:ty),*) => {
        $(
            impl Numeric for $t {
                fn to_f64(self) -> f64 { self as f64 }
            }
        )*
    };
}

impl_numeric!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

/// A data point with x and y coordinates.
///
/// Uses f64 for precision with timestamps and large values.
/// Convert to pixel coordinates via `crate::chart::plot_area::to_pixel()`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Datum {
    pub x: f64,
    pub y: f64,
}

impl Datum {
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0 };

    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Create a datum on the x-axis (y = 0).
    pub fn x(x: f64) -> Self {
        Self { x, y: 0.0 }
    }

    /// Create a datum on the y-axis (x = 0).
    pub fn y(y: f64) -> Self {
        Self { x: 0.0, y }
    }
}

// Generic From for any tuple of numeric types
impl<X: Numeric, Y: Numeric> From<(X, Y)> for Datum {
    fn from((x, y): (X, Y)) -> Self {
        Self {
            x: x.to_f64(),
            y: y.to_f64(),
        }
    }
}

// Generic From for any array of numeric type
impl<T: Numeric> From<[T; 2]> for Datum {
    fn from([x, y]: [T; 2]) -> Self {
        Self {
            x: x.to_f64(),
            y: y.to_f64(),
        }
    }
}

/// Trait for converting various data formats into a collection of datums.
///
/// Two main patterns:
/// 1. Iterator of point-like things: `[(x, y), (x, y)]` -> each becomes a Datum
/// 2. Iterator of values: `[y, y, y]` -> auto-enumerate to (0, y), (1, y), (2, y)
pub trait IntoDatums {
    fn into_datums(self) -> Vec<Datum>;
}

// Arrays of tuples
impl<X: Numeric, Y: Numeric, const N: usize> IntoDatums for [(X, Y); N] {
    fn into_datums(self) -> Vec<Datum> {
        self.into_iter().map(Into::into).collect()
    }
}

// Slices of tuples
impl<X: Numeric, Y: Numeric> IntoDatums for &[(X, Y)] {
    fn into_datums(self) -> Vec<Datum> {
        self.iter().copied().map(Into::into).collect()
    }
}

// Vec of tuples
impl<X: Numeric, Y: Numeric> IntoDatums for Vec<(X, Y)> {
    fn into_datums(self) -> Vec<Datum> {
        self.into_iter().map(Into::into).collect()
    }
}

// Arrays of arrays
impl<T: Numeric, const N: usize> IntoDatums for [[T; 2]; N] {
    fn into_datums(self) -> Vec<Datum> {
        self.into_iter().map(Into::into).collect()
    }
}

// Slices of arrays
impl<T: Numeric> IntoDatums for &[[T; 2]] {
    fn into_datums(self) -> Vec<Datum> {
        self.iter().copied().map(Into::into).collect()
    }
}

// Single values (auto-enumerate) - arrays
impl<T: Numeric, const N: usize> IntoDatums for [T; N] {
    fn into_datums(self) -> Vec<Datum> {
        self.into_iter()
            .enumerate()
            .map(|(i, v)| Datum::new(i as f64, v.to_f64()))
            .collect()
    }
}

// Single values (auto-enumerate) - slices
impl<T: Numeric> IntoDatums for &[T] {
    fn into_datums(self) -> Vec<Datum> {
        self.iter()
            .enumerate()
            .map(|(i, v)| Datum::new(i as f64, v.to_f64()))
            .collect()
    }
}

// Single values (auto-enumerate) - Vec
impl<T: Numeric> IntoDatums for Vec<T> {
    fn into_datums(self) -> Vec<Datum> {
        self.into_iter()
            .enumerate()
            .map(|(i, v)| Datum::new(i as f64, v.to_f64()))
            .collect()
    }
}

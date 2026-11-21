//! Bullet charts: a measure bar over qualitative ranges, with a target tick.
//!
//! Each row is one measure (Stephen Few's bullet graph). Its ranges are the
//! shaded bands behind it, darkest first; the measure is a thin bar along the
//! row and the target a short cross tick. A chart lays the rows out as
//! horizontal bars, one category per row, so every row keeps native bar
//! hover and the row names label the category axis.

use crate::color::{Color, Pair};
use crate::data::IntoData;
use crate::data::mark::{bar, tick};
use crate::{Data, Mark};

/// One bullet graph row.
#[derive(Debug, Clone)]
pub struct Row {
    name: String,
    measure: f64,
    target: Option<f64>,
    ranges: Ranges,
}

/// The upper bounds of a row's qualitative ranges, in ascending order.
///
/// The first range starts at zero, so every bound is positive.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Ranges(Vec<f64>);

/// Why bounds cannot form [`Ranges`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Error {
    /// A bound is not finite.
    NonFinite(f64),
    /// A bound is zero or negative.
    Nonpositive(f64),
    /// A bound does not exceed the one before it.
    Unordered { previous: f64, bound: f64 },
}

/// Bullet graph rows sharing one value axis.
#[derive(Debug, Clone)]
pub struct Bullet {
    rows: Vec<Row>,
    measure: Color,
    target: Color,
}

/// Creates a bullet graph row with a name and its measured value.
pub fn row(name: impl Into<String>, measure: f64) -> Row {
    Row {
        name: name.into(),
        measure,
        target: None,
        ranges: Ranges::default(),
    }
}

/// Lays out bullet graph rows, the first at the top.
///
/// # Examples
///
/// ```
/// use hyozu::bullet::{self, Ranges};
///
/// let chart = hyozu::data(bullet::bullet([
///     bullet::row("Revenue", 270.0)
///         .target(250.0)
///         .ranges(Ranges::new(&[150.0, 225.0, 300.0]).expect("ascending bounds")),
///     bullet::row("Profit", 22.5)
///         .target(27.0)
///         .ranges(Ranges::new(&[20.0, 25.0, 30.0]).expect("ascending bounds")),
/// ]));
/// ```
pub fn bullet(rows: impl IntoIterator<Item = Row>) -> Bullet {
    Bullet {
        rows: rows.into_iter().collect(),
        measure: Color::Contrast(None),
        target: Color::Contrast(None),
    }
}

impl Row {
    /// Sets the target the measure is compared against.
    pub fn target(mut self, target: f64) -> Self {
        self.target = Some(target);
        self
    }

    /// Sets the qualitative ranges behind the measure.
    pub fn ranges(mut self, ranges: Ranges) -> Self {
        self.ranges = ranges;
        self
    }

    /// Returns the row name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the measured value.
    pub fn measure(&self) -> f64 {
        self.measure
    }

    /// Returns the target, if any.
    pub fn target_value(&self) -> Option<f64> {
        self.target
    }

    /// Returns the qualitative ranges.
    pub fn ranges_value(&self) -> &Ranges {
        &self.ranges
    }
}

impl Ranges {
    /// Validates upper bounds: finite, positive and strictly ascending.
    pub fn new(bounds: &[f64]) -> Result<Self, Error> {
        let mut previous = 0.0;
        for &bound in bounds {
            if !bound.is_finite() {
                return Err(Error::NonFinite(bound));
            }
            if bound <= 0.0 {
                return Err(Error::Nonpositive(bound));
            }
            if bound <= previous {
                return Err(Error::Unordered { previous, bound });
            }
            previous = bound;
        }
        Ok(Self(bounds.to_vec()))
    }

    /// Returns the upper bounds.
    pub fn bounds(&self) -> &[f64] {
        &self.0
    }

    /// The width of the range at `index`, or zero past the last.
    fn width(&self, index: usize) -> f64 {
        let lower = index.checked_sub(1).and_then(|i| self.0.get(i)).copied().unwrap_or(0.0);
        self.0.get(index).map_or(0.0, |upper| upper - lower)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinite(b) => write!(f, "range bound {b} is not finite"),
            Self::Nonpositive(b) => write!(f, "range bound {b} must be positive"),
            Self::Unordered { previous, bound } => {
                write!(f, "range bound {bound} must exceed the bound before it, {previous}")
            }
        }
    }
}

impl std::error::Error for Error {}

impl Bullet {
    /// Sets the measure bar color (the theme's text color by default).
    pub fn measure_color(mut self, color: impl Into<Color>) -> Self {
        self.measure = color.into();
        self
    }

    /// Sets the target tick color (the theme's text color by default).
    pub fn target_color(mut self, color: impl Into<Color>) -> Self {
        self.target = color.into();
        self
    }

    /// Returns the rows.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The marks drawing the rows: the ranges as one stacked bar per row,
    /// the measures as a thin bar over them, and the targets as ticks. Row
    /// `i` sits at category `n - 1 - i`, so the first row draws on top.
    pub fn into_marks(self) -> Vec<Mark> {
        let n = self.rows.len();
        let category = |i: usize| (n - 1 - i) as f64;
        let bands = self.rows.iter().map(|r| r.ranges.0.len()).max().unwrap_or(0);
        let range_series: Vec<bar::Series> = (0..bands)
            .map(|k| {
                let widths: Vec<(f64, f64)> = self
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(i, r)| (category(i), r.ranges.width(k)))
                    .collect();
                bar::bar(widths).with_color(Color::Themed(shade(k, bands)))
            })
            .collect();
        let measures: Vec<(f64, f64)> = self
            .rows
            .iter()
            .enumerate()
            .map(|(i, r)| (category(i), r.measure))
            .collect();
        let targets: Vec<(f64, f64)> = self
            .rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| r.target.map(|t| (category(i), t)))
            .collect();

        let mut marks = Vec::with_capacity(3);
        if !range_series.is_empty() {
            marks.push(Mark::Bars(
                bar::Bars::from_series(range_series)
                    .stacked()
                    .horizontal()
                    .with_size(0.8)
                    .data_labels(None),
            ));
        }
        marks.push(Mark::Bars(
            bar::Bars::from_series(vec![bar::bar(measures).with_color(self.measure)])
                .horizontal()
                .with_size(0.3)
                .data_labels(None),
        ));
        if !targets.is_empty() {
            marks.push(Mark::Tick(
                tick::tick(targets)
                    .vertical()
                    .color(self.target)
                    .width(3.0)
                    .length(0.55),
            ));
        }
        marks
    }
}

impl IntoData for Bullet {
    fn into_data(self) -> Data {
        let names: Vec<String> = self.rows.iter().rev().map(|r| r.name.clone()).collect();
        self.into_marks().into_data().y_axis_labels(names)
    }
}

impl From<Bullet> for Data {
    fn from(bullet: Bullet) -> Self {
        bullet.into_data()
    }
}

/// The shade of range `k` of `count`: darkest first, fading toward the
/// background on either theme.
fn shade(k: usize, count: usize) -> Pair {
    let t = if count > 1 { k as f32 / (count - 1) as f32 } else { 0.0 };
    let grey = |v: f32| crate::core::Color::from_rgb(v, v, v);
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    Pair::new(grey(lerp(0.62, 0.9)), grey(lerp(0.42, 0.2)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_must_ascend_from_zero() {
        assert!(Ranges::new(&[1.0, 2.0, 3.0]).is_ok());
        assert_eq!(Ranges::new(&[0.0]), Err(Error::Nonpositive(0.0)));
        assert_eq!(
            Ranges::new(&[2.0, 2.0]),
            Err(Error::Unordered {
                previous: 2.0,
                bound: 2.0
            })
        );
        assert!(matches!(Ranges::new(&[f64::NAN]), Err(Error::NonFinite(_))));
    }

    #[test]
    fn rows_become_stacked_ranges_a_measure_and_targets() {
        let ranges = Ranges::new(&[150.0, 225.0, 300.0]).expect("ascending");
        let marks = bullet([
            row("Revenue", 270.0).target(250.0).ranges(ranges),
            row("Orders", 80.0).ranges(Ranges::new(&[50.0, 100.0]).expect("ascending")),
        ])
        .into_marks();
        assert_eq!(marks.len(), 3);
        let Mark::Bars(bands) = &marks[0] else { panic!("ranges") };
        assert_eq!(bands.all_series().len(), 3);
        // First row on top: category 1. Widths are differences of bounds.
        let widths: Vec<_> = bands.all_series().iter().map(|s| s.points()[0].y).collect();
        assert_eq!(widths, [150.0, 75.0, 75.0]);
        assert_eq!(bands.all_series()[2].points()[1].y, 0.0, "a missing range is empty");
        assert_eq!(bands.all_series()[0].points()[0].x, 1.0);
        let Mark::Tick(targets) = &marks[2] else {
            panic!("targets")
        };
        assert_eq!(targets.points.len(), 1);
    }
}

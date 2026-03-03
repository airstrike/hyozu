use crate::color::Color;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};

/// The kind of waterfall entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    /// A positive delta (adds to running total)
    Increase,
    /// A negative delta (subtracts from running total)
    Decrease,
    /// An absolute total (resets running total)
    Total,
}

/// A single entry in a waterfall chart.
#[derive(Debug, Clone)]
pub struct Entry {
    pub(crate) value: f64,
    pub(crate) kind: EntryKind,
    pub(crate) label: Option<String>,
    pub(crate) color: Option<Color>,
}

/// Creates a waterfall entry.
pub fn entry(value: impl Into<f64>, kind: EntryKind) -> Entry {
    Entry {
        value: value.into(),
        kind,
        label: None,
        color: None,
    }
}

impl Entry {
    /// Sets a label for this entry.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the color for this entry.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }
}

/// Waterfall chart specification.
///
/// Shows how an initial value is affected by intermediate positive and negative
/// values, leading to a final value.
#[derive(Debug, Clone)]
pub struct Waterfall {
    pub(crate) entries: Vec<Entry>,
    /// Whether to draw connector lines between bars
    pub(crate) connector: bool,
}

/// Creates a waterfall chart from entries.
///
/// # Examples
///
/// ```
/// use hyozu::waterfall::{self, EntryKind::*};
///
/// let chart = waterfall::waterfall([
///     waterfall::entry(100, Total).label("Start"),
///     waterfall::entry(30, Increase).label("+Sales"),
///     waterfall::entry(-20, Decrease).label("-Costs"),
///     waterfall::entry(110, Total).label("End"),
/// ]);
/// ```
pub fn waterfall(data: impl IntoWaterfall) -> Waterfall {
    data.into_waterfall()
}

/// Trait for converting inputs into a Waterfall.
pub trait IntoWaterfall {
    fn into_waterfall(self) -> Waterfall;
}

// From array of entries
impl<const N: usize> IntoWaterfall for [Entry; N] {
    fn into_waterfall(self) -> Waterfall {
        Waterfall {
            entries: self.into(),
            connector: true,
        }
    }
}

// From Vec of entries
impl IntoWaterfall for Vec<Entry> {
    fn into_waterfall(self) -> Waterfall {
        Waterfall {
            entries: self,
            connector: true,
        }
    }
}

impl Waterfall {
    /// Sets whether to draw connector lines between bars.
    pub fn connector(mut self, show: bool) -> Self {
        self.connector = show;
        self
    }

    /// Returns the entries.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Creates the appropriate x-axis for a waterfall chart (categorical).
    pub fn x_axis() -> Axis {
        Axis::new(Orientation::Bottom)
            .with_kind(Kind::Categorical)
            .labels(Placement::OnTicks)
            .with_ticks(axis::tick::Ticks::categorical())
    }

    /// Creates the appropriate y-axis for a waterfall chart (scalar, zero-anchored).
    pub fn y_axis() -> Axis {
        Axis::new(Orientation::Left)
            .with_kind(Kind::ScalarAnchored)
            .with_ticks(axis::tick::Ticks::continuous())
    }
}

impl From<Waterfall> for crate::Data {
    fn from(waterfall: Waterfall) -> Self {
        use crate::data::IntoData;
        waterfall.into_data()
    }
}

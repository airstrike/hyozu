pub mod label;
pub mod marker;

use crate::color::Color;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};
use crate::data::{Datum, IntoDatums, Mark};

/// Trait for types that can be converted into a Line mark.
/// Used by the `lines!` macro to accept both raw data and Line builders.
pub trait IntoLines {
    fn into_lines(self) -> Mark;
}

impl IntoLines for Line {
    fn into_lines(self) -> Mark {
        Mark::Line(self)
    }
}

impl<T: IntoDatums> IntoLines for T {
    fn into_lines(self) -> Mark {
        Mark::Line(line(self))
    }
}

/// Line chart specification.
#[derive(Debug, Clone)]
pub struct Line {
    pub(crate) points: Vec<Datum>,
    pub(crate) color: Option<Color>,
    pub(crate) width: f32,
    /// Data label configuration
    pub(crate) label: Option<label::Label>,
    /// Marker configuration
    pub(crate) marker: Option<marker::Marker>,
    /// Optional name for this line (used in legends).
    pub(crate) name: Option<String>,
}

/// Creates a line chart mark from data points.
///
/// Accepts the same natural data formats as bars:
/// - Point tuples: `[(0, 100), (1, 200)]`
/// - Point arrays: `[[0, 100], [1, 200]]`
/// - Just values (auto-enumerated): `[100, 200, 300]`
///
/// # Examples
///
/// ```
/// use hyozu::line;
///
/// // Explicit points
/// let mark = line([(0.0, 2.0), (1.0, 3.0), (2.0, 4.0)]);
///
/// // Auto-enumerated values
/// let mark = line([2, 3, 4, 3, 2]);
/// ```
pub fn line(data: impl IntoDatums) -> Line {
    Line {
        points: data.into_datums(),
        color: None,
        width: 2.0,
        label: None,
        marker: None,
        name: None,
    }
}

/// Creates line marks from one or more data series.
///
/// # Examples
///
/// ```
/// use hyozu::{lines, line, data};
///
/// // Multiple series from raw data
/// let chart = data(lines![
///     [1, 2, 3, 4],
///     [2, 4, 3, 5],
/// ]);
///
/// // Multiple series with individual styling
/// let chart = data(lines![
///     line([1, 2, 3, 4]).color(0xFF5733),
///     line([2, 4, 3, 5]).color(0x33C3FF),
/// ]);
/// ```
#[macro_export]
macro_rules! lines {
    // Single line
    ($data:expr) => {
        vec![$crate::mark::line::IntoLines::into_lines($data)]
    };
    // Multiple lines
    ($($x:expr),+ $(,)?) => {
        vec![$($crate::mark::line::IntoLines::into_lines($x)),+]
    };
}

impl Line {
    /// Sets the color for the line.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the line width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the name for this line (used in legends).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Returns the name of this line.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Configure data labels for the line points.
    pub fn data_labels(mut self, label: impl Into<Option<label::Label>>) -> Self {
        self.label = label.into();
        self
    }

    /// Returns a mutable reference to the label configuration.
    pub fn label_mut(&mut self) -> Option<&mut label::Label> {
        self.label.as_mut()
    }

    /// Configure markers for the line points.
    ///
    /// # Examples
    ///
    /// ```
    /// use hyozu::line;
    /// use hyozu::line::marker::{Shape, Show};
    ///
    /// // All points with circle markers
    /// let mark = line([1, 2, 3, 4]).markers(Shape::Circle);
    ///
    /// // Only first and last with diamond markers
    /// let mark = line([1, 2, 3, 4]).markers(Shape::Diamond + Show::FirstAndLast);
    /// ```
    pub fn markers(mut self, marker: impl Into<Option<marker::Marker>>) -> Self {
        self.marker = marker.into();
        self
    }

    /// Returns a mutable reference to the marker configuration.
    pub fn marker_mut(&mut self) -> Option<&mut marker::Marker> {
        self.marker.as_mut()
    }

    // === Axis factory methods ===

    /// Creates the appropriate x-axis for a line chart.
    ///
    /// Line charts (when using auto-enumerated data) have index x-axes with:
    /// - `Kind::Index` for proper bounds (integer-aligned, no fractional padding)
    /// - Labels placed on ticks
    /// - Continuous tick style
    pub fn x_axis() -> Axis {
        Axis::new(Orientation::Bottom)
            .with_kind(Kind::Index)
            .labels(Placement::OnTicks)
            .with_ticks(axis::tick::Ticks::continuous())
    }

    /// Creates a time-based x-axis for line charts with timestamp data.
    ///
    /// - `Kind::Time` for timestamp bounds
    /// - Smart label formatting based on scale (HH:MM, Jun 15, Jun 2024, etc.)
    /// - Continuous tick style
    pub fn time_axis() -> Axis {
        Axis::new(Orientation::Bottom)
            .with_kind(Kind::Time)
            .labels(Placement::OnTicks)
            .with_ticks(axis::tick::Ticks::continuous())
    }

    /// Creates the appropriate y-axis for a line chart.
    ///
    /// Line charts have scalar y-axes with:
    /// - `Kind::Scalar` for proper bounds (padding, nice numbers)
    /// - Does NOT anchor at zero (unlike bar charts)
    /// - Continuous tick style
    pub fn y_axis() -> Axis {
        Axis::new(Orientation::Left)
            .with_kind(Kind::Scalar)
            .with_ticks(axis::tick::Ticks::continuous())
    }
}

impl From<Line> for crate::Data {
    fn from(line: Line) -> Self {
        use crate::data::IntoData;
        line.into_data()
    }
}

impl<const N: usize> From<[Line; N]> for crate::Data {
    fn from(lines: [Line; N]) -> Self {
        crate::Data::from(lines.into_iter().map(crate::Mark::Line).collect::<Vec<_>>())
    }
}

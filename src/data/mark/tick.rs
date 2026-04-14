use crate::color::Color;
use crate::data::{Datum, IntoDatums};

/// Orientation of tick mark lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Orientation {
    /// Horizontal tick lines (perpendicular to vertical bars). Default.
    #[default]
    Horizontal,
    /// Vertical tick lines (perpendicular to horizontal bars).
    Vertical,
}

/// Per-datum tick marks for annotating individual data points.
///
/// Unlike [`Rule`](super::Rule), which draws a single line spanning the entire
/// chart, a `Tick` draws short perpendicular lines at specific data coordinates,
/// scoped to each category's band. This makes them ideal for per-bar targets,
/// thresholds, or benchmarks.
///
/// # Data convention
///
/// Data points are always `(category_index, value)`, regardless of orientation.
/// The tick mark handles axis mapping internally.
///
/// # Examples
///
/// ```
/// use hyozu::tick;
///
/// // Per-bar targets for a horizontal bar chart
/// let targets = tick([(0, 80.0), (1, 95.0), (2, 75.0)])
///     .vertical()
///     .color(0xD4A843);
/// ```
#[derive(Debug, Clone)]
pub struct Tick {
    /// Data points: (category_index, value) for each tick.
    pub(crate) points: Vec<Datum>,
    /// Tick line orientation.
    pub(crate) orientation: Orientation,
    /// Optional line color (defaults to text color).
    pub(crate) color: Option<Color>,
    /// Stroke width in pixels (default 2.0).
    pub(crate) width: f32,
    /// Length as proportion of category band (default 0.75).
    pub(crate) length: f32,
}

/// Creates a tick mark from data points.
///
/// Each data point is `(category_index, value)`.
///
/// # Examples
///
/// ```
/// use hyozu::tick;
///
/// // Tick marks at specific category/value positions
/// let t = tick([(0, 80.0), (1, 95.0), (2, 75.0)]);
/// ```
pub fn tick(data: impl IntoDatums) -> Tick {
    Tick {
        points: data.into_datums(),
        orientation: Orientation::default(),
        color: None,
        width: 2.0,
        length: 0.75,
    }
}

impl Tick {
    /// Sets horizontal orientation (default). For vertical bar charts.
    pub fn horizontal(mut self) -> Self {
        self.orientation = Orientation::Horizontal;
        self
    }

    /// Sets vertical orientation. For horizontal bar charts.
    pub fn vertical(mut self) -> Self {
        self.orientation = Orientation::Vertical;
        self
    }

    /// Sets the line color.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the stroke width in pixels (default 2.0).
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the tick length as a proportion of the category band (default 0.75).
    pub fn length(mut self, length: f32) -> Self {
        self.length = length.clamp(0.05, 1.0);
        self
    }

    /// Tick marks inherit axes from the chart they overlay.
    pub fn x_axis() -> Option<crate::data::Axis> {
        None
    }

    /// Tick marks inherit axes from the chart they overlay.
    pub fn y_axis() -> Option<crate::data::Axis> {
        None
    }
}

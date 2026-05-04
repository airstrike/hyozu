use crate::color::Color;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};

pub use super::bar::Direction;

/// A single box plot entry with pre-computed or raw statistics.
#[derive(Debug, Clone)]
pub struct Entry {
    pub(crate) min: f64,
    pub(crate) q1: f64,
    pub(crate) median: f64,
    pub(crate) q3: f64,
    pub(crate) max: f64,
    pub(crate) outliers: Vec<f64>,
    pub(crate) name: Option<String>,
    pub(crate) color: Option<Color>,
}

/// Creates a box plot entry from pre-computed statistics.
pub fn entry(min: f64, q1: f64, median: f64, q3: f64, max: f64) -> Entry {
    Entry {
        min,
        q1,
        median,
        q3,
        max,
        outliers: Vec::new(),
        name: None,
        color: None,
    }
}

/// Creates a box plot entry by computing statistics from raw data.
pub fn entry_from_data(values: &[f64]) -> Entry {
    if values.is_empty() {
        return entry(0.0, 0.0, 0.0, 0.0, 0.0);
    }

    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let median = percentile(&sorted, 50.0);
    let q1 = percentile(&sorted, 25.0);
    let q3 = percentile(&sorted, 75.0);
    let iqr = q3 - q1;

    let lower_fence = q1 - 1.5 * iqr;
    let upper_fence = q3 + 1.5 * iqr;

    let n = sorted.len();
    let min = sorted.iter().copied().find(|&v| v >= lower_fence).unwrap_or(sorted[0]);
    let max = sorted
        .iter()
        .rev()
        .copied()
        .find(|&v| v <= upper_fence)
        .unwrap_or(sorted[n - 1]);

    let outliers: Vec<f64> = sorted
        .iter()
        .copied()
        .filter(|&v| v < lower_fence || v > upper_fence)
        .collect();

    Entry {
        min,
        q1,
        median,
        q3,
        max,
        outliers,
        name: None,
        color: None,
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }

    let k = (p / 100.0) * (sorted.len() - 1) as f64;
    let f = k.floor() as usize;
    let c = (f + 1).min(sorted.len() - 1);
    let d = k - f as f64;

    sorted[f] + d * (sorted[c] - sorted[f])
}

impl Entry {
    /// Sets the name for this entry (used in legends and axis labels).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the color for this entry.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets explicit outlier values for this entry.
    pub fn with_outliers(mut self, outliers: impl Into<Vec<f64>>) -> Self {
        self.outliers = outliers.into();
        self
    }

    /// Returns the name of this entry, if set.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

/// A box plot specification showing statistical distributions.
#[derive(Debug, Clone)]
pub struct BoxPlot {
    pub(crate) entries: Vec<Entry>,
    pub(crate) direction: Direction,
    /// Box width proportion (like bar Size), default 0.6
    pub(crate) width: f32,
}

/// Creates a box plot from entries.
pub fn boxplot(entries: impl Into<Vec<Entry>>) -> BoxPlot {
    BoxPlot {
        entries: entries.into(),
        direction: Direction::default(),
        width: 0.6,
    }
}

impl BoxPlot {
    /// Sets the direction to horizontal.
    pub fn horizontal(mut self) -> Self {
        self.direction = Direction::Horizontal;
        self
    }

    /// Sets the direction to vertical.
    pub fn vertical(mut self) -> Self {
        self.direction = Direction::Vertical;
        self
    }

    /// Sets the box width proportion (clamped to [0.1, 1.0]).
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width.clamp(0.1, 1.0);
        self
    }

    /// Returns the direction of the box plot.
    pub fn direction(&self) -> Direction {
        self.direction
    }

    /// Returns the entries.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Creates the appropriate x-axis for this box plot.
    pub fn x_axis(&self) -> Axis {
        match self.direction {
            Direction::Vertical => Axis::new(Orientation::Bottom)
                .with_kind(Kind::Categorical)
                .labels(Placement::OnTicks)
                .with_ticks(axis::tick::Ticks::categorical()),
            Direction::Horizontal => Axis::new(Orientation::Bottom)
                .with_kind(Kind::Scalar)
                .with_ticks(axis::tick::Ticks::continuous()),
        }
    }

    /// Creates the appropriate y-axis for this box plot.
    pub fn y_axis(&self) -> Axis {
        match self.direction {
            Direction::Vertical => Axis::new(Orientation::Left)
                .with_kind(Kind::Scalar)
                .with_ticks(axis::tick::Ticks::continuous()),
            Direction::Horizontal => Axis::new(Orientation::Left)
                .with_kind(Kind::Categorical)
                .labels(Placement::OnTicks)
                .with_ticks(axis::tick::Ticks::categorical()),
        }
    }
}

impl<Message, Theme, Renderer> From<BoxPlot> for crate::Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(bp: BoxPlot) -> Self {
        <BoxPlot as crate::data::IntoData<Message, Theme, Renderer>>::into_data(bp)
    }
}

// Allow creating from array of entries
impl<const N: usize> From<[Entry; N]> for BoxPlot {
    fn from(entries: [Entry; N]) -> Self {
        boxplot(entries.to_vec())
    }
}

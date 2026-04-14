use crate::color::Color;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};

pub use super::bar::Direction;

/// Optional box plot statistics for overlay.
#[derive(Debug, Clone)]
pub struct BoxStats {
    pub(crate) min: f64,
    pub(crate) q1: f64,
    pub(crate) median: f64,
    pub(crate) q3: f64,
    pub(crate) max: f64,
}

impl BoxStats {
    pub fn new(min: f64, q1: f64, median: f64, q3: f64, max: f64) -> Self {
        Self {
            min,
            q1,
            median,
            q3,
            max,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// Density curve points: (value, density) pairs, sorted by value
    pub(crate) density: Vec<(f64, f64)>,
    /// Optional box plot stats overlay
    pub(crate) stats: Option<BoxStats>,
    pub(crate) name: Option<String>,
    pub(crate) color: Option<Color>,
}

/// Creates a violin entry from pre-computed density points.
pub fn violin_entry(density: impl Into<Vec<(f64, f64)>>) -> Entry {
    Entry {
        density: density.into(),
        stats: None,
        name: None,
        color: None,
    }
}

/// Creates a violin entry by computing KDE from raw data.
pub fn violin_from_data(values: &[f64]) -> Entry {
    if values.is_empty() {
        return violin_entry(vec![]);
    }

    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let n = sorted.len();
    let min = sorted[0];
    let max = sorted[n - 1];

    // Compute bandwidth using Silverman's rule of thumb
    let mean: f64 = sorted.iter().sum::<f64>() / n as f64;
    let variance: f64 = sorted.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
    let std_dev = variance.sqrt();
    let bandwidth = 1.06 * std_dev * (n as f64).powf(-0.2);
    let bandwidth = if bandwidth <= 0.0 { 1.0 } else { bandwidth };

    // Extend range slightly beyond data
    let range = max - min;
    let extend = range * 0.1 + bandwidth;
    let kde_min = min - extend;
    let kde_max = max + extend;

    // Evaluate KDE at regular intervals
    let num_points = 64;
    let step = (kde_max - kde_min) / (num_points - 1) as f64;

    let density: Vec<(f64, f64)> = (0..num_points)
        .map(|i| {
            let x = kde_min + i as f64 * step;
            let density = sorted
                .iter()
                .map(|&xi| gaussian_kernel((x - xi) / bandwidth))
                .sum::<f64>()
                / (n as f64 * bandwidth);
            (x, density)
        })
        .collect();

    // Compute box stats
    let median = percentile(&sorted, 50.0);
    let q1 = percentile(&sorted, 25.0);
    let q3 = percentile(&sorted, 75.0);

    Entry {
        density,
        stats: Some(BoxStats::new(min, q1, median, q3, max)),
        name: None,
        color: None,
    }
}

fn gaussian_kernel(x: f64) -> f64 {
    (-(x * x) / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
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
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn with_stats(mut self, min: f64, q1: f64, median: f64, q3: f64, max: f64) -> Self {
        self.stats = Some(BoxStats::new(min, q1, median, q3, max));
        self
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }
}

#[derive(Debug, Clone)]
pub struct Violin {
    pub(crate) entries: Vec<Entry>,
    pub(crate) direction: Direction,
    pub(crate) width: f32,     // max width proportion, default 0.8
    pub(crate) show_box: bool, // overlay mini box plot
}

/// Creates a violin chart from entries.
pub fn violin(entries: impl Into<Vec<Entry>>) -> Violin {
    Violin {
        entries: entries.into(),
        direction: Direction::default(),
        width: 0.8,
        show_box: true,
    }
}

impl Violin {
    pub fn horizontal(mut self) -> Self {
        self.direction = Direction::Horizontal;
        self
    }

    pub fn vertical(mut self) -> Self {
        self.direction = Direction::Vertical;
        self
    }

    pub fn show_box(mut self, show: bool) -> Self {
        self.show_box = show;
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width.clamp(0.1, 1.0);
        self
    }

    pub fn direction(&self) -> Direction {
        self.direction
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    // Axis factory methods
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

impl From<Violin> for crate::Data {
    fn from(v: Violin) -> Self {
        use crate::data::IntoData;
        v.into_data()
    }
}

impl<const N: usize> From<[Entry; N]> for Violin {
    fn from(entries: [Entry; N]) -> Self {
        violin(entries.to_vec())
    }
}

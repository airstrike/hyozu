use std::sync::Arc;

use crate::data::axis::{self, Axis, Kind, Orientation, Placement};

/// A heatmap mark: a 2D grid where each cell is colored by value.
///
/// Both axes are categorical. Values are stored in row-major order.
#[derive(Clone)]
pub struct Heatmap {
    pub(crate) values: Vec<f64>,
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    pub(crate) row_names: Vec<String>,
    pub(crate) col_names: Vec<String>,
    pub(crate) show_labels: bool,
    pub(crate) label_format: Arc<dyn Fn(f64) -> String + Send + Sync>,
    pub(crate) value_range: Option<(f64, f64)>,
    /// Gradient stops for color mapping. If empty, auto-selected based on data.
    pub(crate) color_stops: Vec<crate::core::Color>,
}

impl std::fmt::Debug for Heatmap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Heatmap")
            .field("values", &self.values)
            .field("rows", &self.rows)
            .field("cols", &self.cols)
            .field("row_names", &self.row_names)
            .field("col_names", &self.col_names)
            .field("show_labels", &self.show_labels)
            .field("label_format", &"<function>")
            .field("value_range", &self.value_range)
            .field("color_stops", &self.color_stops)
            .finish()
    }
}

// === Built-in color scales ===

use crate::core::Color as IcedColor;

/// Blue → near-white → red. Good for correlation matrices and data centered on zero.
pub fn divergent_stops() -> Vec<IcedColor> {
    vec![
        IcedColor::from_rgb8(0x33, 0x66, 0xAA), // steel blue
        IcedColor::from_rgb8(0x88, 0xBB, 0xDD), // light blue
        IcedColor::from_rgb8(0xEE, 0xEE, 0xEE), // near-white
        IcedColor::from_rgb8(0xDD, 0x88, 0x66), // salmon
        IcedColor::from_rgb8(0xBB, 0x33, 0x33), // brick red
    ]
}

/// Light → dark single hue. Good for magnitudes and counts.
pub fn sequential_stops() -> Vec<IcedColor> {
    vec![
        IcedColor::from_rgb8(0xEE, 0xF0, 0xF8), // very light blue-gray
        IcedColor::from_rgb8(0x9E, 0xBC, 0xDB), // mid blue
        IcedColor::from_rgb8(0x33, 0x66, 0xAA), // steel blue
        IcedColor::from_rgb8(0x18, 0x3D, 0x7A), // dark blue
    ]
}

/// Warm sequential: cream → amber → deep brown.
pub fn warm_stops() -> Vec<IcedColor> {
    vec![
        IcedColor::from_rgb8(0xFD, 0xF0, 0xD5), // cream
        IcedColor::from_rgb8(0xF4, 0xBB, 0x6A), // amber
        IcedColor::from_rgb8(0xD4, 0x6B, 0x22), // burnt orange
        IcedColor::from_rgb8(0x7C, 0x2D, 0x12), // deep brown
    ]
}

/// Creates a heatmap from row-major values with given dimensions.
///
/// Defaults to a sequential (blue) color scale. Use `.divergent()` for data
/// centered on zero, or `.color_stops(stops)` for a custom gradient.
pub fn heatmap(values: impl Into<Vec<f64>>, rows: usize, cols: usize) -> Heatmap {
    Heatmap {
        values: values.into(),
        rows,
        cols,
        row_names: Vec::new(),
        col_names: Vec::new(),
        show_labels: false,
        label_format: Arc::new(|v| format!("{v:.1}")),
        value_range: None,
        color_stops: sequential_stops(),
    }
}

impl From<Vec<Vec<f64>>> for Heatmap {
    fn from(data: Vec<Vec<f64>>) -> Self {
        let rows = data.len();
        let cols = if rows > 0 { data[0].len() } else { 0 };
        let values: Vec<f64> = data.into_iter().flatten().collect();
        heatmap(values, rows, cols)
    }
}

impl<const R: usize, const C: usize> From<[[f64; C]; R]> for Heatmap {
    fn from(data: [[f64; C]; R]) -> Self {
        let values: Vec<f64> = data.iter().flatten().copied().collect();
        heatmap(values, R, C)
    }
}

impl Heatmap {
    /// Set row names (categorical labels for the Y axis).
    pub fn row_names(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.row_names = names.into_iter().map(Into::into).collect();
        self
    }

    /// Set column names (categorical labels for the X axis).
    pub fn col_names(mut self, names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.col_names = names.into_iter().map(Into::into).collect();
        self
    }

    /// Enable or disable value labels inside cells.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Set the format function for cell labels.
    pub fn label_format(mut self, f: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.label_format = Arc::new(f);
        self
    }

    /// Set explicit value range for color mapping.
    /// If not set, the range is auto-computed from data.
    pub fn value_range(mut self, min: f64, max: f64) -> Self {
        self.value_range = Some((min, max));
        self
    }

    /// Use a divergent color scale (blue → white → red).
    /// Best for data centered on zero like correlation matrices.
    pub fn divergent(mut self) -> Self {
        self.color_stops = divergent_stops();
        self
    }

    /// Use a sequential color scale (light → dark blue).
    /// Best for magnitudes, counts, and non-negative data. This is the default.
    pub fn sequential(mut self) -> Self {
        self.color_stops = sequential_stops();
        self
    }

    /// Use a warm sequential color scale (cream → amber → brown).
    pub fn warm(mut self) -> Self {
        self.color_stops = warm_stops();
        self
    }

    /// Use custom gradient stops for color mapping.
    /// Colors are interpolated in OKLch space between stops.
    pub fn color_stops(mut self, stops: impl Into<Vec<crate::core::Color>>) -> Self {
        self.color_stops = stops.into();
        self
    }

    /// Returns the current color stops.
    pub fn get_color_stops(&self) -> &[crate::core::Color] {
        &self.color_stops
    }

    /// Get value at (row, col).
    pub fn get(&self, row: usize, col: usize) -> f64 {
        self.values[row * self.cols + col]
    }

    /// Number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Compute value range (auto from data or explicit).
    pub fn compute_range(&self) -> (f64, f64) {
        if let Some(range) = self.value_range {
            return range;
        }
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for &v in &self.values {
            min = min.min(v);
            max = max.max(v);
        }
        if min.is_infinite() {
            min = 0.0;
        }
        if max.is_infinite() {
            max = 1.0;
        }
        if (max - min).abs() < f64::EPSILON {
            max = min + 1.0;
        }
        (min, max)
    }

    /// Default X axis for heatmap (categorical, bottom).
    pub fn x_axis(&self) -> Axis {
        let mut axis = Axis::new(Orientation::Bottom)
            .with_kind(Kind::Categorical)
            .with_ticks(axis::tick::Ticks::categorical());
        if !self.col_names.is_empty() {
            axis = axis
                .labels(Placement::BetweenTicks)
                .labels(self.col_names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        }
        axis
    }

    /// Default Y axis for heatmap (categorical, left).
    pub fn y_axis(&self) -> Axis {
        let mut axis = Axis::new(Orientation::Left)
            .with_kind(Kind::Categorical)
            .with_ticks(axis::tick::Ticks::categorical());
        if !self.row_names.is_empty() {
            axis = axis
                .labels(Placement::BetweenTicks)
                .labels(self.row_names.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        }
        axis
    }
}

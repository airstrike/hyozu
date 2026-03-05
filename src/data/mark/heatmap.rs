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
            .finish()
    }
}

/// Creates a heatmap from row-major values with given dimensions.
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

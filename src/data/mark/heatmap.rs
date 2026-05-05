use std::sync::Arc;

use crate::data::axis::{self, Axis, Kind, Orientation, Placement};
use crate::palette::Palette;
use crate::scale::ColorScale;

/// A heatmap mark: a 2D grid where each cell is colored by value.
///
/// Both axes are categorical. Values are stored in row-major order. Color
/// mapping is described by [`ColorScale<f64>`] (domain + palette +
/// transform + format); the default linear transform with no explicit
/// domain or palette preserves the historical heatmap behavior of
/// auto-inferring the value range and using a sequential blue gradient.
#[derive(Clone)]
pub struct Heatmap {
    pub(crate) values: Vec<f64>,
    pub(crate) rows: usize,
    pub(crate) cols: usize,
    pub(crate) row_names: Vec<String>,
    pub(crate) col_names: Vec<String>,
    pub(crate) show_labels: bool,
    pub(crate) label_format: Arc<dyn Fn(f64) -> String + Send + Sync>,
    /// Color encoding scale: domain (auto-inferred when `None`), palette
    /// (mark default when `None`), transform, and optional legend tick
    /// formatter. Default is [`ColorScale::default()`] — linear, no
    /// domain override, falling back to [`sequential_stops`].
    pub(crate) color: ColorScale<f64>,
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
            .field("color", &self.color)
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

/// Wrap a `Vec<core::Color>` of raw RGB stops as a [`Palette::Gradient`].
/// Used by the preset builders (`.divergent()`, `.warm()`, etc.) to feed
/// the new `ColorScale.palette` slot without leaking the wrapper-color
/// type into the public surface.
fn palette_from_stops(stops: Vec<IcedColor>) -> Palette {
    Palette::Gradient(stops.into_iter().map(Into::into).collect())
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
        color: ColorScale::default(),
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

    /// Use a divergent color scale (blue → white → red).
    /// Best for data centered on zero like correlation matrices.
    pub fn divergent(mut self) -> Self {
        self.color.palette = Some(palette_from_stops(divergent_stops()));
        self
    }

    /// Use a sequential color scale (light → dark blue).
    /// Best for magnitudes, counts, and non-negative data. This is the default.
    pub fn sequential(mut self) -> Self {
        self.color.palette = Some(palette_from_stops(sequential_stops()));
        self
    }

    /// Use a warm sequential color scale (cream → amber → brown).
    pub fn warm(mut self) -> Self {
        self.color.palette = Some(palette_from_stops(warm_stops()));
        self
    }

    /// Use custom gradient stops for color mapping.
    /// Colors are interpolated in OKLch space between stops.
    pub fn color_stops(mut self, stops: impl Into<Vec<crate::core::Color>>) -> Self {
        self.color.palette = Some(palette_from_stops(stops.into()));
        self
    }

    /// Replaces the entire color encoding scale (domain + palette +
    /// transform + format) in one go. Use this when you've built a
    /// [`ColorScale<f64>`] elsewhere (e.g. shared across marks).
    pub fn color_scale(mut self, scale: ColorScale<f64>) -> Self {
        self.color = scale;
        self
    }

    /// Sets the explicit value domain `(lo, hi)` on the color scale.
    /// Overrides the data-derived auto-inferred range at draw time.
    pub fn color_domain(mut self, lo: f64, hi: f64) -> Self {
        self.color.domain = Some((lo, hi));
        self
    }

    /// Switches the color scale's transform to linear (the default).
    pub fn linear(mut self) -> Self {
        self.color.transform = crate::scale::Transform::Linear;
        self
    }

    /// Switches the color scale's transform to square root. Useful for
    /// moderately skewed numeric distributions.
    pub fn sqrt(mut self) -> Self {
        self.color.transform = crate::scale::Transform::Sqrt;
        self
    }

    /// Switches the color scale's transform to logarithmic. Useful for
    /// data spanning orders of magnitude.
    pub fn log(mut self) -> Self {
        self.color.transform = crate::scale::Transform::Log;
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

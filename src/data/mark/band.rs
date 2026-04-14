use crate::color::Color;

/// Orientation of a reference band.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandOrientation {
    /// A shaded horizontal strip spanning a Y-value range (the common case).
    Horizontal,
    /// A shaded vertical strip spanning an X-value range.
    Vertical,
}

/// A shaded reference band (value range) overlaid on a chart.
///
/// Bands draw translucent rectangles across the plot area between `lower`
/// and `upper` values on one axis and the full extent of the other axis.
/// Typical uses include "acceptable range" bands, control limits, or
/// highlighted time windows.
#[derive(Debug, Clone)]
pub struct Band {
    pub(crate) lower: f64,
    pub(crate) upper: f64,
    pub(crate) orientation: BandOrientation,
    pub(crate) color: Option<Color>,
    pub(crate) opacity: f32,
    pub(crate) label: Option<String>,
}

/// Creates a horizontal reference band between `lower` and `upper` Y values.
///
/// Switch to a vertical (X-range) band with `.vertical()`.
///
/// # Examples
///
/// ```
/// use hyozu::band;
///
/// // Acceptable temperature range
/// let ok_range = band(18.0, 24.0);
/// ```
pub fn band(lower: impl Into<f64>, upper: impl Into<f64>) -> Band {
    let lower = lower.into();
    let upper = upper.into();
    Band {
        lower: lower.min(upper),
        upper: lower.max(upper),
        orientation: BandOrientation::Horizontal,
        color: None,
        opacity: 0.2,
        label: None,
    }
}

impl Band {
    /// Sets the orientation to horizontal (default) — band spans Y range.
    pub fn horizontal(mut self) -> Self {
        self.orientation = BandOrientation::Horizontal;
        self
    }

    /// Sets the orientation to vertical — band spans X range.
    pub fn vertical(mut self) -> Self {
        self.orientation = BandOrientation::Vertical;
        self
    }

    /// Sets the fill color for this band.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the fill opacity (clamped to `[0.0, 1.0]`).
    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Sets an optional label for this band.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Returns the lower bound of the band.
    pub fn lower(&self) -> f64 {
        self.lower
    }

    /// Returns the upper bound of the band.
    pub fn upper(&self) -> f64 {
        self.upper
    }

    /// Returns the orientation.
    pub fn orientation(&self) -> BandOrientation {
        self.orientation
    }

    /// Bands inherit their axes from the chart they overlay.
    pub fn x_axis() -> Option<crate::data::Axis> {
        None
    }

    /// Bands inherit their axes from the chart they overlay.
    pub fn y_axis() -> Option<crate::data::Axis> {
        None
    }
}

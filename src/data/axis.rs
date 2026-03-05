use crate::color::Color;
use crate::core::{Font, Pixels};

pub mod label;
pub mod tick;

pub use label::{Labels, Placement};
pub use tick::{Alignment, Ticks};

/// Orientation of an axis relative to the data area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Bottom,
    Left,
    Top,
    Right,
}

/// The semantic kind of an axis - determines bounds computation and formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Kind {
    /// Continuous numeric values that should fit the data range.
    /// - Bounds: data range + padding, snapped to nice round numbers
    /// - No zero-anchoring (use ScalarAnchored for that)
    /// - Best for: line charts, scatter plots, time series
    #[default]
    Scalar,

    /// Continuous numeric values anchored at zero.
    /// - Bounds: 0 to nice max (for non-negative data)
    /// - Best for: bar charts where bars must start from baseline
    ScalarAnchored,

    /// Discrete categories with no numeric relationship
    /// - Bounds: position ± 0.5 for centering
    /// - No padding beyond centering
    Categorical,

    /// Integer indices or counts (e.g., sample numbers 0, 1, 2, ...)
    /// - Bounds: 0 to ceil(max), integer-aligned
    /// - No fractional padding
    Index,

    /// Unix timestamps (seconds since epoch as f32).
    /// - Bounds: exact data range (no padding for time)
    /// - Ticks snap to nice time intervals (1min, 5min, 15min, 1hr, etc.)
    /// - Best for: time series with actual timestamps
    Time,
}

impl Kind {
    /// Compute normalized axis bounds from a data range.
    ///
    /// This is the primary way to create `Bounds` - it applies kind-appropriate
    /// transformations (padding, nice numbers, zero-anchoring) automatically.
    pub fn bounds(self, data_min: f64, data_max: f64) -> Bounds {
        match self {
            Kind::Scalar => {
                let range = data_max - data_min;
                if range <= 0.0 {
                    return Bounds {
                        min: data_min - 1.0,
                        max: data_max + 1.0,
                    };
                }

                // Add 5% padding
                let padded_min = data_min - range * 0.05;
                let padded_max = data_max + range * 0.05;

                // Snap to nice round numbers
                let padded_range = padded_max - padded_min;
                let step = nice_step(padded_range, 5);
                let mut nice_min = (padded_min / step).floor() * step;
                let nice_max = (padded_max / step).ceil() * step;

                // Don't cross zero if original data didn't
                if data_min >= 0.0 && nice_min < 0.0 {
                    nice_min = 0.0;
                }

                Bounds {
                    min: nice_min,
                    max: nice_max,
                }
            }
            Kind::ScalarAnchored => {
                let range = data_max - data_min;
                if range <= 0.0 {
                    return Bounds {
                        min: 0.0,
                        max: data_max + 1.0,
                    };
                }

                // Add 5% padding on top only
                let padded_max = data_max + range * 0.05;

                // Snap to nice round numbers
                let step = nice_step(padded_max, 5);
                let nice_max = (padded_max / step).ceil() * step;

                // Zero anchor for non-negative, otherwise compute nice min
                let final_min = if data_min >= 0.0 {
                    0.0
                } else {
                    let padded_min = data_min - range * 0.05;
                    (padded_min / step).floor() * step
                };

                Bounds {
                    min: final_min,
                    max: nice_max,
                }
            }
            Kind::Categorical => {
                // ±0.5 centering for categories
                Bounds {
                    min: data_min - 0.5,
                    max: data_max + 0.5,
                }
            }
            Kind::Index => {
                // Integer alignment, start from 0 if near 0
                Bounds {
                    min: data_min.floor().min(0.0),
                    max: data_max.ceil(),
                }
            }
            Kind::Time => {
                // Exact bounds for time - no padding, just the data range
                // Ticks will snap to nice time intervals separately
                Bounds {
                    min: data_min,
                    max: data_max,
                }
            }
        }
    }
}

/// Axis bounds - an opaque type representing the visual range of an axis.
///
/// Create bounds through `Kind::bounds()` which applies appropriate normalization:
/// ```ignore
/// let bounds = Kind::Scalar.bounds(data_min, data_max);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    min: f64,
    max: f64,
}

impl Bounds {
    /// Create exact bounds without normalization.
    /// Use for user-specified explicit bounds that shouldn't be adjusted.
    pub(crate) fn exact(min: f64, max: f64) -> Self {
        Self { min, max }
    }

    /// Get the minimum bound.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Get the maximum bound.
    pub fn max(&self) -> f64 {
        self.max
    }

    /// Convert to tuple.
    pub fn as_tuple(&self) -> (f64, f64) {
        (self.min, self.max)
    }
}

/// Compute a nice step size for tick marks.
fn nice_step(range: f64, target_count: usize) -> f64 {
    let rough_step = range / (target_count as f64);
    let magnitude = 10_f64.powf(rough_step.log10().floor());
    let normalized = rough_step / magnitude;

    if normalized <= 1.5 {
        magnitude
    } else if normalized <= 3.0 {
        2.0 * magnitude
    } else if normalized <= 7.0 {
        5.0 * magnitude
    } else {
        10.0 * magnitude
    }
}

/// Axis guide that generates tick marks, labels, grid lines, and axis line.
#[derive(Debug, Clone)]
pub struct Axis {
    orientation: Orientation,
    kind: Kind,

    // Tick configuration
    pub(crate) ticks: tick::Ticks,
    pub(crate) labels: label::Labels,

    // Bounds (if set, override calculated bounds)
    pub(crate) lower_bound: Option<f64>,
    pub(crate) upper_bound: Option<f64>,

    // Display options
    title: Option<String>,
    show_line: bool,
    show_labels: bool,
    show_grid: bool,

    // Style overrides
    axis_color: Option<Color>,
    label_color: Option<Color>,
    grid_color: Option<Color>,
    font: Option<Font>,
    label_size: Option<Pixels>,
}

impl Axis {
    /// Create a new axis with the given orientation.
    ///
    /// All styling is optional and will fallback to the design system if not specified.
    pub fn new(orientation: Orientation) -> Self {
        Self {
            orientation,
            kind: Kind::default(),
            ticks: tick::Ticks::default(),
            labels: label::Labels::default(),
            lower_bound: None,
            upper_bound: None,
            title: None,
            show_line: true,
            show_labels: true,
            show_grid: true,
            axis_color: None,
            label_color: None,
            grid_color: None,
            font: None,
            label_size: None,
        }
    }

    /// Set the axis kind.
    pub fn with_kind(mut self, kind: Kind) -> Self {
        self.kind = kind;
        self
    }

    /// Returns the kind of this axis.
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// Set tick configuration.
    pub fn with_ticks(mut self, ticks: impl Into<tick::Ticks>) -> Self {
        self.ticks = ticks.into();
        self
    }

    /// Set custom labels for this axis.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// // Categorical labels
    /// axis.labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
    ///
    /// // With placement
    /// axis.labels(Placement::BetweenTicks + ["Jan", "Feb", "Mar"])
    ///
    /// // Formatted numeric labels
    /// axis.labels(|v| format!("${:.0}", v))
    /// ```
    pub fn labels(mut self, labels: impl Into<label::Labels>) -> Self {
        let new_labels = labels.into();
        // Merge with existing labels to preserve any previously set properties
        if new_labels.placement.is_some() {
            self.labels.placement = new_labels.placement;
        }
        if new_labels.values.is_some() {
            self.labels.values = new_labels.values;
        }
        if new_labels.format.is_some() {
            self.labels.format = new_labels.format;
        }
        self
    }

    /// Set label format function (deprecated - use labels() instead).
    pub fn with_label_format(mut self, format: fn(f64) -> String) -> Self {
        // Convert to Labels format for backward compatibility
        use std::sync::Arc;
        self.labels.format = Some(Arc::new(format));
        self
    }

    /// Set bounds for this axis.
    ///
    /// Pass `None` to auto-scale that bound from data.
    pub fn with_bounds(mut self, lower: impl Into<Option<f64>>, upper: impl Into<Option<f64>>) -> Self {
        self.lower_bound = lower.into();
        self.upper_bound = upper.into();
        self
    }

    /// Set the axis line color (overrides design system).
    pub fn with_axis_color(mut self, color: impl Into<Color>) -> Self {
        self.axis_color = Some(color.into());
        self
    }

    /// Set the label text color (overrides design system).
    pub fn with_label_color(mut self, color: impl Into<Color>) -> Self {
        self.label_color = Some(color.into());
        self
    }

    /// Set the grid line color (overrides design system).
    pub fn with_grid_color(mut self, color: impl Into<Color>) -> Self {
        self.grid_color = Some(color.into());
        self
    }

    /// Set the font (overrides design system).
    pub fn with_font(mut self, font: Font) -> Self {
        self.font = Some(font);
        self
    }

    /// Set the label size (overrides design system).
    pub fn with_label_size(mut self, size: impl Into<Pixels>) -> Self {
        self.label_size = Some(size.into());
        self
    }

    /// Check if grid is enabled.
    pub fn has_grid(&self) -> bool {
        self.show_grid
    }

    /// Show or hide the axis line.
    pub fn show_line(mut self, show: bool) -> Self {
        self.show_line = show;
        self
    }

    /// Show or hide tick labels.
    pub fn show_labels(mut self, show: bool) -> Self {
        self.show_labels = show;
        self
    }

    /// Show or hide grid lines.
    pub fn show_grid(mut self, show: bool) -> Self {
        self.show_grid = show;
        self
    }

    /// Set the axis title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Check if tick marks are enabled.
    pub(crate) fn has_ticks(&self) -> bool {
        self.ticks.style != tick::Style::None
    }

    /// Check if tick labels are enabled.
    pub(crate) fn has_labels(&self) -> bool {
        self.show_labels
    }

    /// Get the font, if specified.
    pub(crate) fn font(&self) -> Option<Font> {
        self.font
    }

    /// Get the label size, if specified.
    pub(crate) fn label_size(&self) -> Option<Pixels> {
        self.label_size
    }

    /// Get the axis color, if specified.
    pub(crate) fn axis_color(&self) -> Option<Color> {
        self.axis_color
    }

    /// Get the label color, if specified.
    pub(crate) fn label_color(&self) -> Option<Color> {
        self.label_color
    }

    /// Returns the label placement (on ticks or between ticks).
    pub fn placement(&self) -> Option<Placement> {
        self.labels.placement
    }

    /// Returns the orientation of this axis.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Returns whether this axis is categorical.
    pub fn is_categorical(&self) -> bool {
        self.kind == Kind::Categorical
    }

    /// Returns whether labels are shown.
    pub fn shows_labels(&self) -> bool {
        self.show_labels
    }

    /// Returns whether the axis line is shown.
    pub fn shows_line(&self) -> bool {
        self.show_line
    }

    /// Returns whether grid lines are shown.
    pub fn shows_grid(&self) -> bool {
        self.show_grid
    }

    /// Returns a mutable reference to the tick configuration.
    pub fn ticks_mut(&mut self) -> &mut tick::Ticks {
        &mut self.ticks
    }
}

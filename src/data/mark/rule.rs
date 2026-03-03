use crate::color::Color;

/// Orientation of a rule line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleOrientation {
    /// A horizontal line at a Y value
    Horizontal,
    /// A vertical line at an X value
    Vertical,
}

/// A reference line (rule) mark overlaid on a chart.
///
/// Rules draw horizontal or vertical lines at specified data values,
/// useful for thresholds, targets, or annotations.
#[derive(Debug, Clone)]
pub struct Rule {
    pub(crate) value: f64,
    pub(crate) orientation: RuleOrientation,
    pub(crate) color: Option<Color>,
    pub(crate) width: f32,
    pub(crate) label: Option<String>,
}

/// Creates a horizontal rule at the given Y value.
///
/// # Examples
///
/// ```
/// use hyozu::rule;
///
/// let target = rule(1500.0);
/// ```
pub fn rule(value: impl Into<f64>) -> Rule {
    Rule {
        value: value.into(),
        orientation: RuleOrientation::Horizontal,
        color: None,
        width: 1.5,
        label: None,
    }
}

impl Rule {
    /// Sets the orientation to horizontal (default).
    pub fn horizontal(mut self) -> Self {
        self.orientation = RuleOrientation::Horizontal;
        self
    }

    /// Sets the orientation to vertical.
    pub fn vertical(mut self) -> Self {
        self.orientation = RuleOrientation::Vertical;
        self
    }

    /// Sets the line color.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the line width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets a text label for this rule.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Returns the value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the orientation.
    pub fn orientation(&self) -> RuleOrientation {
        self.orientation
    }

    /// Rules use the axes of the chart they're overlaid on, so they don't define their own.
    pub fn x_axis() -> Option<crate::data::Axis> {
        None
    }

    /// Rules use the axes of the chart they're overlaid on.
    pub fn y_axis() -> Option<crate::data::Axis> {
        None
    }
}

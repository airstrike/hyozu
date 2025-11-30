use crate::color::Color;
use crate::core::Pixels;
use std::sync::Arc;

/// Position of data labels on bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Above the bar (outside)
    Above,
    /// At the end (top for positive, bottom for negative bars) - inside the bar
    End,
    /// At the center of the bar - inside the bar
    Center,
    /// At the base (bottom for positive, top for negative bars) - inside the bar
    Base,
}

/// Data label configuration for bars.
#[derive(Clone)]
pub struct Label {
    pub(crate) position: Position,
    pub(crate) format: Arc<dyn Fn(f64) -> String + Send + Sync>,
    pub(crate) color: Option<Color>,
    pub(crate) size: Option<Pixels>,
}

impl PartialEq for Label {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && (self.format)(1.0) == (other.format)(1.0)
            && self.color == other.color
            && self.size == other.size
    }
}

impl std::fmt::Debug for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Label")
            .field("position", &self.position)
            .field("format", &"<function>")
            .field("color", &self.color)
            .field("size", &self.size)
            .finish()
    }
}

/// Default label formatter that displays numbers naturally.
pub fn default(value: f64) -> String {
    if value.fract().abs() < 0.001 {
        format!("{}", value as i64)
    } else {
        format!("{:.1}", value)
    }
}

impl Default for Label {
    fn default() -> Self {
        Self {
            position: Position::Above,
            format: Arc::new(default),
            color: None,
            size: None,
        }
    }
}

impl Label {
    /// Create a new label configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the position of the label.
    pub fn with_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Set a custom format function for the label text.
    pub fn with_format(
        mut self,
        f: impl Fn(f64) -> String + Send + Sync + 'static,
    ) -> Self {
        self.format = Arc::new(f);
        self
    }

    /// Set the label color.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the label size.
    pub fn with_size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    // === Property getters ===

    /// Returns the label position.
    pub fn position(&self) -> Position {
        self.position
    }

    /// Returns the label color.
    pub fn color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    /// Returns the label size.
    pub fn size(&self) -> Option<Pixels> {
        self.size
    }
}

// === From implementations ===

impl From<Position> for Label {
    fn from(position: Position) -> Self {
        Label { position, ..Default::default() }
    }
}

impl From<Position> for Option<Label> {
    fn from(position: Position) -> Self {
        Some(Label::from(position))
    }
}

// === Position + component ===

impl<F: Fn(f64) -> String + Send + Sync + 'static> std::ops::Add<F> for Position {
    type Output = Label;
    fn add(self, format: F) -> Label {
        Label::from(self).with_format(format)
    }
}

// === Label + component (accumulator) ===

impl std::ops::Add<Position> for Label {
    type Output = Label;
    fn add(self, position: Position) -> Label {
        self.with_position(position)
    }
}

impl<F: Fn(f64) -> String + Send + Sync + 'static> std::ops::Add<F> for Label {
    type Output = Label;
    fn add(self, format: F) -> Label {
        self.with_format(format)
    }
}

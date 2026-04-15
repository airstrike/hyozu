use crate::color::Color;
use crate::core::font::{Style, Weight};
use crate::core::{Font, Pixels};
use crate::text;
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
    /// Typography override for the label text.
    pub(crate) text: text::Style,
    pub(crate) fill: Option<Color>,
}

impl PartialEq for Label {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && (self.format)(1.0) == (other.format)(1.0)
            && self.color == other.color
            && self.text == other.text
            && self.fill == other.fill
    }
}

impl std::fmt::Debug for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Label")
            .field("position", &self.position)
            .field("format", &"<function>")
            .field("color", &self.color)
            .field("text", &self.text)
            .field("fill", &self.fill)
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
            text: text::Style::new(),
            fill: None,
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
    pub fn with_format(mut self, f: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Arc::new(f);
        self
    }

    /// Set the label color.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Override the full text style.
    ///
    /// Any field left unset on `style` falls back to
    /// [`crate::Design::data_label_text`].
    pub fn with_text(mut self, style: text::Style) -> Self {
        self.text = style;
        self
    }

    /// Set the label font family (overrides the theme default).
    ///
    /// Accepts anything convertible to a [`Font`] — `"Inter"` works.
    pub fn with_font(mut self, font: impl Into<Font>) -> Self {
        self.text.family = Some(font.into());
        self
    }

    /// Set the label size.
    pub fn with_size(mut self, size: impl Into<Pixels>) -> Self {
        self.text.size = Some(size.into());
        self
    }

    /// Set the label font weight.
    pub fn with_weight(mut self, weight: Weight) -> Self {
        self.text.weight = Some(weight);
        self
    }

    /// Set the label font style.
    pub fn with_style(mut self, style: Style) -> Self {
        self.text.style = Some(style);
        self
    }

    /// Set the label background fill color.
    pub fn with_fill(mut self, fill: impl Into<Color>) -> Self {
        self.fill = Some(fill.into());
        self
    }

    // === Property setters ===

    /// Sets the label color in place.
    pub fn set_color(&mut self, color: Option<Color>) {
        self.color = color;
    }

    /// Sets the label size in place.
    pub fn set_size(&mut self, size: Option<Pixels>) {
        self.text.size = size;
    }

    /// Sets the label font weight in place.
    pub fn set_weight(&mut self, weight: Option<Weight>) {
        self.text.weight = weight;
    }

    /// Sets the label font style in place.
    pub fn set_style(&mut self, style: Option<Style>) {
        self.text.style = style;
    }

    /// Sets the label font family in place.
    pub fn set_font(&mut self, font: Option<Font>) {
        self.text.family = font;
    }

    /// Sets the label background fill color in place.
    pub fn set_fill(&mut self, fill: Option<Color>) {
        self.fill = fill;
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
        self.text.size
    }

    /// Returns the label font weight.
    pub fn weight(&self) -> Option<Weight> {
        self.text.weight
    }

    /// Returns the label font style.
    pub fn style(&self) -> Option<Style> {
        self.text.style
    }

    /// Returns the label background fill color.
    pub fn fill(&self) -> Option<&Color> {
        self.fill.as_ref()
    }

    /// Returns the label text style override.
    pub fn text(&self) -> &text::Style {
        &self.text
    }
}

// === From implementations ===

impl From<Position> for Label {
    fn from(position: Position) -> Self {
        Label {
            position,
            ..Default::default()
        }
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

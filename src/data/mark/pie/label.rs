use crate::color::Color;
use crate::core::Font;
use crate::core::font::{Style, Weight};
use crate::text;
use std::sync::Arc;

pub use Position::{Edge, Inside, Outside};

// Re-export common font weight/style variants so label composition
// reads naturally:  `Outside + font("Inter") + Bold`
pub use crate::core::font::Style::Italic;
pub use crate::core::font::Weight::{Black, Bold, Light, Medium, Semibold, Thin};

/// Position of pie slice labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// Inside the slice — midway between inner and outer radius (or
    /// `radius * 0.65` for full pies).
    #[default]
    Inside,
    /// Outside the outer arc, with a leader line from the slice edge.
    Outside,
    /// At the outer arc edge, just inside the slice.
    Edge,
}

/// Controls which slices show labels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Show {
    /// Show labels on every slice.
    #[default]
    All,
    /// Only show labels on slices whose fraction (0.0..=1.0) is at least the
    /// given threshold.
    Threshold(f32),
    /// Only show labels on the N largest slices.
    Top(usize),
}

impl Eq for Show {}

/// Distinguishes the built-in value/percent constructors from user
/// closures so the renderer can substitute the value-format precedence
/// chain for [`Label::value`] (mark override → data scale → default).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum FormatKind {
    /// `Label::default` — percent text via the default closure.
    #[default]
    DefaultPercent,
    /// `Label::percent` — explicit percent label.
    Percent,
    /// `Label::value` — raw value; renderer overrides the closure with
    /// the value-format chain.
    Value,
    /// `Label::custom` or `Label::custom_format` — user closure wins.
    Custom,
}

/// Label configuration for pie slices.
#[derive(Clone)]
pub struct Label {
    /// Position relative to the slice arc.
    pub(crate) position: Position,
    /// Which slices show this label (when broadcast via `Pie::labels`).
    pub(crate) show: Show,
    /// Format function for the label text. Receives `(value, percentage)`
    /// where percentage is `0.0..=1.0`.
    pub(crate) format: Arc<dyn Fn(f64, f64) -> String + Send + Sync>,
    /// Provenance of the `format` closure — see [`FormatKind`].
    pub(crate) format_kind: FormatKind,
    pub(crate) color: Option<Color>,
    pub(crate) text: text::Style,
    pub(crate) fill: Option<Color>,
}

impl PartialEq for Label {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && self.show == other.show
            && self.format_kind == other.format_kind
            && (self.format)(1.0, 0.5) == (other.format)(1.0, 0.5)
            && self.color == other.color
            && self.text == other.text
            && self.fill == other.fill
    }
}

impl std::fmt::Debug for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Label")
            .field("position", &self.position)
            .field("show", &self.show)
            .field("format", &"<function>")
            .field("format_kind", &self.format_kind)
            .field("color", &self.color)
            .field("text", &self.text)
            .field("fill", &self.fill)
            .finish()
    }
}

/// Default label formatter that displays the percentage.
pub fn default(_value: f64, pct: f64) -> String {
    format!("{:.0}%", pct * 100.0)
}

impl Default for Label {
    fn default() -> Self {
        Self {
            position: Position::default(),
            show: Show::default(),
            format: Arc::new(default),
            format_kind: FormatKind::DefaultPercent,
            color: None,
            text: text::Style::new(),
            fill: None,
        }
    }
}

impl Label {
    /// Creates a label that shows the percentage.
    pub fn percent() -> Self {
        Self {
            format: Arc::new(|_value, pct| format!("{:.0}%", pct * 100.0)),
            format_kind: FormatKind::Percent,
            ..Self::default()
        }
    }

    /// Creates a label that shows the raw value, formatted via the
    /// value-format chain (mark override → `Data::value_scale` → built-in
    /// default). The renderer substitutes the chain at draw time so the
    /// in-mark text matches the legend column and tooltip body.
    pub fn value() -> Self {
        Self {
            format: Arc::new(|value, _pct| crate::scale::default_f64_format(value)),
            format_kind: FormatKind::Value,
            ..Self::default()
        }
    }

    /// Creates a label with a custom format function.
    ///
    /// The function receives `(value, percentage)` where percentage is 0.0..1.0.
    pub fn custom(f: impl Fn(f64, f64) -> String + Send + Sync + 'static) -> Self {
        Self {
            format: Arc::new(f),
            format_kind: FormatKind::Custom,
            ..Self::default()
        }
    }

    /// Sets the label position.
    pub fn with_position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Sets which slices show labels.
    pub fn with_show(mut self, show: Show) -> Self {
        self.show = show;
        self
    }

    /// Sets the label color.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Override the full text style for this label.
    pub fn with_text(mut self, style: text::Style) -> Self {
        self.text = style;
        self
    }

    /// Sets the label font family (overrides the theme default).
    pub fn with_font(mut self, font: impl Into<Font>) -> Self {
        self.text.family = Some(font.into());
        self
    }

    /// Sets the label font size.
    pub fn with_size(mut self, size: impl Into<crate::core::Pixels>) -> Self {
        self.text.size = Some(size.into());
        self
    }

    /// Sets the label font weight.
    pub fn with_weight(mut self, weight: Weight) -> Self {
        self.text.weight = Some(weight);
        self
    }

    /// Sets the label font style.
    pub fn with_style(mut self, style: Style) -> Self {
        self.text.style = Some(style);
        self
    }

    /// Sets the label background fill color.
    pub fn with_fill(mut self, fill: impl Into<Color>) -> Self {
        self.fill = Some(fill.into());
        self
    }

    // === Property setters (in-place) ===

    pub fn set_position(&mut self, position: Position) {
        self.position = position;
    }

    pub fn set_show(&mut self, show: Show) {
        self.show = show;
    }

    pub fn set_color(&mut self, color: Option<Color>) {
        self.color = color;
    }

    pub fn set_size(&mut self, size: Option<crate::core::Pixels>) {
        self.text.size = size;
    }

    pub fn set_weight(&mut self, weight: Option<Weight>) {
        self.text.weight = weight;
    }

    pub fn set_style(&mut self, style: Option<Style>) {
        self.text.style = style;
    }

    pub fn set_font(&mut self, font: Option<Font>) {
        self.text.family = font;
    }

    pub fn set_fill(&mut self, fill: Option<Color>) {
        self.fill = fill;
    }

    // === Property getters ===

    pub fn position(&self) -> Position {
        self.position
    }

    pub fn show(&self) -> Show {
        self.show
    }

    pub fn color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    pub fn size(&self) -> Option<crate::core::Pixels> {
        self.text.size
    }

    pub fn weight(&self) -> Option<Weight> {
        self.text.weight
    }

    pub fn style(&self) -> Option<Style> {
        self.text.style
    }

    pub fn fill(&self) -> Option<&Color> {
        self.fill.as_ref()
    }

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

impl From<Show> for Label {
    fn from(show: Show) -> Self {
        Label {
            show,
            ..Default::default()
        }
    }
}

impl From<Position> for Option<Label> {
    fn from(position: Position) -> Self {
        Some(Label::from(position))
    }
}

impl From<Show> for Option<Label> {
    fn from(show: Show) -> Self {
        Some(Label::from(show))
    }
}

// === Position + component ===

impl<F: Fn(f64, f64) -> String + Send + Sync + 'static> std::ops::Add<F> for Position {
    type Output = Label;
    fn add(self, format: F) -> Label {
        Label::from(self).custom_format(format)
    }
}

impl std::ops::Add<crate::text::Style> for Position {
    type Output = Label;
    fn add(self, text: crate::text::Style) -> Label {
        Label::from(self).with_text(text)
    }
}

impl std::ops::Add<crate::core::Font> for Position {
    type Output = Label;
    fn add(self, font: crate::core::Font) -> Label {
        Label::from(self).with_font(font)
    }
}

impl std::ops::Add<Weight> for Position {
    type Output = Label;
    fn add(self, weight: Weight) -> Label {
        Label::from(self).with_weight(weight)
    }
}

impl std::ops::Add<Show> for Position {
    type Output = Label;
    fn add(self, show: Show) -> Label {
        Label {
            position: self,
            show,
            ..Default::default()
        }
    }
}

// === Show + component ===

impl std::ops::Add<Position> for Show {
    type Output = Label;
    fn add(self, position: Position) -> Label {
        Label {
            position,
            show: self,
            ..Default::default()
        }
    }
}

impl<F: Fn(f64, f64) -> String + Send + Sync + 'static> std::ops::Add<F> for Show {
    type Output = Label;
    fn add(self, format: F) -> Label {
        Label::from(self).custom_format(format)
    }
}

impl std::ops::Add<crate::text::Style> for Show {
    type Output = Label;
    fn add(self, text: crate::text::Style) -> Label {
        Label::from(self).with_text(text)
    }
}

impl std::ops::Add<Weight> for Show {
    type Output = Label;
    fn add(self, weight: Weight) -> Label {
        Label::from(self).with_weight(weight)
    }
}

// === Label + component (accumulator) ===

impl std::ops::Add<Position> for Label {
    type Output = Label;
    fn add(self, position: Position) -> Label {
        self.with_position(position)
    }
}

impl std::ops::Add<Show> for Label {
    type Output = Label;
    fn add(self, show: Show) -> Label {
        self.with_show(show)
    }
}

impl Label {
    /// Replace the format function. Used internally by `Add` impls; mirrors
    /// the public surface of `bar::label::Label::with_format`.
    fn custom_format(mut self, f: impl Fn(f64, f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Arc::new(f);
        self.format_kind = FormatKind::Custom;
        self
    }

    /// Returns the provenance of the format closure — the renderer
    /// reads this to decide whether to substitute the value-format
    /// chain.
    pub fn format_kind(&self) -> FormatKind {
        self.format_kind
    }
}

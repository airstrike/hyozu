use crate::color::Color;
use crate::core::font::{Style, Weight};
use crate::core::{Font, Pixels};
use crate::data::mark::waterfall::EntryKind;
use crate::text;
use std::sync::Arc;

pub use Position::{Above, Base, Center, End};

// Re-export common font weight/style variants so label composition
// reads naturally:  `Above + font("Inter") + Bold + Italic`
pub use crate::core::font::Style::Italic;
pub use crate::core::font::Weight::{Black, Bold, Light, Medium, Semibold, Thin};

/// Position of data labels on waterfall bars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    /// Outside the bar — above the top for positive bars/totals,
    /// below the bottom for negative bars.
    Above,
    /// At the bar tip (top for positive, bottom for negative) — inside the bar.
    End,
    /// At the center of the bar.
    Center,
    /// At the baseline-side edge of the bar — inside.
    Base,
}

/// Controls which entries show labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Show {
    /// Show labels on every entry.
    #[default]
    All,
    /// Show labels on totals only.
    Totals,
    /// Show labels on increase/decrease entries only.
    Changes,
    /// Show labels on the first and last entries only.
    EndsOnly,
}

impl Show {
    /// Returns whether this entry should display a label given its position
    /// and total count.
    pub fn allows(&self, index: usize, total: usize, kind: EntryKind) -> bool {
        match self {
            Show::All => true,
            Show::Totals => matches!(kind, EntryKind::Total),
            Show::Changes => !matches!(kind, EntryKind::Total),
            Show::EndsOnly => index == 0 || (total > 0 && index + 1 == total),
        }
    }
}

/// Data label configuration for waterfall charts.
#[derive(Clone)]
pub struct Label {
    pub(crate) position: Position,
    pub(crate) show: Show,
    pub(crate) format: Arc<dyn Fn(f64) -> String + Send + Sync>,
    pub(crate) color: Option<Color>,
    /// Typography override for the label text.
    pub(crate) text: text::Style,
    pub(crate) fill: Option<Color>,
}

impl PartialEq for Label {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position
            && self.show == other.show
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
            .field("show", &self.show)
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
            show: Show::default(),
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

    /// Set which entries show labels.
    pub fn with_show(mut self, show: Show) -> Self {
        self.show = show;
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
    pub fn with_text(mut self, style: text::Style) -> Self {
        self.text = style;
        self
    }

    /// Set the label font family (overrides the theme default).
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

    pub fn set_size(&mut self, size: Option<Pixels>) {
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

    pub fn size(&self) -> Option<Pixels> {
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

impl<F: Fn(f64) -> String + Send + Sync + 'static> std::ops::Add<F> for Position {
    type Output = Label;
    fn add(self, format: F) -> Label {
        Label::from(self).with_format(format)
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

impl<F: Fn(f64) -> String + Send + Sync + 'static> std::ops::Add<F> for Show {
    type Output = Label;
    fn add(self, format: F) -> Label {
        Label::from(self).with_format(format)
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

impl<F: Fn(f64) -> String + Send + Sync + 'static> std::ops::Add<F> for Label {
    type Output = Label;
    fn add(self, format: F) -> Label {
        self.with_format(format)
    }
}

impl std::ops::Add<crate::text::Style> for Label {
    type Output = Label;
    fn add(self, text: crate::text::Style) -> Label {
        self.with_text(text)
    }
}

impl std::ops::Add<crate::core::Font> for Label {
    type Output = Label;
    fn add(self, font: crate::core::Font) -> Label {
        self.with_font(font)
    }
}

impl std::ops::Add<Weight> for Label {
    type Output = Label;
    fn add(self, weight: Weight) -> Label {
        self.with_weight(weight)
    }
}

impl std::ops::Add<Style> for Label {
    type Output = Label;
    fn add(self, style: Style) -> Label {
        self.with_style(style)
    }
}

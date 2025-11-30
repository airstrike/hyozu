use crate::color::Color;
use crate::core::Pixels;
use std::sync::Arc;

/// Position of data labels on line chart points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// Automatically position to avoid overlapping the line
    #[default]
    Auto,
    /// Above the point
    Above,
    /// Below the point
    Below,
    /// To the left of the point
    Left,
    /// To the right of the point
    Right,
}

/// Controls which points show data labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Show {
    /// Show labels on all points
    #[default]
    Any,
    /// Show label only on the first point
    FirstOnly,
    /// Show label only on the last point
    LastOnly,
    /// Show labels on both first and last points
    FirstAndLast,
    /// Show labels on first occurrence of min and max Y values
    MinMaxFirst,
    /// Show labels on all occurrences of min and max Y values
    MinMaxAll,
    /// Show labels on last occurrence of min and max Y values
    MinMaxLast,
}

/// Data label configuration for line charts.
#[derive(Clone)]
pub struct Label {
    /// Position of the label relative to the data point
    pub position: Position,
    /// Which points show labels
    pub show: Show,
    pub(crate) format: Arc<dyn Fn(f64) -> String + Send + Sync>,
    /// Label color
    pub color: Option<Color>,
    /// Label size
    pub size: Option<Pixels>,
}

impl std::fmt::Debug for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Label")
            .field("position", &self.position)
            .field("show", &self.show)
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
            show: Show::default(),
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
    pub fn position(mut self, position: Position) -> Self {
        self.position = position;
        self
    }

    /// Set which points show labels.
    pub fn show(mut self, show: Show) -> Self {
        self.show = show;
        self
    }

    /// Set a custom format function for the label text.
    pub fn format(
        mut self,
        f: impl Fn(f64) -> String + Send + Sync + 'static,
    ) -> Self {
        self.format = Arc::new(f);
        self
    }

    /// Set the label color.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the label size.
    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }
}

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

impl std::ops::Add<Show> for Position {
    type Output = Label;
    fn add(self, rhs: Show) -> Label {
        Label {
            position: self,
            show: rhs,
            ..Default::default()
        }
    }
}

impl<F: Fn(f64) -> String + Send + Sync + 'static> std::ops::Add<F>
    for Position
{
    type Output = Label;
    fn add(self, format: F) -> Label {
        Label::from(self).format(format)
    }
}

impl std::ops::Add<Position> for Show {
    type Output = Label;
    fn add(self, rhs: Position) -> Label {
        Label {
            position: rhs,
            show: self,
            ..Default::default()
        }
    }
}

impl<F: Fn(f64) -> String + Send + Sync + 'static> std::ops::Add<F> for Show {
    type Output = Label;
    fn add(self, format: F) -> Label {
        Label::from(self).format(format)
    }
}

impl std::ops::Add<Position> for Label {
    type Output = Label;
    fn add(self, position: Position) -> Label {
        self.position(position)
    }
}

impl std::ops::Add<Show> for Label {
    type Output = Label;
    fn add(self, show: Show) -> Label {
        self.show(show)
    }
}

impl<F: Fn(f64) -> String + Send + Sync + 'static> std::ops::Add<F> for Label {
    type Output = Label;
    fn add(self, format: F) -> Label {
        self.format(format)
    }
}

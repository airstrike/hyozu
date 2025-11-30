use super::label::Label;
use crate::color::Color;
use crate::data::{Datum, IntoDatums};

/// A single series of bars within a bar chart.
#[derive(Debug, Clone)]
pub struct Series {
    /// Data points for this series.
    pub(crate) points: Vec<Datum>,
    /// Optional color for this series.
    pub(crate) color: Option<Color>,
    /// Optional data labels for this series.
    pub(crate) label: Option<Label>,
}

impl Series {
    /// Create a new bar series from data points.
    pub fn new(data: impl IntoDatums) -> Self {
        Self {
            points: data.into_datums(),
            color: None,
            label: Some(Label::default()),
        }
    }

    /// Sets the color for this series.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Configure data labels for this series.
    pub fn with_labels(mut self, label: impl Into<Label>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the label position in place (for property updates).
    pub fn set_label_position(&mut self, position: super::label::Position) {
        if let Some(label) = &mut self.label {
            label.position = position;
        } else {
            self.label = Some(Label::default().with_position(position));
        }
    }

    // === Property getters ===

    /// Returns a reference to the label configuration.
    pub fn label(&self) -> Option<&Label> {
        self.label.as_ref()
    }

    /// Returns a reference to the color.
    pub fn color(&self) -> Option<&crate::color::Color> {
        self.color.as_ref()
    }

    /// Returns a reference to the data points.
    pub fn points(&self) -> &[Datum] {
        &self.points
    }
}

impl<T: IntoDatums> From<T> for Series {
    fn from(data: T) -> Self {
        Series::new(data)
    }
}

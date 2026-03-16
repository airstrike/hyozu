use super::label::Label;
use crate::color::Color;
use crate::core::Pixels;
use crate::core::font::{Style, Weight};
use crate::data::{Datum, IntoDatums};

/// A single series of bars within a bar chart.
#[derive(Debug, Clone)]
pub struct Series {
    /// Data points for this series.
    pub(crate) points: Vec<Datum>,
    /// Optional color for this series.
    pub(crate) color: Option<Color>,
    /// Per-point color overrides. Empty = no overrides.
    /// When `point_colors[i]` is `Some(color)`, that bar uses it instead of the series color.
    pub(crate) point_colors: Vec<Option<Color>>,
    /// Per-point label overrides. Empty = no overrides.
    pub(crate) point_labels: Vec<Option<Label>>,
    /// Optional data labels for this series.
    pub(crate) label: Option<Label>,
    /// Optional name for this series (used in legends).
    pub(crate) name: Option<String>,
}

impl Series {
    /// Create a new bar series from data points.
    pub fn new(data: impl IntoDatums) -> Self {
        Self {
            points: data.into_datums(),
            color: None,
            point_colors: Vec::new(),
            point_labels: Vec::new(),
            label: Some(Label::default()),
            name: None,
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

    /// Sets the name for this series (used in legends).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    // === Property getters ===

    /// Returns the name of this series.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

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

    /// Returns the per-point color override for a given index, if any.
    pub fn point_color(&self, index: usize) -> Option<&Color> {
        self.point_colors.get(index).and_then(|c| c.as_ref())
    }

    /// Sets a per-point color override. Grows the vec with `None` if needed.
    pub fn set_point_color(&mut self, index: usize, color: impl Into<Color>) {
        if index >= self.point_colors.len() {
            self.point_colors.resize(index + 1, None);
        }
        self.point_colors[index] = Some(color.into());
    }

    /// Clears the per-point color override at the given index.
    pub fn clear_point_color(&mut self, index: usize) {
        if index < self.point_colors.len() {
            self.point_colors[index] = None;
        }
    }

    /// Returns true if any per-point color overrides are set.
    pub fn has_point_colors(&self) -> bool {
        self.point_colors.iter().any(|c| c.is_some())
    }

    // === Series-level label setters ===

    /// Sets the label color for this series.
    pub fn set_label_color(&mut self, color: Option<Color>) {
        if let Some(label) = &mut self.label {
            label.set_color(color);
        } else {
            let mut label = Label::default();
            label.set_color(color);
            self.label = Some(label);
        }
    }

    /// Sets the label size for this series.
    pub fn set_label_size(&mut self, size: Option<Pixels>) {
        if let Some(label) = &mut self.label {
            label.set_size(size);
        } else {
            let mut label = Label::default();
            label.set_size(size);
            self.label = Some(label);
        }
    }

    /// Sets the label font weight for this series.
    pub fn set_label_weight(&mut self, weight: Option<Weight>) {
        if let Some(label) = &mut self.label {
            label.set_weight(weight);
        } else {
            let mut label = Label::default();
            label.set_weight(weight);
            self.label = Some(label);
        }
    }

    /// Sets the label font style for this series.
    pub fn set_label_style(&mut self, style: Option<Style>) {
        if let Some(label) = &mut self.label {
            label.set_style(style);
        } else {
            let mut label = Label::default();
            label.set_style(style);
            self.label = Some(label);
        }
    }

    /// Sets the label background fill for this series.
    pub fn set_label_fill(&mut self, fill: Option<Color>) {
        if let Some(label) = &mut self.label {
            label.set_fill(fill);
        } else {
            let mut label = Label::default();
            label.set_fill(fill);
            self.label = Some(label);
        }
    }

    // === Per-point label overrides ===

    /// Returns the per-point label override for a given index, if any.
    pub fn point_label(&self, index: usize) -> Option<&Label> {
        self.point_labels.get(index).and_then(|l| l.as_ref())
    }

    /// Sets a per-point label color override.
    pub fn set_point_label_color(&mut self, index: usize, color: Option<Color>) {
        self.ensure_point_label(index);
        if let Some(Some(label)) = self.point_labels.get_mut(index) {
            label.set_color(color);
        }
    }

    /// Sets a per-point label size override.
    pub fn set_point_label_size(&mut self, index: usize, size: Option<Pixels>) {
        self.ensure_point_label(index);
        if let Some(Some(label)) = self.point_labels.get_mut(index) {
            label.set_size(size);
        }
    }

    /// Sets a per-point label weight override.
    pub fn set_point_label_weight(&mut self, index: usize, weight: Option<Weight>) {
        self.ensure_point_label(index);
        if let Some(Some(label)) = self.point_labels.get_mut(index) {
            label.set_weight(weight);
        }
    }

    /// Sets a per-point label style override.
    pub fn set_point_label_style(&mut self, index: usize, style: Option<Style>) {
        self.ensure_point_label(index);
        if let Some(Some(label)) = self.point_labels.get_mut(index) {
            label.set_style(style);
        }
    }

    /// Sets a per-point label fill override.
    pub fn set_point_label_fill(&mut self, index: usize, fill: Option<Color>) {
        self.ensure_point_label(index);
        if let Some(Some(label)) = self.point_labels.get_mut(index) {
            label.set_fill(fill);
        }
    }

    /// Ensures a point label entry exists at the given index.
    fn ensure_point_label(&mut self, index: usize) {
        if index >= self.point_labels.len() {
            self.point_labels.resize(index + 1, None);
        }
        if self.point_labels[index].is_none() {
            self.point_labels[index] = Some(Label::default());
        }
    }
}

impl<T: IntoDatums> From<T> for Series {
    fn from(data: T) -> Self {
        Series::new(data)
    }
}

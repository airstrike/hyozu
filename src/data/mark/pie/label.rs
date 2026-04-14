use crate::color::Color;
use crate::core::font::{Style, Weight};
use std::sync::Arc;

/// Label configuration for pie slices.
#[derive(Clone)]
pub struct Label {
    /// Format function for the label text
    pub(crate) format: Arc<dyn Fn(f64, f64) -> String + Send + Sync>,
    /// Label color
    pub(crate) color: Option<Color>,
    /// Label font size
    pub(crate) size: Option<crate::core::Pixels>,
    /// Label font weight
    pub(crate) weight: Option<Weight>,
    /// Label font style
    pub(crate) style: Option<Style>,
    /// Label background fill color
    pub(crate) fill: Option<Color>,
}

impl std::fmt::Debug for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Label")
            .field("format", &"<function>")
            .field("color", &self.color)
            .field("size", &self.size)
            .field("weight", &self.weight)
            .field("style", &self.style)
            .field("fill", &self.fill)
            .finish()
    }
}

impl Label {
    /// Creates a label that shows the percentage.
    pub fn percent() -> Self {
        Self {
            format: Arc::new(|_value, pct| format!("{:.0}%", pct * 100.0)),
            color: None,
            size: None,
            weight: None,
            style: None,
            fill: None,
        }
    }

    /// Creates a label that shows the raw value.
    pub fn value() -> Self {
        Self {
            format: Arc::new(|value, _pct| format!("{}", value)),
            color: None,
            size: None,
            weight: None,
            style: None,
            fill: None,
        }
    }

    /// Creates a label with a custom format function.
    ///
    /// The function receives `(value, percentage)` where percentage is 0.0..1.0.
    pub fn custom(f: impl Fn(f64, f64) -> String + Send + Sync + 'static) -> Self {
        Self {
            format: Arc::new(f),
            color: None,
            size: None,
            weight: None,
            style: None,
            fill: None,
        }
    }

    /// Sets the label color.
    pub fn with_color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the label font size.
    pub fn with_size(mut self, size: impl Into<crate::core::Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }

    /// Sets the label font weight.
    pub fn with_weight(mut self, weight: Weight) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Sets the label font style.
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the label background fill color.
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
    pub fn set_size(&mut self, size: Option<crate::core::Pixels>) {
        self.size = size;
    }

    /// Sets the label font weight in place.
    pub fn set_weight(&mut self, weight: Option<Weight>) {
        self.weight = weight;
    }

    /// Sets the label font style in place.
    pub fn set_style(&mut self, style: Option<Style>) {
        self.style = style;
    }

    /// Sets the label background fill color in place.
    pub fn set_fill(&mut self, fill: Option<Color>) {
        self.fill = fill;
    }

    // === Property getters ===

    /// Returns the label color.
    pub fn color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    /// Returns the label size.
    pub fn size(&self) -> Option<crate::core::Pixels> {
        self.size
    }

    /// Returns the label font weight.
    pub fn weight(&self) -> Option<Weight> {
        self.weight
    }

    /// Returns the label font style.
    pub fn style(&self) -> Option<Style> {
        self.style
    }

    /// Returns the label background fill color.
    pub fn fill(&self) -> Option<&Color> {
        self.fill.as_ref()
    }
}

use crate::color::Color;
use std::sync::Arc;

/// Label configuration for pie slices.
#[derive(Clone)]
#[allow(dead_code)]
pub struct Label {
    /// Format function for the label text
    pub(crate) format: Arc<dyn Fn(f64, f64) -> String + Send + Sync>,
    /// Label color
    pub(crate) color: Option<Color>,
    /// Label font size
    pub(crate) size: Option<crate::core::Pixels>,
}

impl std::fmt::Debug for Label {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Label")
            .field("format", &"<function>")
            .field("color", &self.color)
            .field("size", &self.size)
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
        }
    }

    /// Creates a label that shows the raw value.
    pub fn value() -> Self {
        Self {
            format: Arc::new(|value, _pct| format!("{}", value)),
            color: None,
            size: None,
        }
    }

    /// Creates a label with a custom format function.
    ///
    /// The function receives `(value, percentage)` where percentage is 0.0..1.0.
    pub fn custom(
        f: impl Fn(f64, f64) -> String + Send + Sync + 'static,
    ) -> Self {
        Self {
            format: Arc::new(f),
            color: None,
            size: None,
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
}

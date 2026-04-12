use crate::color::Color;

/// Position of the legend relative to the plot area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Position {
    /// Below the title, above the plot area (default).
    #[default]
    Above,
    /// Below the plot area and bottom axis.
    Below,
    /// Vertical column to the left of the plot area.
    Left,
    /// Vertical column to the right of the plot area.
    Right,
}

/// Configuration for the chart legend.
///
/// # Examples
///
/// ```
/// use hyozu::{LegendConfig, LegendPosition};
///
/// // Short form — just set position
/// let config = LegendPosition::Below;
///
/// // Detailed form — builder
/// let config = LegendConfig::below().font_size(10.0);
/// ```
#[derive(Debug, Clone)]
pub struct Legend {
    pub(crate) position: Position,
    /// Font size override. None = default (10.0).
    pub(crate) font_size: Option<f32>,
    /// Optional text color override.
    pub(crate) text_color: Option<Color>,
    /// Whether legend entries wrap to multiple rows (default true).
    pub(crate) wrap: bool,
    /// Whether clicking a legend entry toggles its series visibility.
    /// Defaults to `false` so existing charts are unaffected.
    pub(crate) interactive: bool,
}

impl Default for Legend {
    fn default() -> Self {
        Self {
            position: Position::default(),
            font_size: None,
            text_color: None,
            wrap: true,
            interactive: false,
        }
    }
}

impl Legend {
    /// Create a legend positioned above the plot area.
    pub fn above() -> Self {
        Self {
            position: Position::Above,
            ..Self::default()
        }
    }

    /// Create a legend positioned below the plot area.
    pub fn below() -> Self {
        Self {
            position: Position::Below,
            ..Self::default()
        }
    }

    /// Create a legend positioned to the left of the plot area.
    pub fn left() -> Self {
        Self {
            position: Position::Left,
            ..Self::default()
        }
    }

    /// Create a legend positioned to the right of the plot area.
    pub fn right() -> Self {
        Self {
            position: Position::Right,
            ..Self::default()
        }
    }

    /// Sets the font size for legend text.
    pub fn font_size(mut self, size: f32) -> Self {
        self.font_size = Some(size);
        self
    }

    /// Sets the text color for legend labels.
    pub fn text_color(mut self, color: impl Into<Color>) -> Self {
        self.text_color = Some(color.into());
        self
    }

    /// Whether legend entries wrap to multiple rows/columns (default true).
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Enable or disable click-to-toggle on legend entries.
    ///
    /// When enabled, clicking a legend entry hides or shows its corresponding
    /// series. Visibility state lives inside the chart widget and is reset
    /// when the widget is destroyed.
    pub fn interactive(mut self, enabled: bool) -> Self {
        self.interactive = enabled;
        self
    }
}

impl From<Position> for Legend {
    fn from(position: Position) -> Self {
        Self {
            position,
            ..Self::default()
        }
    }
}

/// Tick mark configuration - instructions for generating ticks from data.
#[derive(Debug, Clone)]
pub struct Ticks {
    /// Tick mark style
    pub style: Style,

    /// Tick frequency
    pub frequency: Frequency,

    /// Tick alignment mode
    pub alignment: Alignment,

    /// Tick mark length in pixels
    pub mark_length: f32,
}

/// Tick alignment mode for "nice" tick placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    /// Find nice round numbers within the range (default behavior)
    #[default]
    Auto,
    /// Align ticks working forward from the start of the range
    SnapToStart,
    /// Align ticks working backward from the end of the range
    SnapToEnd,
}

impl Ticks {
    /// Create default ticks for categorical data (bars).
    pub fn categorical() -> Self {
        Self {
            style: Style::Outset,
            frequency: Frequency::EveryItem,
            alignment: Alignment::Auto,
            mark_length: 6.0,
        }
    }

    /// Create default ticks for continuous data (lines).
    pub fn continuous() -> Self {
        Self {
            style: Style::Outset,
            frequency: Frequency::EveryItem,
            alignment: Alignment::Auto,
            mark_length: 6.0,
        }
    }

    /// Set the alignment mode.
    pub fn alignment(mut self, alignment: Alignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set the style.
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Default for Ticks {
    fn default() -> Self {
        Self::continuous()
    }
}

// From implementations for ergonomic API
impl From<Alignment> for Ticks {
    fn from(alignment: Alignment) -> Self {
        Ticks {
            alignment,
            ..Default::default()
        }
    }
}

impl From<Style> for Ticks {
    fn from(style: Style) -> Self {
        Ticks {
            style,
            ..Default::default()
        }
    }
}

// Add operator for combining tick options
impl std::ops::Add<Alignment> for Style {
    type Output = Ticks;
    fn add(self, alignment: Alignment) -> Ticks {
        Ticks {
            style: self,
            alignment,
            ..Default::default()
        }
    }
}

impl std::ops::Add<Style> for Alignment {
    type Output = Ticks;
    fn add(self, style: Style) -> Ticks {
        Ticks {
            alignment: self,
            style,
            ..Default::default()
        }
    }
}

impl std::ops::Add<Alignment> for Ticks {
    type Output = Ticks;
    fn add(self, alignment: Alignment) -> Ticks {
        self.alignment(alignment)
    }
}

impl std::ops::Add<Style> for Ticks {
    type Output = Ticks;
    fn add(self, style: Style) -> Ticks {
        self.style(style)
    }
}

impl From<Vec<f64>> for Ticks {
    fn from(values: Vec<f64>) -> Self {
        Ticks {
            frequency: Frequency::Custom(values),
            ..Default::default()
        }
    }
}

impl<const N: usize> From<[f64; N]> for Ticks {
    fn from(values: [f64; N]) -> Self {
        Ticks {
            frequency: Frequency::Custom(values.to_vec()),
            ..Default::default()
        }
    }
}

/// Style of tick marks on an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Tick marks extend inward from axis
    Inset,
    /// Tick marks extend outward from axis
    Outset,
    /// Tick marks cross the axis line
    Cross,
    /// No tick marks (labels only)
    None,
}

/// Frequency of tick marks.
#[derive(Debug, Clone)]
pub enum Frequency {
    /// Show tick for every data point
    EveryItem,
    /// Show tick every N items
    EveryNthItem(usize),
    /// Show ticks only at the first and last positions
    FirstAndLast,
    /// Custom tick positions (for manual override)
    Custom(Vec<f64>),
}

/// A calculated tick mark with position and label.
#[derive(Debug, Clone)]
pub struct Mark {
    pub position: f64,
    pub label: String,
}

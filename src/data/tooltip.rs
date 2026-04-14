use std::sync::Arc;

/// Information about a single data point for tooltip formatting.
#[derive(Debug, Clone)]
pub struct TooltipEntry {
    /// The x-coordinate of the data point.
    pub x: f64,
    /// The y-coordinate of the data point.
    pub y: f64,
    /// The name of the series, if any.
    pub series_name: Option<String>,
    /// The index of the series within its mark.
    pub series_index: usize,
    /// The index of the mark in the marks list.
    pub mark_index: usize,
    /// The resolved color of this series (matches the line/area/mark color).
    pub color: crate::core::Color,
}

/// Tooltip configuration for a chart.
///
/// Controls how hover tooltips are displayed, including the format
/// function, tracking line, and point markers.
///
/// # Examples
///
/// ```ignore
/// use hyozu::{data, line, Tooltip, Swatch, ColoredText};
///
/// // Default: colored swatch circle + label
/// data(line([10, 20, 30])).tooltip(Tooltip::default())
///
/// // Colored text (no swatch)
/// data(line([10, 20, 30])).tooltip(ColoredText)
///
/// // Swatch + colored text
/// data(line([10, 20, 30])).tooltip(Swatch + ColoredText)
///
/// // Custom format with swatch
/// data(line([10, 20, 30])).tooltip(Swatch + |e: &TooltipEntry| format!("${:.2}", e.y))
///
/// // Custom format with colored text
/// data(line([10, 20, 30])).tooltip(ColoredText + |e: &TooltipEntry| format!("${:.2}", e.y))
/// ```
#[derive(Clone)]
pub struct Tooltip {
    /// Format function that produces the tooltip text for an entry.
    pub(crate) format: Arc<dyn Fn(&TooltipEntry) -> String + Send + Sync>,
    /// Whether to show a colored swatch circle before each entry.
    pub(crate) swatch: bool,
    /// Whether to render each entry's text in the series color.
    pub(crate) colored_text: bool,
    /// Whether to show a vertical tracking line at the cursor x position.
    pub(crate) tracking_line: bool,
    /// Whether to show hover annotations at matched data points
    /// (e.g. circle markers on lines).
    pub(crate) markers: bool,
}

impl std::fmt::Debug for Tooltip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tooltip")
            .field("format", &"<function>")
            .field("swatch", &self.swatch)
            .field("colored_text", &self.colored_text)
            .field("tracking_line", &self.tracking_line)
            .field("markers", &self.markers)
            .finish()
    }
}

/// Default tooltip formatter: shows "name: value" or just "value".
fn default_format(entry: &TooltipEntry) -> String {
    let value = if entry.y.fract().abs() < 0.001 {
        format!("{}", entry.y as i64)
    } else {
        format!("{:.1}", entry.y)
    };

    match &entry.series_name {
        Some(name) => format!("{name}: {value}"),
        None => value,
    }
}

impl Default for Tooltip {
    fn default() -> Self {
        Self {
            format: Arc::new(default_format),
            swatch: true,
            colored_text: false,
            tracking_line: true,
            markers: true,
        }
    }
}

impl Tooltip {
    /// Set a custom format function for tooltip text.
    pub fn format(mut self, f: impl Fn(&TooltipEntry) -> String + Send + Sync + 'static) -> Self {
        self.format = Arc::new(f);
        self
    }

    /// Set whether to show a colored swatch circle before each entry.
    pub fn swatch(mut self, show: bool) -> Self {
        self.swatch = show;
        self
    }

    /// Set whether to render text in the series color.
    pub fn colored_text(mut self, colored: bool) -> Self {
        self.colored_text = colored;
        self
    }

    /// Set whether to show a vertical tracking line.
    pub fn tracking_line(mut self, show: bool) -> Self {
        self.tracking_line = show;
        self
    }

    /// Set whether to show markers on hovered data points.
    pub fn markers(mut self, show: bool) -> Self {
        self.markers = show;
        self
    }
}

// Atom types for composable tooltip construction

/// Show a colored swatch circle before each tooltip entry.
///
/// Can be used directly or combined with other atoms via `+`.
///
/// ```ignore
/// data(line([1, 2, 3])).tooltip(Swatch)
/// data(line([1, 2, 3])).tooltip(Swatch + ColoredText)
/// data(line([1, 2, 3])).tooltip(Swatch + |e: &TooltipEntry| format!("${}", e.y))
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Swatch;

/// Render each tooltip entry's text in the series color.
///
/// Can be used directly or combined with other atoms via `+`.
///
/// ```ignore
/// data(line([1, 2, 3])).tooltip(ColoredText)
/// data(line([1, 2, 3])).tooltip(ColoredText + Swatch)
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ColoredText;

impl From<Swatch> for Tooltip {
    fn from(_: Swatch) -> Self {
        Tooltip {
            swatch: true,
            colored_text: false,
            ..Default::default()
        }
    }
}

impl From<ColoredText> for Tooltip {
    fn from(_: ColoredText) -> Self {
        Tooltip {
            swatch: false,
            colored_text: true,
            ..Default::default()
        }
    }
}

/// A closure producing text gets swatch styling by default.
impl<F> From<F> for Tooltip
where
    F: Fn(&TooltipEntry) -> String + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Tooltip::default().format(f)
    }
}

impl std::ops::Add<ColoredText> for Swatch {
    type Output = Tooltip;
    fn add(self, _rhs: ColoredText) -> Tooltip {
        Tooltip {
            swatch: true,
            colored_text: true,
            ..Default::default()
        }
    }
}

impl std::ops::Add<Swatch> for ColoredText {
    type Output = Tooltip;
    fn add(self, _rhs: Swatch) -> Tooltip {
        Tooltip {
            swatch: true,
            colored_text: true,
            ..Default::default()
        }
    }
}

// Atom + format closure

impl<F: Fn(&TooltipEntry) -> String + Send + Sync + 'static> std::ops::Add<F> for Swatch {
    type Output = Tooltip;
    fn add(self, format: F) -> Tooltip {
        Tooltip::from(Swatch).format(format)
    }
}

impl<F: Fn(&TooltipEntry) -> String + Send + Sync + 'static> std::ops::Add<F> for ColoredText {
    type Output = Tooltip;
    fn add(self, format: F) -> Tooltip {
        Tooltip::from(ColoredText).format(format)
    }
}

// Tooltip + atoms (for chaining after initial construction)

impl std::ops::Add<Swatch> for Tooltip {
    type Output = Tooltip;
    fn add(mut self, _rhs: Swatch) -> Tooltip {
        self.swatch = true;
        self
    }
}

impl std::ops::Add<ColoredText> for Tooltip {
    type Output = Tooltip;
    fn add(mut self, _rhs: ColoredText) -> Tooltip {
        self.colored_text = true;
        self
    }
}

impl<F: Fn(&TooltipEntry) -> String + Send + Sync + 'static> std::ops::Add<F> for Tooltip {
    type Output = Tooltip;
    fn add(self, format: F) -> Tooltip {
        self.format(format)
    }
}

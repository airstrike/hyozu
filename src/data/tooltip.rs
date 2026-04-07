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
#[derive(Clone)]
pub struct Tooltip {
    /// Format function that produces the tooltip text for an entry.
    pub(crate) format: Arc<dyn Fn(&TooltipEntry) -> String + Send + Sync>,
    /// Whether to show a vertical tracking line at the cursor x position.
    pub(crate) tracking_line: bool,
    /// Whether to show markers on the hovered data points.
    pub(crate) markers: bool,
}

impl std::fmt::Debug for Tooltip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tooltip")
            .field("format", &"<function>")
            .field("tracking_line", &self.tracking_line)
            .field("markers", &self.markers)
            .finish()
    }
}

/// Default tooltip formatter: shows "name: value" or just "value".
///
/// Numbers are formatted as integers when they have no fractional part,
/// otherwise with one decimal place.
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

impl<F> From<F> for Tooltip
where
    F: Fn(&TooltipEntry) -> String + Send + Sync + 'static,
{
    fn from(f: F) -> Self {
        Tooltip::default().format(f)
    }
}

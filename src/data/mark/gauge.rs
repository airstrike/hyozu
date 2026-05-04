use crate::color::Color;
use crate::data::axis::tick;
use std::sync::Arc;

/// A colored zone on a gauge chart.
#[derive(Debug, Clone)]
pub struct Zone {
    pub(crate) from: f64,
    pub(crate) to: f64,
    pub(crate) color: Color,
}

/// Gauge chart specification.
///
/// Displays a value on a partial arc, optionally with colored zones.
#[derive(Clone)]
pub struct Gauge {
    /// Current value to display
    pub(crate) value: f64,
    /// Minimum value of the range
    pub(crate) min: f64,
    /// Maximum value of the range
    pub(crate) max: f64,
    /// Optional colored zones on the arc
    pub(crate) zones: Vec<Zone>,
    /// Arc sweep angle in degrees (default 270)
    pub(crate) sweep: f32,
    /// Arc thickness as proportion of radius (default 0.2)
    pub(crate) thickness: f32,
    /// Whether to show the center value label
    pub(crate) show_value: bool,
    /// Optional value format function
    pub(crate) format: Option<Arc<dyn Fn(f64) -> String + Send + Sync>>,
    /// Optional unit label displayed below value
    pub(crate) unit: Option<String>,
    /// Spacing between value text and unit label
    pub(crate) label_spacing: f32,
    /// Optional tick marks
    pub(crate) ticks: Option<tick::Ticks>,
    /// Whether to show tick labels (default true)
    pub(crate) show_tick_labels: bool,
    /// Whether to use gradient arc mode
    pub(crate) gradient: bool,
    /// Number of segments for gradient arc rendering (default 100)
    pub(crate) color_stops: usize,
    /// Opacity of the dimming overlay on unfilled portion (default 0.55)
    pub(crate) dim_opacity: f32,
    /// Whether to show a needle indicator (default false)
    pub(crate) show_needle: bool,
    /// Needle color (None = use text color)
    pub(crate) needle_color: Option<Color>,
    /// Needle line width in pixels (default 2.0)
    pub(crate) needle_width: f32,
    /// Needle length as proportion of inner radius (default 0.9)
    pub(crate) needle_length: f32,
    /// Pivot circle radius as proportion of gauge radius (default 0.03)
    pub(crate) pivot_radius: f32,
    /// Whether to show a dot at the needle tip (default true when needle shown)
    pub(crate) show_needle_tip: bool,
    /// Optional subtitle text below value/unit
    pub(crate) subtitle: Option<String>,
}

impl std::fmt::Debug for Gauge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Gauge")
            .field("value", &self.value)
            .field("min", &self.min)
            .field("max", &self.max)
            .field("zones", &self.zones)
            .field("sweep", &self.sweep)
            .field("thickness", &self.thickness)
            .field("show_value", &self.show_value)
            .field("format", &self.format.as_ref().map(|_| "<function>"))
            .field("unit", &self.unit)
            .field("label_spacing", &self.label_spacing)
            .field("ticks", &self.ticks)
            .field("show_tick_labels", &self.show_tick_labels)
            .field("gradient", &self.gradient)
            .field("color_stops", &self.color_stops)
            .field("dim_opacity", &self.dim_opacity)
            .field("show_needle", &self.show_needle)
            .field("needle_color", &self.needle_color)
            .field("needle_width", &self.needle_width)
            .field("needle_length", &self.needle_length)
            .field("pivot_radius", &self.pivot_radius)
            .field("show_needle_tip", &self.show_needle_tip)
            .field("subtitle", &self.subtitle)
            .finish()
    }
}

/// Creates a gauge chart.
///
/// # Examples
///
/// ```
/// use hyozu::gauge;
///
/// // Simple gauge 0-100
/// let chart = gauge(75.0);
///
/// // Custom range
/// let chart = gauge(75.0).range(0.0, 200.0);
/// ```
pub fn gauge(value: impl Into<f64>) -> Gauge {
    Gauge {
        value: value.into(),
        min: 0.0,
        max: 100.0,
        zones: Vec::new(),
        sweep: 270.0,
        thickness: 0.2,
        show_value: true,
        format: None,
        unit: None,
        label_spacing: 10.0,
        ticks: None,
        show_tick_labels: true,
        gradient: false,
        color_stops: 100,
        dim_opacity: 0.55,
        show_needle: false,
        needle_color: None,
        needle_width: 2.0,
        needle_length: 0.9,
        pivot_radius: 0.03,
        show_needle_tip: false,
        subtitle: None,
    }
}

impl Gauge {
    /// Sets the range for the gauge.
    pub fn range(mut self, min: f64, max: f64) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Adds a colored zone to the gauge.
    ///
    /// Zones are sequential: the first starts at `min`, each subsequent zone
    /// starts where the previous one ended.
    ///
    /// ```
    /// # use hyozu::{gauge, Color};
    /// gauge(75.0)
    ///     .range(0.0, 130.0)
    ///     .zone(50.0, Color::Danger)   // 0 to 50
    ///     .zone(90.0, Color::Success)  // 50 to 90
    ///     .zone(100.0, Color::Warning) // 90 to 100
    ///     .zone(130.0, Color::Danger); // 100 to 130
    /// ```
    pub fn zone(mut self, to: f64, color: impl Into<Color>) -> Self {
        let from = self.zones.last().map_or(self.min, |z| z.to);
        self.zones.push(Zone {
            from,
            to,
            color: color.into(),
        });
        self
    }

    /// Adds multiple colored zones to the gauge.
    ///
    /// Zones are sequential: the first starts at `min`, each subsequent zone
    /// starts where the previous one ended.
    ///
    /// ```
    /// # use hyozu::gauge;
    /// gauge(75.0)
    ///     .range(0.0, 130.0)
    ///     .zones([
    ///         (50.0, 0x4CAF50),  // 0 to 50: green
    ///         (90.0, 0xFFC107),  // 50 to 90: yellow
    ///         (100.0, 0xFF9800), // 90 to 100: amber
    ///         (130.0, 0xF44336), // 100 to 130: red
    ///     ]);
    /// ```
    pub fn zones<C, I>(mut self, zones: I) -> Self
    where
        C: Into<Color>,
        I: IntoIterator<Item = (f64, C)>,
    {
        for (to, color) in zones {
            let from = self.zones.last().map_or(self.min, |z| z.to);
            self.zones.push(Zone {
                from,
                to,
                color: color.into(),
            });
        }
        self
    }

    /// Sets the arc sweep angle in degrees (default 270).
    pub fn sweep(mut self, degrees: f32) -> Self {
        self.sweep = degrees.clamp(90.0, 360.0);
        self
    }

    /// Sets the arc thickness as proportion of radius (default 0.2).
    pub fn thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness.clamp(0.05, 0.5);
        self
    }

    /// Whether to show the center value label (default true).
    pub fn show_value(mut self, show: bool) -> Self {
        self.show_value = show;
        self
    }

    /// Sets a custom format function for the value display.
    pub fn format(mut self, f: impl Fn(f64) -> String + Send + Sync + 'static) -> Self {
        self.format = Some(Arc::new(f));
        self
    }

    /// Sets the unit label displayed below the value.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Sets the spacing between the value text and unit label (default 10.0).
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.label_spacing = spacing;
        self
    }

    /// Adds tick marks to the gauge.
    pub fn ticks(mut self, ticks: impl Into<tick::Ticks>) -> Self {
        self.ticks = Some(ticks.into());
        self
    }

    /// Whether to show tick labels (default true).
    pub fn show_tick_labels(mut self, show: bool) -> Self {
        self.show_tick_labels = show;
        self
    }

    /// Whether to use gradient arc mode (default false).
    pub fn gradient(mut self, gradient: bool) -> Self {
        self.gradient = gradient;
        self
    }

    /// Sets the number of segments for gradient arc rendering (default 100).
    pub fn color_stops(mut self, stops: usize) -> Self {
        self.color_stops = stops.max(2);
        self
    }

    /// Sets the dimming opacity for the unfilled arc portion (default 0.55).
    pub fn dim_by(mut self, opacity: f32) -> Self {
        self.dim_opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Whether to show a needle indicator (default false).
    pub fn needle(mut self, show: bool) -> Self {
        self.show_needle = show;
        self
    }

    /// Sets the needle color (default: text color from theme).
    pub fn needle_color(mut self, color: impl Into<Color>) -> Self {
        self.needle_color = Some(color.into());
        self
    }

    /// Sets the needle line width in pixels (default 2.0).
    pub fn needle_width(mut self, width: f32) -> Self {
        self.needle_width = width.max(0.5);
        self
    }

    /// Sets the needle length as proportion of inner radius (default 0.9).
    pub fn needle_length(mut self, length: f32) -> Self {
        self.needle_length = length.clamp(0.3, 1.1);
        self
    }

    /// Sets the pivot circle radius as proportion of gauge radius (default 0.03).
    pub fn pivot_radius(mut self, radius: f32) -> Self {
        self.pivot_radius = radius.clamp(0.0, 0.1);
        self
    }

    /// Whether to show a dot at the needle tip (default true).
    pub fn needle_tip(mut self, show: bool) -> Self {
        self.show_needle_tip = show;
        self
    }

    /// Sets the subtitle text displayed below value/unit.
    pub fn subtitle(mut self, text: impl Into<String>) -> Self {
        self.subtitle = Some(text.into());
        self
    }

    /// Returns the current value.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the range.
    pub fn range_values(&self) -> (f64, f64) {
        (self.min, self.max)
    }

    /// Gauge charts have no axes.
    pub fn x_axis() -> Option<crate::data::Axis> {
        None
    }

    /// Gauge charts have no axes.
    pub fn y_axis() -> Option<crate::data::Axis> {
        None
    }
}

impl<Message, Theme, Renderer> From<Gauge> for crate::Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(gauge: Gauge) -> Self {
        <Gauge as crate::data::IntoData<Message, Theme, Renderer>>::into_data(gauge)
    }
}

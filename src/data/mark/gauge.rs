use crate::color::Color;
use std::sync::Arc;

/// A colored zone on a gauge chart.
#[derive(Debug, Clone)]
pub struct Zone {
    pub(crate) from: f64,
    pub(crate) to: f64,
    pub(crate) color: Color,
}

/// Creates a gauge zone.
pub fn zone(from: f64, to: f64, color: impl Into<Color>) -> Zone {
    Zone {
        from,
        to,
        color: color.into(),
    }
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
    /// Whether to show min/max labels at arc ends
    pub(crate) show_min_max: bool,
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
            .field("show_min_max", &self.show_min_max)
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
        show_min_max: false,
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
    pub fn zone(mut self, from: f64, to: f64, color: impl Into<Color>) -> Self {
        self.zones.push(Zone {
            from,
            to,
            color: color.into(),
        });
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
    pub fn format(
        mut self,
        f: impl Fn(f64) -> String + Send + Sync + 'static,
    ) -> Self {
        self.format = Some(Arc::new(f));
        self
    }

    /// Sets the unit label displayed below the value.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Whether to show min/max labels at arc ends (default false).
    pub fn show_min_max(mut self, show: bool) -> Self {
        self.show_min_max = show;
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

impl From<Gauge> for crate::Data {
    fn from(gauge: Gauge) -> Self {
        use crate::data::IntoData;
        gauge.into_data()
    }
}

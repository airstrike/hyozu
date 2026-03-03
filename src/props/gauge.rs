//! Property descriptors for gauge charts.

use crate::map::Map;

/// A property of a gauge chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Current value to display
    Value(f64),
    /// Arc sweep angle in degrees
    Sweep(f32),
    /// Arc thickness as proportion of radius
    Thickness(f32),
    /// Whether to show the center value label
    ShowValue(bool),
    /// Whether to show min/max labels at arc ends
    ShowMinMax(bool),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Gauge mark.
    pub fn apply(&self, gauge: &mut crate::data::mark::gauge::Gauge) {
        match self {
            Property::Value(v) => gauge.value = *v,
            Property::Sweep(v) => gauge.sweep = v.clamp(90.0, 360.0),
            Property::Thickness(v) => gauge.thickness = v.clamp(0.05, 0.5),
            Property::ShowValue(v) => gauge.show_value = *v,
            Property::ShowMinMax(v) => gauge.show_min_max = *v,
        }
    }
}

pub use Property::{ShowMinMax, ShowValue, Sweep, Thickness, Value};

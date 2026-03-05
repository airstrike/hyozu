//! Property descriptors for heatmap charts.

use crate::data::mark::heatmap;
use crate::map::Map;

/// A property of a heatmap chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Whether to show value labels inside cells.
    ShowLabels(bool),
    /// Explicit value range for color mapping (None = auto).
    ValueRange(Option<(f64, f64)>),
    /// Gradient stops for color mapping.
    ColorStops(Vec<crate::core::Color>),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Heatmap mark.
    pub fn apply(&self, hm: &mut heatmap::Heatmap) {
        match self {
            Property::ShowLabels(show) => hm.show_labels = *show,
            Property::ValueRange(range) => hm.value_range = *range,
            Property::ColorStops(stops) => hm.color_stops = stops.clone(),
        }
    }
}

pub use Property::{ColorStops, ShowLabels, ValueRange};

//! Property descriptors for box plot charts.

use crate::data::mark::boxplot;
use crate::map::Map;

/// A property of a box plot chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Direction (vertical or horizontal)
    Direction(boxplot::Direction),
    /// Box width proportion (0.1 to 1.0)
    Width(f32),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a BoxPlot mark.
    pub fn apply(&self, bp: &mut boxplot::BoxPlot) {
        match self {
            Property::Direction(d) => bp.direction = *d,
            Property::Width(w) => bp.width = w.clamp(0.1, 1.0),
        }
    }
}

pub use Property::{Direction, Width};

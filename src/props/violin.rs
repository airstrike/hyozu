//! Property descriptors for violin charts.

use crate::data::mark::violin;
use crate::map::Map;

/// A property of a violin chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// The orientation direction
    Direction(violin::Direction),
    /// Max width proportion (0.1..=1.0)
    Width(f32),
    /// Whether to show mini box plot overlay
    ShowBox(bool),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Violin mark.
    pub fn apply(&self, v: &mut violin::Violin) {
        match self {
            Property::Direction(d) => v.direction = *d,
            Property::Width(w) => v.width = w.clamp(0.1, 1.0),
            Property::ShowBox(show) => v.show_box = *show,
        }
    }
}

pub use Property::{Direction, ShowBox, Width};

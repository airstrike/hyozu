//! Property descriptors for bar labels.

use crate::data::mark::bar;
use crate::map::Map;

/// A property of a bar label.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Label position
    Position(bar::label::Position),
}

impl Map for Property {}

impl Property {
    /// Applies this property to all series in a Bars mark.
    pub fn apply(&self, bars: &mut bar::Bars) {
        match self {
            Property::Position(p) => {
                for series in bars.series_mut() {
                    series.set_label_position(*p);
                }
            }
        }
    }
}

// Re-export enum variant as constructor function
pub use Property::Position;

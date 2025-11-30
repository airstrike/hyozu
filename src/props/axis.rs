//! Property descriptors for chart axes.

use crate::data::{Axis, axis};
use crate::map::Map;

/// A property of a chart axis.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Axis labels
    Labels(Vec<String>),
    /// Label placement (on ticks or between ticks)
    Placement(axis::Placement),
    /// Tick alignment mode
    TickAlignment(axis::Alignment),
    /// Whether the axis is visible
    Visible(bool),
}

impl Map for Property {}

impl Property {
    /// Applies this property to an Axis.
    pub fn apply(&self, axis: &mut Axis) {
        match self {
            Property::Labels(labels) => {
                *axis = axis.clone().labels(labels.clone());
            }
            Property::Placement(placement) => {
                *axis = axis.clone().labels(*placement);
            }
            Property::TickAlignment(alignment) => {
                *axis = axis.clone().with_ticks(*alignment);
            }
            Property::Visible(visible) => {
                *axis = axis.clone().show_labels(*visible).show_line(*visible);
            }
        }
    }
}

// Re-export enum variants as constructor functions
pub use Property::{Labels, Placement, TickAlignment, Visible};

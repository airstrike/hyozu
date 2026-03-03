//! Property descriptors for waterfall charts.

use crate::map::Map;

/// A property of a waterfall chart.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// Whether to draw connector lines between bars
    Connector(bool),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Waterfall mark.
    pub fn apply(
        &self,
        waterfall: &mut crate::data::mark::waterfall::Waterfall,
    ) {
        match self {
            Property::Connector(v) => waterfall.connector = *v,
        }
    }
}

pub use Property::Connector;

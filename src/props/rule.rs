//! Property descriptors for rule (reference line) marks.

use crate::color::Color;
use crate::map::Map;

/// A property of a rule mark.
#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    /// The value where the rule line is drawn
    Value(f64),
    /// Line color
    Color(Option<Color>),
    /// Line width
    Width(f32),
}

impl Map for Property {}

impl Property {
    /// Applies this property to a Rule mark.
    pub fn apply(&self, rule: &mut crate::data::mark::rule::Rule) {
        match self {
            Property::Value(v) => rule.value = *v,
            Property::Color(c) => rule.color = *c,
            Property::Width(w) => rule.width = *w,
        }
    }
}

/// Wraps a value into a rule property.
#[allow(non_snake_case)]
pub fn Value(value: f64) -> Property {
    Property::Value(value)
}

/// Wraps a color into a rule property.
#[allow(non_snake_case)]
pub fn Color(value: Option<Color>) -> Property {
    Property::Color(value)
}

/// Wraps a width into a rule property.
#[allow(non_snake_case)]
pub fn Width(value: f32) -> Property {
    Property::Width(value)
}

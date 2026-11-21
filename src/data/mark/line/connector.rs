//! Thin guide lines that Excel attaches to line and area series: drop lines
//! and high-low lines.

use super::LineStyle;
use crate::color::Color;

/// The stroke of a drop line or a high-low line.
///
/// Unset, the color resolves to the theme's text color at reduced strength,
/// as Excel draws these lines in a neutral ink rather than a series color.
#[derive(Debug, Clone, PartialEq)]
pub struct Connector {
    color: Option<Color>,
    width: f32,
    style: LineStyle,
}

impl Default for Connector {
    fn default() -> Self {
        Self {
            color: None,
            width: 1.0,
            style: LineStyle::Solid,
        }
    }
}

impl Connector {
    /// A one-pixel solid connector in the theme's ink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the stroke color.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the stroke width in pixels, at least a quarter pixel.
    pub fn width(mut self, width: f32) -> Self {
        self.width = width.max(0.25);
        self
    }

    /// Sets the dash pattern.
    pub fn style(mut self, style: LineStyle) -> Self {
        self.style = style;
        self
    }

    /// Returns the stroke color, if set.
    pub fn color_value(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    /// Returns the stroke width in pixels.
    pub fn width_value(&self) -> f32 {
        self.width
    }

    /// Returns the dash pattern.
    pub fn style_value(&self) -> &LineStyle {
        &self.style
    }
}

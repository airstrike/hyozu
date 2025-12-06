use crate::color::Color;

/// Shape of the marker drawn at each data point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shape {
    /// A filled circle
    #[default]
    Circle,
    /// A filled square
    Square,
    /// A filled diamond (rotated square)
    Diamond,
    /// A filled triangle pointing up
    Triangle,
    /// A filled triangle pointing down
    TriangleDown,
    /// A cross/plus sign (+)
    Cross,
    /// An X shape
    X,
}

/// Controls which points show markers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Show {
    /// Show markers on all points
    #[default]
    Any,
    /// Show marker only on the first point
    FirstOnly,
    /// Show marker only on the last point
    LastOnly,
    /// Show markers on both first and last points
    FirstAndLast,
    /// Show markers on first occurrence of min and max Y values
    MinMaxFirst,
    /// Show markers on all occurrences of min and max Y values
    MinMaxAll,
    /// Show markers on last occurrence of min and max Y values
    MinMaxLast,
}

/// Marker configuration for line charts.
#[derive(Debug, Clone)]
pub struct Marker {
    /// Shape of the marker
    pub shape: Shape,
    /// Which points show markers
    pub show: Show,
    /// Size of the marker (diameter/width)
    pub size: f32,
    /// Fill color (None = use line color)
    pub color: Option<Color>,
    /// Stroke color for outline (None = no outline)
    pub stroke: Option<Color>,
    /// Stroke width for outline
    pub stroke_width: f32,
}

impl Default for Marker {
    fn default() -> Self {
        Self {
            shape: Shape::Circle,
            show: Show::Any,
            size: 6.0,
            color: None,
            stroke: None,
            stroke_width: 1.5,
        }
    }
}

impl Marker {
    /// Create a new marker configuration with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the marker shape.
    pub fn shape(mut self, shape: Shape) -> Self {
        self.shape = shape;
        self
    }

    /// Set which points show markers.
    pub fn show(mut self, show: Show) -> Self {
        self.show = show;
        self
    }

    /// Set the marker size (diameter/width in pixels).
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Set the fill color. If not set, uses the line color.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Set the stroke (outline) color.
    pub fn stroke(mut self, color: impl Into<Color>) -> Self {
        self.stroke = Some(color.into());
        self
    }

    /// Set the stroke width for the outline.
    pub fn stroke_width(mut self, width: f32) -> Self {
        self.stroke_width = width;
        self
    }
}

// Ergonomic conversions: Shape -> Marker

impl From<Shape> for Marker {
    fn from(shape: Shape) -> Self {
        Marker {
            shape,
            ..Default::default()
        }
    }
}

impl From<Show> for Marker {
    fn from(show: Show) -> Self {
        Marker {
            show,
            ..Default::default()
        }
    }
}

impl From<Shape> for Option<Marker> {
    fn from(shape: Shape) -> Self {
        Some(Marker::from(shape))
    }
}

impl From<Show> for Option<Marker> {
    fn from(show: Show) -> Self {
        Some(Marker::from(show))
    }
}

// Operator overloading for fluent configuration: Shape + Show, etc.

impl std::ops::Add<Show> for Shape {
    type Output = Marker;
    fn add(self, rhs: Show) -> Marker {
        Marker {
            shape: self,
            show: rhs,
            ..Default::default()
        }
    }
}

impl std::ops::Add<Shape> for Show {
    type Output = Marker;
    fn add(self, rhs: Shape) -> Marker {
        Marker {
            shape: rhs,
            show: self,
            ..Default::default()
        }
    }
}

impl std::ops::Add<Shape> for Marker {
    type Output = Marker;
    fn add(self, shape: Shape) -> Marker {
        self.shape(shape)
    }
}

impl std::ops::Add<Show> for Marker {
    type Output = Marker;
    fn add(self, show: Show) -> Marker {
        self.show(show)
    }
}

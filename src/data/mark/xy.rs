use crate::color::Color;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};
use crate::data::mark::line::marker;
use crate::data::{Datum, IntoDatums};

pub use marker::Marker;

/// XY scatter chart specification.
///
/// Plots points on a two-dimensional coordinate system without connecting lines.
#[derive(Debug, Clone)]
pub struct Xy {
    pub(crate) points: Vec<Datum>,
    pub(crate) color: Option<Color>,
    pub(crate) marker: marker::Marker,
    /// Optional name for this scatter series (used in legends).
    pub(crate) name: Option<String>,
}

/// Creates an XY scatter chart from point data.
///
/// # Examples
///
/// ```
/// use hyozu::xy;
///
/// // From tuples
/// let chart = xy([(1.0, 2.0), (3.0, 4.0), (5.0, 1.0)]);
///
/// // From auto-enumerated values
/// let chart = xy([10, 20, 15, 25]);
/// ```
pub fn xy(data: impl IntoDatums) -> Xy {
    Xy {
        points: data.into_datums(),
        color: None,
        marker: marker::Marker::default(),
        name: None,
    }
}

impl Xy {
    /// Sets the color for the scatter points.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the marker shape and configuration.
    pub fn markers(mut self, marker: impl Into<marker::Marker>) -> Self {
        self.marker = marker.into();
        self
    }

    /// Sets the name for this scatter series (used in legends).
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    // === Property setters (in-place) ===

    /// Sets the series color in place.
    pub fn set_color(&mut self, color: Option<Color>) {
        self.color = color;
    }

    /// Replaces the marker configuration.
    pub fn set_marker(&mut self, marker: marker::Marker) {
        self.marker = marker;
    }

    /// Sets the marker shape in place.
    pub fn set_marker_shape(&mut self, shape: marker::Shape) {
        self.marker.set_shape(shape);
    }

    /// Sets the marker show mode in place.
    pub fn set_marker_show(&mut self, show: marker::Show) {
        self.marker.set_show(show);
    }

    /// Sets the marker size in place.
    pub fn set_marker_size(&mut self, size: f32) {
        self.marker.set_size(size);
    }

    /// Sets the marker fill color in place. `None` falls back to the series color.
    pub fn set_marker_color(&mut self, color: Option<Color>) {
        self.marker.set_color(color);
    }

    /// Sets the marker stroke color in place. `None` removes the outline.
    pub fn set_marker_stroke(&mut self, stroke: Option<Color>) {
        self.marker.set_stroke(stroke);
    }

    /// Sets the marker stroke width in place.
    pub fn set_marker_stroke_width(&mut self, width: f32) {
        self.marker.set_stroke_width(width);
    }

    /// Returns a mutable reference to the marker configuration.
    pub fn marker_mut(&mut self) -> &mut marker::Marker {
        &mut self.marker
    }

    // === Property getters ===

    /// Returns the name of this scatter series.
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the series color, if any.
    pub fn color_value(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    /// Returns a reference to the marker configuration.
    pub fn marker(&self) -> &marker::Marker {
        &self.marker
    }

    /// Returns the points.
    pub fn points(&self) -> &[Datum] {
        &self.points
    }

    /// Creates the appropriate x-axis for a scatter chart (scalar).
    pub fn x_axis() -> Axis {
        Axis::new(Orientation::Bottom)
            .with_kind(Kind::Scalar)
            .labels(Placement::OnTicks)
            .with_ticks(axis::tick::Ticks::continuous())
    }

    /// Creates the appropriate y-axis for a scatter chart (scalar).
    pub fn y_axis() -> Axis {
        Axis::new(Orientation::Left)
            .with_kind(Kind::Scalar)
            .with_ticks(axis::tick::Ticks::continuous())
    }
}

impl From<Xy> for crate::Data {
    fn from(xy: Xy) -> Self {
        use crate::data::IntoData;
        xy.into_data()
    }
}

use crate::color::Color;
use crate::data::axis::{self, Axis, Kind, Orientation, Placement};
use crate::data::mark::line::marker;
use crate::data::{Datum, IntoDatums};
use crate::encoding::{Encoding, channel};
use crate::feature;

pub use marker::Marker;

/// How each [`Datum`]'s `(x, y)` coordinate is interpreted by the renderer.
///
/// `Cartesian` (the default) routes points through the chart's primary
/// axes plane: `x` and `y` are values on whatever scales the axes carry.
///
/// `Geo` interprets `(x, y)` as `(longitude, latitude)` in degrees and
/// projects through the chart's geo plane (configured via
/// [`crate::Data::geo`] / [`crate::Data::geo_data`]). When no geo data
/// is configured, points project to the plot-area origin — the renderer
/// gates draw on `state.geo_plane.is_some()` so a misconfigured chart
/// shows no points rather than a NaN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoordKind {
    #[default]
    Cartesian,
    Geo,
}

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
    /// Optional size-channel encoding: binds each point's marker diameter to
    /// a closure over its datum. See [`crate::encoding::size_by`]. When set,
    /// the renderer uses the resolved per-point diameter instead of
    /// `marker.size`. This turns the scatter into a bubble chart.
    pub(crate) size_by: Option<Encoding<channel::Size>>,
    /// How `(x, y)` coordinates are interpreted. See [`CoordKind`].
    pub(crate) coord_kind: CoordKind,
    /// Marker fill opacity multiplier (0.0 = fully transparent, 1.0 =
    /// opaque). Multiplies the resolved fill color's alpha channel at
    /// draw time; defaults to `1.0` so cartesian scatter is unchanged.
    pub(crate) opacity: f32,
    /// Optional per-point labels surfaced by the hover tooltip and (in
    /// the future) per-point text overlays. Either empty (no labels) or
    /// aligned 1:1 with [`Self::points`]. Populated by
    /// [`crate::bubble_map`] from each `MapPoint::label`; cartesian Xy
    /// charts leave it empty.
    pub(crate) labels: Vec<Option<String>>,
    /// Optional per-point values surfaced by the hover tooltip in place
    /// of `Datum.y`. Either empty (use `Datum.y`) or aligned 1:1 with
    /// [`Self::points`]. Populated by [`crate::bubble_map`] from each
    /// `MapPoint::value` so geo bubbles report their original magnitude
    /// instead of latitude.
    pub(crate) tooltip_values: Vec<Option<f64>>,
    /// Optional per-point feature ids used by geographic point marks.
    /// Either empty (non-addressable points) or aligned 1:1 with
    /// [`Self::points`]. Populated by [`crate::bubble_map`] from each
    /// `MapPoint::id` so map clicks can emit the same
    /// [`crate::target::Target::Feature`] shape as choropleths.
    pub(crate) feature_ids: Vec<Option<feature::Id>>,
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
        size_by: None,
        coord_kind: CoordKind::Cartesian,
        opacity: 1.0,
        labels: Vec::new(),
        tooltip_values: Vec::new(),
        feature_ids: Vec::new(),
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

    /// Encode the size channel: compute each point's marker diameter from
    /// its datum via the given [`Encoding`]. Replaces any previous encoding.
    ///
    /// Turns an XY scatter into a bubble chart. Resolved diameters fall back
    /// to `marker.size` for any point where the encoding returns `None`
    /// (typically a non-finite input).
    ///
    /// # Example
    ///
    /// ```
    /// use hyozu::{encoding, xy};
    ///
    /// let chart = xy([(1.0, 10.0), (2.0, 40.0), (3.0, 25.0)])
    ///     .size_by(encoding::size_by(|_, d| d.y).range(4.0..=24.0));
    /// ```
    pub fn size_by(mut self, encoding: Encoding<channel::Size>) -> Self {
        self.size_by = Some(encoding);
        self
    }

    /// Sets `(x, y)` interpretation to `(longitude, latitude)`. Renders
    /// via the chart's geo plane (configured via [`crate::Data::geo`]).
    /// When no geo data is configured, points project to the plot-area
    /// origin `(0, 0)` — the renderer is gated on `state.geo_plane`, so
    /// a misconfigured chart shows no points rather than a NaN.
    ///
    /// Pairs with [`Xy::size_by`] for bubble-sized geographic scatter.
    pub fn on_geo(mut self) -> Self {
        self.coord_kind = CoordKind::Geo;
        self
    }

    /// Sets the marker fill opacity (a multiplier on the resolved fill
    /// color's alpha channel; `0.0` = fully transparent, `1.0` = opaque).
    /// Inputs outside `[0.0, 1.0]` are clamped at draw time.
    pub fn opacity(mut self, o: f32) -> Self {
        self.opacity = o;
        self
    }

    /// Resolve the displayed marker diameter for a single point.
    ///
    /// Priority: 1) `size_by` encoding, 2) the marker's `size` fallback.
    /// Mirrors `bar::Series::resolved_color_at` so the bubble path and any
    /// future hit-test/tooltip path agree on what diameter a given point
    /// shows.
    ///
    /// `fallback` is what the renderer falls back to when no encoding is
    /// configured or when the encoding returns `None` for this point —
    /// typically `marker.size`.
    pub fn resolved_size_at(&self, i: usize, fallback: f32) -> f32 {
        if let Some(enc) = &self.size_by
            && let Some(Some(s)) = enc.resolve_size(&self.points).get(i).copied()
        {
            s
        } else {
            fallback
        }
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

    /// Returns the feature id for a point, if this XY mark carries
    /// addressable geographic points.
    pub fn feature_id_at(&self, index: usize) -> Option<&feature::Id> {
        self.feature_ids.get(index).and_then(Option::as_ref)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xy_defaults_to_cartesian() {
        let mark = xy([(1.0, 2.0)]);
        assert_eq!(mark.coord_kind, CoordKind::Cartesian);
    }

    #[test]
    fn on_geo_flips_coord_kind() {
        let mark = xy([(1.0, 2.0)]).on_geo();
        assert_eq!(mark.coord_kind, CoordKind::Geo);
    }

    #[test]
    fn opacity_defaults_to_one() {
        let mark = xy([(1.0, 2.0)]);
        assert_eq!(mark.opacity, 1.0);
    }

    #[test]
    fn opacity_setter_writes_through() {
        let mark = xy([(1.0, 2.0)]).opacity(0.5);
        assert_eq!(mark.opacity, 0.5);
    }
}

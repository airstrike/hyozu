use crate::color::Color;
use crate::data::Datum;
use crate::data::mark::line::marker;
use crate::data::mark::xy::{CoordKind, Xy};
use crate::encoding;

/// A single point on a bubble map.
#[derive(Debug, Clone)]
pub struct MapPoint {
    /// Latitude in degrees (-90 to +90).
    pub(crate) lat: f64,
    /// Longitude in degrees (-180 to +180).
    pub(crate) lon: f64,
    /// Magnitude — drives bubble radius.
    pub(crate) value: f64,
    /// Optional override color.
    pub(crate) color: Option<Color>,
    /// Optional text label shown near the bubble.
    pub(crate) label: Option<String>,
    /// Optional name for this point (used in legends).
    pub(crate) name: Option<String>,
    /// Optional feature id. When set, a click on this bubble emits
    /// `Target::Feature { mark, id }` — matching the docstring on
    /// `target::Target::Feature` that describes "choropleth or bubble map"
    /// features. Bubbles without an id remain click-silent (backward-
    /// compatible).
    pub(crate) id: Option<crate::feature::Id>,
}

/// Creates a map point at the given latitude and longitude with a value.
pub fn map_point(lat: impl Into<f64>, lon: impl Into<f64>, value: impl Into<f64>) -> MapPoint {
    MapPoint {
        lat: lat.into(),
        lon: lon.into(),
        value: value.into(),
        color: None,
        label: None,
        name: None,
        id: None,
    }
}

impl MapPoint {
    /// Sets the color for this point.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the label shown near the bubble.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the name for this point (used in legends).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the feature id used when this bubble is clicked. A click
    /// emits `Action::Clicked(Target::Feature { mark, id })`; unset points
    /// are click-silent. Accepts anything convertible into an
    /// [`Id`](crate::feature::Id) (`&str`, `String`, or an existing `Id`).
    pub fn id(mut self, id: impl Into<crate::feature::Id>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn get_lat(&self) -> f64 {
        self.lat
    }

    pub fn get_lon(&self) -> f64 {
        self.lon
    }

    pub fn get_value(&self) -> f64 {
        self.value
    }

    pub fn get_color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    pub fn get_label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn get_id(&self) -> Option<&crate::feature::Id> {
        self.id.as_ref()
    }
}

/// Bubble map chart specification.
///
/// Renders a world map background with sized bubbles at geographic coordinates.
/// Bubble radius is proportional to value.
#[derive(Debug, Clone)]
pub struct BubbleMap {
    pub(crate) points: Vec<MapPoint>,
    /// Minimum bubble radius in pixels.
    pub(crate) min_radius: f32,
    /// Maximum bubble radius in pixels.
    pub(crate) max_radius: f32,
    /// Bubble opacity (0.0–1.0).
    pub(crate) opacity: f32,
}

/// Creates a bubble map from a collection of map points.
///
/// Returns an [`Xy`] configured for geographic projection: each point's
/// `(lon, lat)` is projected through the chart's geo plane (configured via
/// [`crate::Data::geo`]), and a sqrt-area size encoding maps the magnitudes
/// to bubble diameters in the range `8.0..=60.0` px (i.e. radii `4..=30`,
/// matching the historical bubble-map defaults). The marker fill uses the
/// per-mark opacity multiplier of `0.7`.
///
/// # Examples
///
/// ```
/// use hyozu::{bubble_map, map_point};
///
/// let chart = bubble_map([
///     map_point(40.7, -74.0, 23.3).label("New York"),
///     map_point(34.0, -118.2, 21.2).label("Los Angeles"),
///     map_point(52.2, 21.0, 29.0).label("Warsaw"),
/// ]);
/// ```
pub fn bubble_map(points: impl IntoIterator<Item = MapPoint>) -> Xy {
    /// Historical bubble-map defaults — kept here so `bubble_map(...)`
    /// still produces visually identical bubbles after the migration to
    /// `Xy::on_geo()` + a `size_by` encoding. Diameters are 2× the radii.
    const MIN_RADIUS: f32 = 4.0;
    const MAX_RADIUS: f32 = 30.0;
    const MIN_DIAMETER: f32 = 2.0 * MIN_RADIUS;
    const MAX_DIAMETER: f32 = 2.0 * MAX_RADIUS;

    let entries: Vec<MapPoint> = points.into_iter().collect();

    // Match the legacy renderer's `value_range`: take `value.abs()`,
    // collapse a degenerate or all-non-finite domain to a `+1.0` span so
    // the normalizer below can't divide by zero or produce a NaN.
    let (v_lo, v_hi) = compute_value_range(&entries);

    // Capture the magnitudes alongside the datums so the size closure
    // can look the value up by point index. Datum carries only `(x, y)`,
    // and the size encoding's extractor receives `(usize, &Datum)` — the
    // index is what bridges the two parallel vectors.
    let values: Vec<f64> = entries.iter().map(|e| e.value.abs()).collect();
    let datums: Vec<Datum> = entries.iter().map(|e| Datum::new(e.lon, e.lat)).collect();

    let size_encoding = encoding::size_by(move |i, _d| {
        // The closure returns the desired pixel diameter directly. The
        // outer encoding is configured as a `linear` identity passthrough
        // (domain == range == diameter range) so the diameter we compute
        // here lands in `state.resolved_sizes` unchanged. Sqrt-of-`t` is
        // the historical bubble_map formula (area-proportional, matches
        // `chart::plot_area::bubble_map::interpolate_radius`).
        let v = values.get(i).copied().unwrap_or(0.0);
        let span = v_hi - v_lo;
        let t = if span > 0.0 {
            ((v - v_lo) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let radius = MIN_RADIUS as f64 + t.sqrt() * (MAX_RADIUS - MIN_RADIUS) as f64;
        2.0 * radius
    })
    .domain(MIN_DIAMETER as f64..=MAX_DIAMETER as f64)
    .range(MIN_DIAMETER..=MAX_DIAMETER)
    .linear();

    // Construct the Xy via a struct literal because `mark::xy::xy(...)`
    // takes `impl IntoDatums` (which builds datums from numeric tuples or
    // arrays) and there's no identity impl for `Vec<Datum>`. We already
    // have the datums in hand here, so the literal is the direct path —
    // the rest of the field defaults mirror `mark::xy::xy(...)`.
    Xy {
        points: datums,
        color: None,
        marker: marker::Marker::default(),
        name: None,
        size_by: Some(size_encoding),
        coord_kind: CoordKind::Geo,
        opacity: 0.7,
    }
}

/// Compute the value-magnitude range over a slice of map points. Mirrors
/// the historical `chart::plot_area::bubble_map::value_range` so the
/// migrated `bubble_map(...)` produces visually identical bubbles for
/// degenerate, empty, and all-non-finite inputs.
fn compute_value_range(points: &[MapPoint]) -> (f64, f64) {
    let mut v_lo = f64::INFINITY;
    let mut v_hi = f64::NEG_INFINITY;
    for pt in points {
        let v = pt.value.abs();
        if v.is_finite() {
            if v < v_lo {
                v_lo = v;
            }
            if v > v_hi {
                v_hi = v;
            }
        }
    }
    if !v_lo.is_finite() {
        v_lo = 0.0;
    }
    if !v_hi.is_finite() || v_hi <= v_lo {
        v_hi = v_lo + 1.0;
    }
    (v_lo, v_hi)
}

/// Trait for converting various inputs into a BubbleMap.
pub trait IntoBubbleMap {
    fn into_bubble_map(self) -> BubbleMap;
}

impl<const N: usize> IntoBubbleMap for [MapPoint; N] {
    fn into_bubble_map(self) -> BubbleMap {
        BubbleMap {
            points: self.into(),
            min_radius: 4.0,
            max_radius: 30.0,
            opacity: 0.7,
        }
    }
}

impl IntoBubbleMap for Vec<MapPoint> {
    fn into_bubble_map(self) -> BubbleMap {
        BubbleMap {
            points: self,
            min_radius: 4.0,
            max_radius: 30.0,
            opacity: 0.7,
        }
    }
}

impl BubbleMap {
    /// Sets the minimum bubble radius in pixels.
    pub fn min_radius(mut self, r: f32) -> Self {
        self.min_radius = r.max(1.0);
        self
    }

    /// Sets the maximum bubble radius in pixels.
    pub fn max_radius(mut self, r: f32) -> Self {
        self.max_radius = r.max(self.min_radius);
        self
    }

    /// Sets the bubble opacity (0.0–1.0).
    pub fn opacity(mut self, a: f32) -> Self {
        self.opacity = a.clamp(0.0, 1.0);
        self
    }

    /// Returns the points.
    pub fn points(&self) -> &[MapPoint] {
        &self.points
    }

    /// Returns a mutable reference to the points.
    pub fn points_mut(&mut self) -> &mut Vec<MapPoint> {
        &mut self.points
    }

    /// Bubble maps have no axes.
    pub fn x_axis() -> Option<crate::data::Axis> {
        None
    }

    /// Bubble maps have no axes.
    pub fn y_axis() -> Option<crate::data::Axis> {
        None
    }
}

impl From<BubbleMap> for crate::Data {
    fn from(bm: BubbleMap) -> Self {
        use crate::data::IntoData;
        bm.into_data()
    }
}

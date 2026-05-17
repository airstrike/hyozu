use crate::color::Color;
use crate::data::Datum;
use crate::data::mark::Mark;
use crate::data::mark::line::marker;
use crate::data::mark::text::{self, TextItem};
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
/// Minimum and maximum bubble radii in pixels for [`bubble_map`].
/// Exposed so callers building label or annotation overlays can compute
/// per-bubble offsets that match the rendered radius.
pub const MIN_RADIUS: f32 = 4.0;
pub const MAX_RADIUS: f32 = 30.0;

/// Returns the rendered radius (in pixels) for a bubble whose magnitude
/// is `value` within the given `(v_lo, v_hi)` range — same formula
/// [`bubble_map`] uses internally (sqrt of the linearly-normalized
/// value scaled into the [`MIN_RADIUS`]..[`MAX_RADIUS`] band).
///
/// Use this when composing label or annotation overlays alongside a
/// bubble map so labels can sit at a radius-aware offset (e.g. just
/// above each bubble's top edge) instead of a uniform pixel nudge that
/// goes wrong on the largest bubbles. Returns [`MIN_RADIUS`] when the
/// value is non-finite or below `v_lo`.
pub fn radius_for(value: f64, value_range: (f64, f64)) -> f32 {
    let (v_lo, v_hi) = value_range;
    if !value.is_finite() {
        return MIN_RADIUS;
    }
    let span = v_hi - v_lo;
    let t = if span > 0.0 {
        ((value.abs() - v_lo) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    MIN_RADIUS + (t.sqrt() as f32) * (MAX_RADIUS - MIN_RADIUS)
}

pub fn bubble_map(points: impl IntoIterator<Item = MapPoint>) -> Xy {
    const MIN_DIAMETER: f32 = 2.0 * MIN_RADIUS;
    const MAX_DIAMETER: f32 = 2.0 * MAX_RADIUS;

    let entries: Vec<MapPoint> = points.into_iter().collect();
    let (v_lo, v_hi) = compute_value_range(&entries);

    // Capture the magnitudes alongside the datums so the size closure
    // can look the value up by point index. Datum carries only `(x, y)`,
    // and the size encoding's extractor receives `(usize, &Datum)` — the
    // index is what bridges the two parallel vectors.
    let values: Vec<f64> = entries.iter().map(|e| e.value.abs()).collect();
    let datums: Vec<Datum> = entries.iter().map(|e| Datum::new(e.lon, e.lat)).collect();
    let labels: Vec<Option<String>> = entries.iter().map(|e| e.label.clone()).collect();
    let tooltip_values: Vec<Option<f64>> = entries.iter().map(|e| Some(e.value)).collect();
    let feature_ids: Vec<Option<crate::feature::Id>> = entries.iter().map(|e| e.id.clone()).collect();

    let size_encoding = encoding::size_by(move |i, _d| {
        // The closure returns the desired pixel diameter directly. The
        // outer encoding is configured as a `linear` identity passthrough
        // (domain == range == diameter range) so the diameter we compute
        // here lands in `state.resolved_sizes` unchanged. `t.sqrt()` makes
        // bubble area (not radius) proportional to value — the standard
        // perceptual scaling for bubble charts.
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

    // `mark::xy::xy(...)` takes `impl IntoDatums` and has no identity impl
    // for `Vec<Datum>`; we already have datums in hand, so a struct literal
    // is the direct path. Field defaults mirror `mark::xy::xy(...)`.
    Xy {
        points: datums,
        color: None,
        marker: marker::Marker::default(),
        name: None,
        size_by: Some(size_encoding),
        coord_kind: CoordKind::Geo,
        opacity: 0.7,
        labels,
        tooltip_values,
        feature_ids,
    }
}

/// Creates a bubble map plus a label overlay in one call.
///
/// Returns `[Mark::Xy(bubbles), Mark::Text(labels)]` — the bubbles
/// projected through the chart's geo plane and sized by value, the
/// labels projected through the same plane and offset 16 px above each
/// marker. Points without a `label` contribute a bubble but no label.
///
/// `name` is the bubble series' display name, used by the chart's
/// tooltip and legend. Pass an empty string to suppress.
///
/// Compose the result via `IntoData for Vec<Mark>` (already in scope):
///
/// ```
/// use hyozu::{bubble_map_with_labels, map_point, Mark, Choropleth, choropleth};
///
/// let mut marks = vec![Mark::Choropleth(choropleth([("CA", 1.0)]))];
/// marks.extend(bubble_map_with_labels(
///     [map_point(34.0, -118.2, 1.0).label("LA")],
///     "Stores",
/// ));
/// let data = hyozu::data(marks);
/// ```
pub fn bubble_map_with_labels(points: impl IntoIterator<Item = MapPoint>, name: impl Into<String>) -> Vec<Mark> {
    /// Vertical gap between a bubble's top edge and its label baseline,
    /// in pixels. Picked so the choropleth's hover ring (`radius + 2`)
    /// can't overprint the label when a bubble is hovered.
    const LABEL_PADDING: f32 = 8.0;

    let entries: Vec<MapPoint> = points.into_iter().collect();
    let value_range = compute_value_range(&entries);
    // Pull labels off the entries before they move into `bubble_map`. The
    // label set is sparse — only entries with `label.is_some()` produce a
    // `TextItem` — so the layer renders nothing for unlabeled bubbles.
    // Each labeled item gets a per-item offset sized to its bubble's
    // rendered radius, so the label sits just above the bubble's top
    // edge regardless of magnitude — uniform mark-level offsets put
    // small-bubble labels too high and big-bubble labels inside the disc.
    let label_items: Vec<TextItem> = entries
        .iter()
        .filter_map(|p| {
            p.label.as_ref().map(|l| {
                let radius = radius_for(p.value, value_range);
                TextItem {
                    datum: Datum::new(p.lon, p.lat),
                    label: l.clone(),
                    offset: Some((0.0, -(radius + LABEL_PADDING))),
                }
            })
        })
        .collect();
    let name = name.into();
    let mut bubbles = bubble_map(entries);
    if !name.is_empty() {
        bubbles = bubbles.with_name(name);
    }
    let labels = text::text(label_items)
        .on_geo()
        .offset(0.0, -16.0)
        .align(text::TextAlign::Center)
        .size(11.0);
    vec![Mark::Xy(bubbles), Mark::Text(labels)]
}

/// Compute the value-magnitude range over a slice of map points.
/// Collapses degenerate, empty, and all-non-finite inputs to a `+1.0`
/// span so the size normalizer can't divide by zero or produce NaN.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bubble_map_carries_point_feature_ids_into_xy_mark() {
        let bubbles = bubble_map([map_point(42.0, -72.0, 10.0).id("REG-NE"), map_point(34.0, -118.0, 20.0)]);

        assert_eq!(bubbles.feature_id_at(0).map(|id| id.as_str()), Some("REG-NE"));
        assert!(bubbles.feature_id_at(1).is_none());
    }
}

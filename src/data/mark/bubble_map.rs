use std::sync::Arc;

use crate::color::Color;
use crate::geo::{GeoData, MapScope, ProjectionKind};

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
    /// Runtime-loaded geographic data for the base map.
    pub(crate) geo: Option<Arc<GeoData>>,
    /// Which region of the world to show.
    pub(crate) scope: MapScope,
    /// Whether to draw land polygons.
    pub(crate) show_basemap: bool,
    /// Projection algorithm.
    pub(crate) projection: ProjectionKind,
}

/// Creates a bubble map from a collection of map points.
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
pub fn bubble_map(points: impl IntoBubbleMap) -> BubbleMap {
    points.into_bubble_map()
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
            geo: None,
            scope: MapScope::World,
            show_basemap: true,
            projection: ProjectionKind::default(),
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
            geo: None,
            scope: MapScope::World,
            show_basemap: true,
            projection: ProjectionKind::default(),
        }
    }
}

impl BubbleMap {
    /// Sets the geographic data for the base map background.
    pub fn geo(mut self, geo: Arc<GeoData>) -> Self {
        self.geo = Some(geo);
        self
    }

    /// Sets the map scope (which region to show).
    pub fn scope(mut self, scope: MapScope) -> Self {
        self.scope = scope;
        self
    }

    /// Disables drawing the land polygon basemap.
    pub fn no_basemap(mut self) -> Self {
        self.show_basemap = false;
        self
    }

    /// Sets the projection kind (default: Mercator).
    pub fn projection(mut self, kind: ProjectionKind) -> Self {
        self.projection = kind;
        self
    }

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

impl<Message, Theme, Renderer> From<BubbleMap> for crate::Data<Message, Theme, Renderer>
where
    Message: 'static,
    Theme: 'static,
    Renderer: 'static,
{
    fn from(bm: BubbleMap) -> Self {
        <BubbleMap as crate::data::IntoData<Message, Theme, Renderer>>::into_data(bm)
    }
}

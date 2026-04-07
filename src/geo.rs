//! Geographic data model and (optional) GeoJSON parsing.
//!
//! Coordinates are stored as `(lon, lat)` matching the GeoJSON spec.
//! Users who need `(lat, lon)` order (common in mapping APIs) should
//! convert at the boundary.

use std::collections::HashMap;

// ── Core types ─────────────────────────────────────────────────────

/// Axis-aligned bounding box in geographic coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeoBounds {
    pub min_lon: f32,
    pub min_lat: f32,
    pub max_lon: f32,
    pub max_lat: f32,
}

impl GeoBounds {
    pub fn new(min_lon: f32, min_lat: f32, max_lon: f32, max_lat: f32) -> Self {
        Self {
            min_lon,
            min_lat,
            max_lon,
            max_lat,
        }
    }

    pub fn contains(&self, lon: f32, lat: f32) -> bool {
        lon >= self.min_lon && lon <= self.max_lon && lat >= self.min_lat && lat <= self.max_lat
    }

    pub fn extend(&mut self, lon: f32, lat: f32) {
        self.min_lon = self.min_lon.min(lon);
        self.min_lat = self.min_lat.min(lat);
        self.max_lon = self.max_lon.max(lon);
        self.max_lat = self.max_lat.max(lat);
    }

    pub fn intersects(&self, other: &GeoBounds) -> bool {
        self.min_lon <= other.max_lon
            && self.max_lon >= other.min_lon
            && self.min_lat <= other.max_lat
            && self.max_lat >= other.min_lat
    }
}

/// Predefined map scopes with associated bounding boxes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MapScope {
    World,
    NorthAmerica,
    UnitedStates,
    Europe,
    LatinAmerica,
    Asia,
    Africa,
    Custom(GeoBounds),
}

impl MapScope {
    pub fn bounds(self) -> GeoBounds {
        match self {
            Self::World => GeoBounds::new(-180.0, -60.0, 180.0, 85.0),
            Self::NorthAmerica => GeoBounds::new(-170.0, 7.0, -50.0, 84.0),
            Self::UnitedStates => GeoBounds::new(-125.0, 24.0, -66.0, 50.0),
            Self::Europe => GeoBounds::new(-25.0, 34.0, 45.0, 72.0),
            Self::LatinAmerica => GeoBounds::new(-120.0, -56.0, -34.0, 33.0),
            Self::Asia => GeoBounds::new(25.0, -15.0, 180.0, 55.0),
            Self::Africa => GeoBounds::new(-20.0, -36.0, 55.0, 38.0),
            Self::Custom(bounds) => bounds,
        }
    }
}

/// A single geographic feature with one or more polygon rings.
#[derive(Debug, Clone)]
pub struct GeoFeature {
    pub id: String,
    pub name: String,
    pub polygons: Vec<Vec<(f32, f32)>>,
    pub properties: HashMap<String, String>,
}

impl GeoFeature {
    /// Computes the bounding box of all polygon vertices.
    pub fn bounds(&self) -> Option<GeoBounds> {
        let mut iter = self.polygons.iter().flat_map(|ring| ring.iter().copied());
        let (lon, lat) = iter.next()?;
        let mut bounds = GeoBounds::new(lon, lat, lon, lat);
        for (lon, lat) in iter {
            bounds.extend(lon, lat);
        }
        Some(bounds)
    }
}

/// A collection of [`GeoFeature`]s with O(1) lookup by id.
#[derive(Debug, Clone)]
pub struct GeoData {
    pub features: Vec<GeoFeature>,
    index: HashMap<String, usize>,
}

// ── GeoData methods ────────────────────────────────────────────────

impl GeoData {
    /// Creates a new `GeoData` from a list of features, building the
    /// id-to-index lookup table.
    pub fn new(features: Vec<GeoFeature>) -> Self {
        let index = features.iter().enumerate().map(|(i, f)| (f.id.clone(), i)).collect();
        Self { features, index }
    }

    /// O(1) lookup by feature id.
    pub fn get(&self, id: &str) -> Option<&GeoFeature> {
        self.index.get(id).map(|&i| &self.features[i])
    }

    pub fn len(&self) -> usize {
        self.features.len()
    }

    pub fn is_empty(&self) -> bool {
        self.features.is_empty()
    }

    /// Exposes the id-to-index map for Phase 6 choropleth matching.
    pub fn id_index(&self) -> &HashMap<String, usize> {
        &self.index
    }

    /// Bounding box encompassing all features.
    pub fn bounds(&self) -> Option<GeoBounds> {
        let mut iter = self.features.iter().filter_map(|f| f.bounds());
        let first = iter.next()?;
        let mut bounds = first;
        for fb in iter {
            bounds.extend(fb.min_lon, fb.min_lat);
            bounds.extend(fb.max_lon, fb.max_lat);
        }
        Some(bounds)
    }

    /// Returns a new `GeoData` containing only features whose bounding
    /// box intersects the given scope.
    pub fn filter_by_scope(&self, scope: MapScope) -> GeoData {
        let scope_bounds = scope.bounds();
        let filtered: Vec<GeoFeature> = self
            .features
            .iter()
            .filter(|f| f.bounds().map(|b| b.intersects(&scope_bounds)).unwrap_or(false))
            .cloned()
            .collect();
        GeoData::new(filtered)
    }

    /// Returns a new `GeoData` containing only features where the given
    /// property key matches the given value.
    pub fn filter_by_property(&self, key: &str, value: &str) -> GeoData {
        let filtered: Vec<GeoFeature> = self
            .features
            .iter()
            .filter(|f| f.properties.get(key).map(|v| v == value).unwrap_or(false))
            .cloned()
            .collect();
        GeoData::new(filtered)
    }
}

// ── Projection ────────────────────────────────────────────────────

/// Projection algorithm for mapping geographic coordinates to 2D.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ProjectionKind {
    /// Conformal projection (preserves local shape). Default.
    /// `x = lon`, `y = ln(tan(π/4 + lat/2))`.
    #[default]
    Mercator,
    /// Simple linear projection. `x = lon`, `y = lat`.
    /// Distorts shapes at high latitudes.
    Equirectangular,
    /// Lambert cylindrical equal-area. `x = lon`, `y = sin(lat)`.
    /// Better for choropleth where area perception matters.
    EqualArea,
    /// Natural Earth projection. Pseudo-cylindrical, designed for
    /// pleasing world maps. The standard for atlases and news media.
    NaturalEarth,
}

/// Projects geographic coordinates `(lon, lat)` to pixel coordinates `(x, y)`.
///
/// # Coordinate convention
///
/// Input is always `(lon, lat)` — longitude first, matching GeoJSON spec.
/// Output is `(x, y)` pixel coordinates within the fitted viewport.
///
/// # Usage
///
/// ```ignore
/// let proj = Projection::mercator().fit_size(800.0, 600.0, bounds);
/// let (px, py) = proj.project(-74.0, 40.7); // NYC
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Projection {
    kind: ProjectionKind,
    scale: f32,
    translate_x: f32,
    translate_y: f32,
}

impl Projection {
    /// Creates a new projection with the given kind.
    /// Call `fit_size` to configure scale and translation for a viewport.
    pub fn new(kind: ProjectionKind) -> Self {
        Self {
            kind,
            scale: 1.0,
            translate_x: 0.0,
            translate_y: 0.0,
        }
    }

    pub fn mercator() -> Self {
        Self::new(ProjectionKind::Mercator)
    }

    pub fn equirectangular() -> Self {
        Self::new(ProjectionKind::Equirectangular)
    }

    pub fn equal_area() -> Self {
        Self::new(ProjectionKind::EqualArea)
    }

    pub fn natural_earth() -> Self {
        Self::new(ProjectionKind::NaturalEarth)
    }

    /// Projects a geographic coordinate to pixel coordinates.
    ///
    /// Input: `(lon, lat)` in degrees — longitude first, latitude second.
    /// Output: `(x, y)` in pixels within the fitted viewport.
    pub fn project(&self, lon: f32, lat: f32) -> (f32, f32) {
        let (raw_x, raw_y) = self.project_raw(lon, lat);
        let x = raw_x * self.scale + self.translate_x;
        let y = raw_y * self.scale + self.translate_y;
        (x, y)
    }

    /// Raw (unscaled, untranslated) projection.
    fn project_raw(&self, lon: f32, lat: f32) -> (f32, f32) {
        let lat_rad = lat.to_radians();
        match self.kind {
            ProjectionKind::Mercator => {
                let clamped = lat_rad.clamp(-85.0_f32.to_radians(), 85.0_f32.to_radians());
                let x = lon.to_radians();
                let y = -((std::f32::consts::FRAC_PI_4 + clamped / 2.0).tan().ln());
                (x, y)
            }
            ProjectionKind::Equirectangular => {
                let x = lon.to_radians();
                let y = -lat_rad;
                (x, y)
            }
            ProjectionKind::EqualArea => {
                let x = lon.to_radians();
                let y = -lat_rad.sin();
                (x, y)
            }
            ProjectionKind::NaturalEarth => {
                // Polynomial coefficients from Tom Patterson (same as D3).
                let phi = lat_rad;
                let phi2 = phi * phi;

                let x = lon.to_radians()
                    * (0.8707 + phi2 * (-0.131979 + phi2 * (-0.013791 + phi2 * (0.003971 + phi2 * -0.001529))));
                let y =
                    -phi * (1.007226 + phi2 * (0.015085 + phi2 * (-0.044475 + phi2 * (0.028874 + phi2 * -0.005916))));
                (x, y)
            }
        }
    }

    /// Computes scale and translation so that `bounds` fills the given
    /// viewport `(width, height)` with uniform scaling and centering.
    ///
    /// Analogous to D3's `projection.fitSize([width, height], object)`.
    /// Preserves aspect ratio; blank bands appear on the shorter axis.
    pub fn fit_size(mut self, width: f32, height: f32, bounds: GeoBounds) -> Self {
        // Project all four corners (non-linear projections produce different
        // extents at different latitudes).
        let (x0, y0) = self.project_raw(bounds.min_lon, bounds.max_lat);
        let (x1, y1) = self.project_raw(bounds.max_lon, bounds.max_lat);
        let (x2, y2) = self.project_raw(bounds.min_lon, bounds.min_lat);
        let (x3, y3) = self.project_raw(bounds.max_lon, bounds.min_lat);

        let px_min = x0.min(x1).min(x2).min(x3);
        let px_max = x0.max(x1).max(x2).max(x3);
        let py_min = y0.min(y1).min(y2).min(y3);
        let py_max = y0.max(y1).max(y2).max(y3);

        let proj_width = px_max - px_min;
        let proj_height = py_max - py_min;

        if proj_width <= 0.0 || proj_height <= 0.0 {
            self.scale = 1.0;
            self.translate_x = width / 2.0;
            self.translate_y = height / 2.0;
            return self;
        }

        // Uniform scale: fit the larger dimension, blank bands on the other.
        let scale_x = width / proj_width;
        let scale_y = height / proj_height;
        self.scale = scale_x.min(scale_y);

        // Center the projected bounds in the viewport.
        let scaled_width = proj_width * self.scale;
        let scaled_height = proj_height * self.scale;
        self.translate_x = (width - scaled_width) / 2.0 - px_min * self.scale;
        self.translate_y = (height - scaled_height) / 2.0 - py_min * self.scale;

        self
    }
}

// ── GeoJSON parsing (behind feature flag) ──────────────────────────

#[cfg(feature = "geojson")]
mod geojson_parser {
    use super::*;
    use std::fmt;

    /// Errors that can occur during GeoJSON parsing.
    #[derive(Debug)]
    pub enum GeoError {
        InvalidJson(String),
        UnsupportedGeometry(String),
        MissingField(String),
    }

    impl fmt::Display for GeoError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                GeoError::InvalidJson(msg) => write!(f, "invalid JSON: {msg}"),
                GeoError::UnsupportedGeometry(msg) => {
                    write!(f, "unsupported geometry: {msg}")
                }
                GeoError::MissingField(msg) => write!(f, "missing field: {msg}"),
            }
        }
    }

    impl std::error::Error for GeoError {}

    /// Parses a GeoJSON string into a [`GeoData`] collection.
    ///
    /// Supports `Polygon` and `MultiPolygon` geometry types. Other types
    /// are silently skipped. Only outer rings are retained (holes are
    /// discarded). Rings that cross the antimeridian are automatically
    /// split into eastern and western halves.
    pub fn parse_geojson(input: &str) -> Result<GeoData, GeoError> {
        let root: serde_json::Value = serde_json::from_str(input).map_err(|e| GeoError::InvalidJson(e.to_string()))?;

        let features_arr = root
            .get("features")
            .and_then(|v| v.as_array())
            .ok_or_else(|| GeoError::MissingField("features".into()))?;

        let mut features = Vec::with_capacity(features_arr.len());

        for feat_val in features_arr {
            // Skip features with null geometry.
            let geom = match feat_val.get("geometry") {
                Some(g) if !g.is_null() => g,
                _ => continue,
            };

            let geom_type = geom.get("type").and_then(|t| t.as_str()).unwrap_or("");

            let coords = match geom.get("coordinates") {
                Some(c) => c,
                None => continue,
            };

            let polygons = match geom_type {
                "Polygon" => parse_ring_array(coords),
                "MultiPolygon" => {
                    let mut all = Vec::new();
                    if let Some(polys) = coords.as_array() {
                        for poly in polys {
                            all.extend(parse_ring_array(poly));
                        }
                    }
                    all
                }
                _ => continue, // silently skip other geometry types
            };

            if polygons.is_empty() {
                continue;
            }

            // Extract properties.
            let props_val = feat_val.get("properties");
            let mut properties = HashMap::new();

            let prop_keys = ["NAME", "ISO_A3", "CONTINENT", "ECONOMY", "postal"];
            for key in &prop_keys {
                if let Some(val) = props_val.and_then(|p| p.get(*key)).and_then(|v| v.as_str()) {
                    properties.insert((*key).to_string(), val.to_string());
                }
            }

            // ID precedence: ISO_A3 > postal > NAME
            let id = properties
                .get("ISO_A3")
                .filter(|v| !v.is_empty() && *v != "-99")
                .or_else(|| properties.get("postal").filter(|v| !v.is_empty()))
                .or_else(|| properties.get("NAME").filter(|v| !v.is_empty()))
                .cloned()
                .unwrap_or_default();

            let name = properties.get("NAME").cloned().unwrap_or_default();

            features.push(GeoFeature {
                id,
                name,
                polygons,
                properties,
            });
        }

        Ok(GeoData::new(features))
    }

    /// Parses a JSON coordinate array into polygon rings.
    ///
    /// Only the outer ring (index 0) is taken from each polygon; holes
    /// are discarded. Each ring is passed through antimeridian splitting,
    /// and rings with fewer than 3 points are filtered out.
    fn parse_ring_array(polygon_coords: &serde_json::Value) -> Vec<Vec<(f32, f32)>> {
        let rings = match polygon_coords.as_array() {
            Some(r) => r,
            None => return Vec::new(),
        };

        // Only take the outer ring (index 0).
        let outer = match rings.first().and_then(|r| r.as_array()) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let ring: Vec<(f32, f32)> = outer
            .iter()
            .filter_map(|pt| {
                let arr = pt.as_array()?;
                let lon = arr.first()?.as_f64()? as f32;
                let lat = arr.get(1)?.as_f64()? as f32;
                Some((lon, lat))
            })
            .collect();

        let split = split_antimeridian(ring);
        split.into_iter().filter(|r| r.len() >= 3).collect()
    }

    /// Splits a polygon ring that crosses the antimeridian (±180 longitude)
    /// into separate eastern and western halves.
    ///
    /// If no crossing is detected the original ring is returned as-is.
    fn split_antimeridian(ring: Vec<(f32, f32)>) -> Vec<Vec<(f32, f32)>> {
        if ring.len() < 2 {
            return vec![ring];
        }

        // Detect whether any crossing occurs.
        let has_crossing = ring.windows(2).any(|w| (w[1].0 - w[0].0).abs() > 180.0);

        if !has_crossing {
            return vec![ring];
        }

        let mut east: Vec<(f32, f32)> = Vec::new();
        let mut west: Vec<(f32, f32)> = Vec::new();

        for i in 0..ring.len() {
            let (lon, lat) = ring[i];

            // Check for crossing from previous point.
            if i > 0 {
                let (prev_lon, prev_lat) = ring[i - 1];
                let delta = lon - prev_lon;

                if delta.abs() > 180.0 {
                    // Interpolate latitude at the ±180 boundary.
                    let (effective_lon, effective_prev_lon) = if delta > 0.0 {
                        // Crossed from east to west (prev was positive, current
                        // jumped to negative via wrapping).
                        (lon - 360.0, prev_lon)
                    } else {
                        // Crossed from west to east.
                        (lon + 360.0, prev_lon)
                    };

                    let t = (180.0_f32 - effective_prev_lon.abs()) / (effective_lon - effective_prev_lon).abs();
                    let interp_lat = prev_lat + t * (lat - prev_lat);

                    // Add boundary points to both halves.
                    east.push((180.0, interp_lat));
                    west.push((-180.0, interp_lat));
                }
            }

            if lon >= 0.0 {
                east.push((lon, lat));
            } else {
                west.push((lon, lat));
            }
        }

        let mut result = Vec::new();
        if east.len() >= 3 {
            result.push(east);
        }
        if west.len() >= 3 {
            result.push(west);
        }
        result
    }
}

#[cfg(feature = "geojson")]
pub use geojson_parser::{GeoError, parse_geojson};

// ── Tests ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geo_bounds_contains() {
        let b = GeoBounds::new(-10.0, -10.0, 10.0, 10.0);
        assert!(b.contains(0.0, 0.0));
        assert!(b.contains(-10.0, -10.0));
        assert!(b.contains(10.0, 10.0));
        assert!(!b.contains(11.0, 0.0));
        assert!(!b.contains(0.0, -11.0));
    }

    #[test]
    fn geo_bounds_intersects() {
        let a = GeoBounds::new(0.0, 0.0, 10.0, 10.0);
        let b = GeoBounds::new(5.0, 5.0, 15.0, 15.0);
        let c = GeoBounds::new(20.0, 20.0, 30.0, 30.0);

        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
        assert!(!a.intersects(&c));
        assert!(!c.intersects(&a));
    }

    #[test]
    fn geo_data_new_builds_index() {
        let features = vec![
            GeoFeature {
                id: "USA".into(),
                name: "United States".into(),
                polygons: vec![vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]],
                properties: HashMap::new(),
            },
            GeoFeature {
                id: "CAN".into(),
                name: "Canada".into(),
                polygons: vec![vec![(2.0, 2.0), (3.0, 2.0), (3.0, 3.0)]],
                properties: HashMap::new(),
            },
        ];

        let data = GeoData::new(features);
        assert_eq!(data.len(), 2);
        assert!(!data.is_empty());
        assert!(data.get("USA").is_some());
        assert_eq!(data.get("USA").unwrap().name, "United States");
        assert!(data.get("CAN").is_some());
        assert!(data.get("BRA").is_none());
    }

    #[test]
    fn geo_data_bounds() {
        let features = vec![
            GeoFeature {
                id: "A".into(),
                name: "A".into(),
                polygons: vec![vec![(-5.0, -5.0), (5.0, 5.0), (0.0, 0.0)]],
                properties: HashMap::new(),
            },
            GeoFeature {
                id: "B".into(),
                name: "B".into(),
                polygons: vec![vec![(10.0, 10.0), (20.0, 20.0), (15.0, 15.0)]],
                properties: HashMap::new(),
            },
        ];

        let data = GeoData::new(features);
        let bounds = data.bounds().unwrap();
        assert_eq!(bounds.min_lon, -5.0);
        assert_eq!(bounds.min_lat, -5.0);
        assert_eq!(bounds.max_lon, 20.0);
        assert_eq!(bounds.max_lat, 20.0);
    }

    #[test]
    fn filter_by_property() {
        let mut props_af = HashMap::new();
        props_af.insert("CONTINENT".into(), "Africa".into());
        let mut props_eu = HashMap::new();
        props_eu.insert("CONTINENT".into(), "Europe".into());

        let features = vec![
            GeoFeature {
                id: "NGA".into(),
                name: "Nigeria".into(),
                polygons: vec![vec![(3.0, 6.0), (4.0, 6.0), (4.0, 7.0)]],
                properties: props_af.clone(),
            },
            GeoFeature {
                id: "ZAF".into(),
                name: "South Africa".into(),
                polygons: vec![vec![(28.0, -30.0), (29.0, -30.0), (29.0, -29.0)]],
                properties: props_af,
            },
            GeoFeature {
                id: "FRA".into(),
                name: "France".into(),
                polygons: vec![vec![(2.0, 48.0), (3.0, 48.0), (3.0, 49.0)]],
                properties: props_eu,
            },
        ];

        let data = GeoData::new(features);
        let africa = data.filter_by_property("CONTINENT", "Africa");
        assert_eq!(africa.len(), 2);
        assert!(africa.get("NGA").is_some());
        assert!(africa.get("ZAF").is_some());
    }

    #[test]
    fn map_scope_bounds_world() {
        let bounds = MapScope::World.bounds();
        assert_eq!(bounds.min_lon, -180.0);
        assert_eq!(bounds.min_lat, -60.0);
        assert_eq!(bounds.max_lon, 180.0);
        assert_eq!(bounds.max_lat, 85.0);
    }

    // ── Projection tests ────────────────────────────────────────────

    #[test]
    fn default_projection_kind_is_mercator() {
        assert_eq!(ProjectionKind::default(), ProjectionKind::Mercator);
    }

    #[test]
    fn mercator_origin_at_center() {
        let bounds = GeoBounds::new(-180.0, -85.0, 180.0, 85.0);
        let proj = Projection::mercator().fit_size(800.0, 600.0, bounds);
        let (x, y) = proj.project(0.0, 0.0);
        assert!((x - 400.0).abs() < 1.0, "x={x}, expected ~400");
        assert!((y - 300.0).abs() < 1.0, "y={y}, expected ~300");
    }

    #[test]
    fn equirect_origin_at_center() {
        let bounds = GeoBounds::new(-180.0, -90.0, 180.0, 90.0);
        let proj = Projection::equirectangular().fit_size(800.0, 600.0, bounds);
        let (x, y) = proj.project(0.0, 0.0);
        assert!((x - 400.0).abs() < 1.0, "x={x}, expected ~400");
        assert!((y - 300.0).abs() < 1.0, "y={y}, expected ~300");
    }

    #[test]
    fn equal_area_origin_at_center() {
        let bounds = GeoBounds::new(-180.0, -90.0, 180.0, 90.0);
        let proj = Projection::equal_area().fit_size(800.0, 600.0, bounds);
        let (x, y) = proj.project(0.0, 0.0);
        assert!((x - 400.0).abs() < 1.0, "x={x}, expected ~400");
        assert!((y - 300.0).abs() < 1.0, "y={y}, expected ~300");
    }

    #[test]
    fn us_bounds_nyc_quadrant() {
        let us = MapScope::UnitedStates.bounds();
        let proj = Projection::mercator().fit_size(800.0, 600.0, us);
        let (x, y) = proj.project(-74.0, 40.7);
        // NYC is east (right half) and north (upper half) of US center
        assert!(x > 400.0, "NYC x={x} should be >400");
        assert!(y < 300.0, "NYC y={y} should be <300");
        assert!(x > 0.0 && x < 800.0, "NYC x={x} out of viewport");
        assert!(y > 0.0 && y < 600.0, "NYC y={y} out of viewport");
    }

    #[test]
    fn aspect_ratio_preserved() {
        let bounds = MapScope::UnitedStates.bounds();
        let proj_wide = Projection::mercator().fit_size(1600.0, 600.0, bounds);
        let proj_square = Projection::mercator().fit_size(800.0, 800.0, bounds);

        let ratio = |proj: &Projection| -> f32 {
            let (x0, y0) = proj.project(bounds.min_lon, bounds.max_lat);
            let (x1, y1) = proj.project(bounds.max_lon, bounds.min_lat);
            (x1 - x0).abs() / (y1 - y0).abs()
        };

        let r_wide = ratio(&proj_wide);
        let r_square = ratio(&proj_square);
        assert!(
            (r_wide - r_square).abs() < 0.01,
            "ratios differ: wide={r_wide}, square={r_square}"
        );
    }

    #[test]
    fn mercator_clamps_poles() {
        let proj = Projection::mercator().fit_size(800.0, 600.0, GeoBounds::new(-180.0, -90.0, 180.0, 90.0));
        let (x_n, y_n) = proj.project(0.0, 90.0);
        let (x_s, y_s) = proj.project(0.0, -90.0);
        assert!(x_n.is_finite() && y_n.is_finite());
        assert!(x_s.is_finite() && y_s.is_finite());
    }

    #[test]
    fn fit_size_fills_viewport() {
        let bounds = MapScope::UnitedStates.bounds();
        let proj = Projection::mercator().fit_size(800.0, 600.0, bounds);

        let (x_tl, y_tl) = proj.project(bounds.min_lon, bounds.max_lat);
        let (x_tr, y_tr) = proj.project(bounds.max_lon, bounds.max_lat);
        let (x_bl, y_bl) = proj.project(bounds.min_lon, bounds.min_lat);
        let (x_br, y_br) = proj.project(bounds.max_lon, bounds.min_lat);

        let proj_w = x_tr.max(x_br) - x_tl.min(x_bl);
        let proj_h = y_bl.max(y_br) - y_tl.min(y_tr);

        // One dimension fills exactly, the other is smaller.
        let fills_width = (proj_w - 800.0).abs() < 1.0;
        let fills_height = (proj_h - 600.0).abs() < 1.0;
        assert!(
            fills_width || fills_height,
            "neither fills viewport: w={proj_w}, h={proj_h}"
        );
        assert!(proj_w <= 800.5, "width {proj_w} exceeds viewport");
        assert!(proj_h <= 600.5, "height {proj_h} exceeds viewport");
    }

    #[test]
    fn scope_bounds_are_valid() {
        let scopes = [
            MapScope::World,
            MapScope::NorthAmerica,
            MapScope::UnitedStates,
            MapScope::Europe,
            MapScope::LatinAmerica,
            MapScope::Asia,
            MapScope::Africa,
        ];
        for scope in scopes {
            let b = scope.bounds();
            assert!(b.min_lon < b.max_lon, "{scope:?}: min_lon >= max_lon");
            assert!(b.min_lat < b.max_lat, "{scope:?}: min_lat >= max_lat");
            assert!(b.min_lon >= -180.0 && b.max_lon <= 180.0);
            assert!(b.min_lat >= -90.0 && b.max_lat <= 90.0);
        }
    }

    #[test]
    fn custom_scope_bounds() {
        let custom = GeoBounds::new(10.0, 20.0, 30.0, 40.0);
        assert_eq!(MapScope::Custom(custom).bounds(), custom);
    }

    #[cfg(feature = "geojson")]
    mod geojson_tests {
        use super::super::*;

        #[test]
        fn parse_minimal_geojson() {
            let json = r#"{
                "type": "FeatureCollection",
                "features": [{
                    "type": "Feature",
                    "properties": {
                        "NAME": "Testland",
                        "ISO_A3": "TST"
                    },
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [
                            [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]]
                        ]
                    }
                }]
            }"#;

            let data = parse_geojson(json).unwrap();
            assert_eq!(data.len(), 1);
            let feat = data.get("TST").unwrap();
            assert_eq!(feat.name, "Testland");
            assert_eq!(feat.polygons.len(), 1);
            assert_eq!(feat.polygons[0].len(), 5);
        }

        #[test]
        fn parse_multipolygon() {
            let json = r#"{
                "type": "FeatureCollection",
                "features": [{
                    "type": "Feature",
                    "properties": {
                        "NAME": "Archipelago",
                        "ISO_A3": "ARC"
                    },
                    "geometry": {
                        "type": "MultiPolygon",
                        "coordinates": [
                            [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]],
                            [[[5.0, 5.0], [6.0, 5.0], [6.0, 6.0], [5.0, 5.0]]]
                        ]
                    }
                }]
            }"#;

            let data = parse_geojson(json).unwrap();
            assert_eq!(data.len(), 1);
            let feat = data.get("ARC").unwrap();
            assert_eq!(feat.polygons.len(), 2);
        }

        #[test]
        fn null_geometry_skipped() {
            let json = r#"{
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "properties": { "NAME": "Ghost" },
                        "geometry": null
                    },
                    {
                        "type": "Feature",
                        "properties": { "NAME": "Real", "ISO_A3": "REA" },
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [
                                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]
                            ]
                        }
                    }
                ]
            }"#;

            let data = parse_geojson(json).unwrap();
            assert_eq!(data.len(), 1);
            assert!(data.get("REA").is_some());
        }

        #[test]
        fn invalid_json_returns_error() {
            let result = parse_geojson("not json at all");
            assert!(result.is_err());
            match result.unwrap_err() {
                GeoError::InvalidJson(_) => {}
                other => panic!("expected InvalidJson, got: {other}"),
            }
        }

        #[test]
        fn id_falls_back_to_postal_then_name() {
            let json = r#"{
                "type": "FeatureCollection",
                "features": [
                    {
                        "type": "Feature",
                        "properties": { "NAME": "California", "postal": "CA" },
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [
                                [[-120.0, 35.0], [-115.0, 35.0], [-115.0, 40.0], [-120.0, 35.0]]
                            ]
                        }
                    },
                    {
                        "type": "Feature",
                        "properties": { "NAME": "NoCode" },
                        "geometry": {
                            "type": "Polygon",
                            "coordinates": [
                                [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]
                            ]
                        }
                    }
                ]
            }"#;

            let data = parse_geojson(json).unwrap();
            // postal takes precedence when ISO_A3 is absent
            assert!(data.get("CA").is_some());
            assert_eq!(data.get("CA").unwrap().name, "California");
            // NAME is last resort
            assert!(data.get("NoCode").is_some());
        }

        #[test]
        fn antimeridian_splitting() {
            // A ring that crosses the antimeridian.
            let json = r#"{
                "type": "FeatureCollection",
                "features": [{
                    "type": "Feature",
                    "properties": { "NAME": "Crosser", "ISO_A3": "CRS" },
                    "geometry": {
                        "type": "Polygon",
                        "coordinates": [
                            [[170.0, 50.0], [175.0, 55.0], [-175.0, 55.0], [-170.0, 50.0], [170.0, 50.0]]
                        ]
                    }
                }]
            }"#;

            let data = parse_geojson(json).unwrap();
            let feat = data.get("CRS").unwrap();
            // Should have been split into at least 2 polygon rings.
            assert!(
                feat.polygons.len() >= 2,
                "expected >= 2 polygons after antimeridian split, got {}",
                feat.polygons.len()
            );
        }

        #[test]
        fn parse_ne_110m_countries() {
            let path = "/tmp/ne_110m_countries.geojson";
            let Ok(contents) = std::fs::read_to_string(path) else {
                eprintln!("skipping integration test: {path} not found");
                return;
            };

            let data = parse_geojson(&contents).unwrap();

            // ~177 features in Natural Earth 110m countries
            assert!(data.len() >= 170, "expected >= 170 features, got {}", data.len());

            // USA should exist.
            assert!(data.get("USA").is_some(), "USA not found");

            // Russia should be split at the antimeridian.
            let russia = data.get("RUS").expect("Russia not found");
            assert!(
                russia.polygons.len() >= 2,
                "expected Russia to have >= 2 polygons (antimeridian split), got {}",
                russia.polygons.len()
            );

            // Africa filter should yield ~50+ countries.
            let africa = data.filter_by_property("CONTINENT", "Africa");
            assert!(
                africa.len() >= 50,
                "expected >= 50 African countries, got {}",
                africa.len()
            );

            // Europe scope filter should yield ~30+ countries.
            let europe = data.filter_by_scope(MapScope::Europe);
            assert!(
                europe.len() >= 30,
                "expected >= 30 countries in Europe scope, got {}",
                europe.len()
            );
        }

        #[test]
        fn parse_ne_110m_states() {
            let path = "/tmp/ne_110m_states.geojson";
            let Ok(contents) = std::fs::read_to_string(path) else {
                eprintln!("skipping integration test: {path} not found");
                return;
            };

            let data = parse_geojson(&contents).unwrap();

            // ~50+ features in US states
            assert!(data.len() >= 50, "expected >= 50 state features, got {}", data.len());
        }
    }
}

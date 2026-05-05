//! Chart-level geo projection plane: caches projected polygons for
//! all geo-aware marks. Constructed once per layout pass; reprojection
//! short-circuits on `Arc::ptr_eq` + `prev_size` + `prev_scope` +
//! `prev_projection`.
//!
//! Per-mark consumers (Choropleth, geo-Xy) read the cached polygons
//! and point projections via [`Plane::project_polygon`] and
//! [`Plane::project_point`].

use std::sync::Arc;

use crate::core::{Point, Rectangle};
use crate::feature;
use crate::geo::{GeoData, MapScope, Projection, ProjectionKind};

/// Caches the per-frame projection state for geo-aware marks. One
/// instance lives on [`super::State::geo_plane`] when a geo mark is
/// present; `ensure` reprojects only when an input has changed.
pub struct Plane {
    /// Local-pixel bounds the projection was fitted into.
    pub bounds: Rectangle,
    /// The active geo dataset, if any. `None` produces an empty plane.
    pub geo: Option<Arc<GeoData>>,
    /// The active scope (filters features and supplies the fit bounds).
    pub scope: MapScope,
    /// The active projection algorithm.
    pub projection: ProjectionKind,
    /// Filtered features projected to pixel-space rings.
    /// `[feature_idx][ring_idx][point_idx] -> (x, y)`.
    pub projected_polygons: Vec<Vec<Vec<(f32, f32)>>>,
    /// Feature ids aligned 1:1 with [`Self::projected_polygons`].
    pub filtered_ids: Vec<feature::Id>,
    /// Axis-aligned `(min_x, min_y, max_x, max_y)` bounding box per
    /// filtered feature, aligned 1:1 with [`Self::projected_polygons`].
    pub feature_bboxes: Vec<(f32, f32, f32, f32)>,
    /// Cached fitted projection used by [`Self::project_point`]. `None`
    /// when [`Self::geo`] is `None`.
    fitted: Option<Projection>,
    prev_size: (f32, f32),
    prev_geo: Option<Arc<GeoData>>,
    prev_scope: MapScope,
    prev_projection: ProjectionKind,
}

impl Plane {
    /// Empty plane: no geo, no projected polygons. The first
    /// [`Self::ensure`] call populates everything.
    pub fn new() -> Self {
        Self {
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            geo: None,
            scope: MapScope::World,
            projection: ProjectionKind::default(),
            projected_polygons: Vec::new(),
            filtered_ids: Vec::new(),
            feature_bboxes: Vec::new(),
            fitted: None,
            prev_size: (0.0, 0.0),
            prev_geo: None,
            prev_scope: MapScope::World,
            prev_projection: ProjectionKind::default(),
        }
    }

    /// Reproject when any of `(bounds.size, geo, scope, projection)`
    /// changed; otherwise short-circuit. Geo equality is `Arc::ptr_eq`
    /// — caller is responsible for bumping the `Arc` only when the
    /// payload actually changes.
    pub fn ensure(
        &mut self,
        bounds: Rectangle,
        geo: &Option<Arc<GeoData>>,
        scope: MapScope,
        projection: ProjectionKind,
    ) {
        let current_size = (bounds.width, bounds.height);
        let geo_same = match (&self.prev_geo, geo) {
            (Some(prev), Some(cur)) => Arc::ptr_eq(prev, cur),
            (None, None) => true,
            (Some(_), None) | (None, Some(_)) => false,
        };
        let clean = geo_same
            && self.prev_size == current_size
            && self.prev_scope == scope
            && self.prev_projection == projection;
        if clean {
            self.bounds = bounds;
            return;
        }

        self.prev_size = current_size;
        self.prev_geo = geo.clone();
        self.prev_scope = scope;
        self.prev_projection = projection;

        self.bounds = bounds;
        self.geo = geo.clone();
        self.scope = scope;
        self.projection = projection;

        match geo {
            None => {
                self.projected_polygons.clear();
                self.filtered_ids.clear();
                self.feature_bboxes.clear();
                self.fitted = None;
            }
            Some(data) => {
                let fitted = Projection::new(projection).fit_size(bounds.width, bounds.height, scope.bounds());
                let filtered = data.filter_by_scope(scope);

                self.projected_polygons = filtered
                    .features
                    .iter()
                    .map(|feature| {
                        feature
                            .polygons
                            .iter()
                            .map(|ring| ring.iter().map(|&(lon, lat)| fitted.project(lon, lat)).collect())
                            .collect()
                    })
                    .collect();

                self.filtered_ids = filtered.features.iter().map(|f| f.id.clone()).collect();

                self.feature_bboxes = self
                    .projected_polygons
                    .iter()
                    .map(|rings| {
                        let mut min_x = f32::INFINITY;
                        let mut min_y = f32::INFINITY;
                        let mut max_x = f32::NEG_INFINITY;
                        let mut max_y = f32::NEG_INFINITY;
                        for ring in rings {
                            for &(x, y) in ring {
                                min_x = min_x.min(x);
                                min_y = min_y.min(y);
                                max_x = max_x.max(x);
                                max_y = max_y.max(y);
                            }
                        }
                        (min_x, min_y, max_x, max_y)
                    })
                    .collect();

                self.fitted = Some(fitted);
            }
        }
    }

    /// Returns the projected rings for `feat_idx`, or `None` when the
    /// index is out of range.
    pub fn project_polygon(&self, feat_idx: usize) -> Option<&[Vec<(f32, f32)>]> {
        self.projected_polygons.get(feat_idx).map(|v| v.as_slice())
    }

    /// Projects a single `(lon, lat)` through the cached fitted
    /// projection. Returns plane-local `(0, 0)` when no geo is loaded
    /// — call sites that opt into geo coordinates are expected to
    /// gate on `state.geo_plane.is_some()` so the no-geo fallback
    /// only fires on a misconfiguration.
    pub fn project_point(&self, lon: f32, lat: f32) -> Point {
        match &self.fitted {
            Some(p) => {
                let (x, y) = p.project(lon, lat);
                Point::new(x, y)
            }
            None => Point::ORIGIN,
        }
    }
}

impl Default for Plane {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(width: f32, height: f32) -> Rectangle {
        Rectangle {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    #[test]
    fn new_starts_empty() {
        let plane = Plane::new();
        assert!(plane.geo.is_none());
        assert!(plane.projected_polygons.is_empty());
        assert!(plane.filtered_ids.is_empty());
        assert!(plane.feature_bboxes.is_empty());
        assert_eq!(plane.scope, MapScope::World);
        assert_eq!(plane.projection, ProjectionKind::default());
    }

    #[test]
    fn ensure_with_no_geo_leaves_polygons_empty() {
        let mut plane = Plane::new();
        plane.ensure(rect(800.0, 600.0), &None, MapScope::World, ProjectionKind::Mercator);
        assert!(plane.projected_polygons.is_empty());
        assert!(plane.filtered_ids.is_empty());
        assert_eq!(plane.bounds.width, 800.0);
        assert_eq!(plane.bounds.height, 600.0);
        assert_eq!(plane.prev_size, (800.0, 600.0));
        assert_eq!(plane.prev_scope, MapScope::World);
        assert_eq!(plane.prev_projection, ProjectionKind::Mercator);
    }

    #[test]
    fn ensure_short_circuits_on_unchanged_inputs() {
        // Stash a sentinel polygon after the first call; a true
        // short-circuit on the second call leaves it in place, while
        // a reproject (with `geo: None`) would clear the vector.
        let mut plane = Plane::new();
        plane.ensure(rect(800.0, 600.0), &None, MapScope::World, ProjectionKind::Mercator);
        plane.projected_polygons.push(vec![vec![(7.0, 7.0)]]);

        plane.ensure(rect(800.0, 600.0), &None, MapScope::World, ProjectionKind::Mercator);
        assert_eq!(plane.projected_polygons.len(), 1);
        assert_eq!(plane.projected_polygons[0][0][0], (7.0, 7.0));
    }

    #[test]
    fn ensure_repopulates_when_size_changes() {
        let mut plane = Plane::new();
        plane.ensure(rect(800.0, 600.0), &None, MapScope::World, ProjectionKind::Mercator);
        plane.ensure(rect(400.0, 300.0), &None, MapScope::World, ProjectionKind::Mercator);
        assert_eq!(plane.prev_size, (400.0, 300.0));
        assert_eq!(plane.bounds.width, 400.0);
        assert_eq!(plane.bounds.height, 300.0);
    }

    #[test]
    fn ensure_repopulates_when_projection_changes() {
        // Projection swap is part of the dirty key.
        let mut plane = Plane::new();
        plane.ensure(rect(800.0, 600.0), &None, MapScope::World, ProjectionKind::Mercator);
        plane.ensure(rect(800.0, 600.0), &None, MapScope::World, ProjectionKind::EqualArea);
        assert_eq!(plane.prev_projection, ProjectionKind::EqualArea);
        assert_eq!(plane.projection, ProjectionKind::EqualArea);
    }

    #[test]
    fn project_point_with_no_fit_returns_origin() {
        let plane = Plane::new();
        let p = plane.project_point(0.0, 0.0);
        assert_eq!(p, Point::ORIGIN);
    }

    #[test]
    fn project_polygon_out_of_range_returns_none() {
        let plane = Plane::new();
        assert!(plane.project_polygon(0).is_none());
    }
}

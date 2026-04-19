//! Per-feature centroid coordinates derived from `hyozu::GeoData`.
//!
//! Centroids are bbox centers, not area-weighted polygon centroids. Good
//! enough for bubble-marker placement; off by a handful of pixels on
//! irregularly-shaped features. Derived from `GeoData` at construction so
//! there's only one asset to load and keep in sync.

use std::collections::HashMap;
use std::sync::Arc;

use hyozu::GeoData;

/// State-code → (lat, lon) lookup. Keyed by the same string used as the
/// feature id (USPS 2-letter code for Natural Earth admin_1 US states).
#[derive(Debug, Clone)]
pub struct Centroids {
    inner: HashMap<String, (f64, f64)>,
}

impl Centroids {
    /// Derive per-feature centroids by taking each feature's bounding-box
    /// center. Features without bounds (empty polygons) are skipped.
    #[must_use]
    pub fn from_geo(geo: &GeoData) -> Self {
        let mut inner = HashMap::with_capacity(geo.features.len());
        for feature in &geo.features {
            let Some(bounds) = feature.bounds() else {
                continue;
            };
            let lon = f64::from((bounds.min_lon + bounds.max_lon) / 2.0);
            let lat = f64::from((bounds.min_lat + bounds.max_lat) / 2.0);
            inner.insert(feature.id.as_str().to_owned(), (lat, lon));
        }
        Self { inner }
    }

    /// Look up a state's centroid. Returns `None` for ids not in the map.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<(f64, f64)> {
        self.inner.get(id).copied()
    }

    /// Number of centroids indexed.
    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the map is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Convenience alias for the `Arc<Centroids>` flavor stored on the dashboard.
pub type SharedCentroids = Arc<Centroids>;

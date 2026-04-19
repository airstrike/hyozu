use std::sync::Arc;

use crate::geo::{GeoData, MapScope, ProjectionKind};

/// How values are mapped onto the [0, 1] color scale range.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Normalization {
    /// Linear: `t = (v - min) / (max - min)`. Good for uniform data.
    Linear,
    /// Square root: `t = sqrt((v - min) / (max - min))`. Expands the
    /// low end, good for moderately skewed data (GDP, population).
    #[default]
    Sqrt,
    /// Logarithmic: `t = log(v - min + 1) / log(max - min + 1)`. Best
    /// for data spanning orders of magnitude.
    Log,
}

/// A single entry: maps a feature ID to a value.
#[derive(Debug, Clone)]
pub struct ChoroplethEntry {
    pub(crate) id: crate::feature::Id,
    pub(crate) value: f64,
}

/// Creates a choropleth entry. `id` accepts anything convertible into an
/// [`Id`](crate::feature::Id) — `&str`, `String`, or an existing `Id`.
pub fn choropleth_entry(id: impl Into<crate::feature::Id>, value: impl Into<f64>) -> ChoroplethEntry {
    ChoroplethEntry {
        id: id.into(),
        value: value.into(),
    }
}

/// Choropleth chart specification.
///
/// Fills geographic features with colors mapped to data values.
/// Each entry maps a feature ID (matching a GeoJSON property) to a numeric value;
/// the value is then mapped onto a color scale defined by `color_stops`.
#[derive(Debug, Clone)]
pub struct Choropleth {
    pub(crate) entries: Vec<ChoroplethEntry>,
    pub(crate) geo: Option<Arc<GeoData>>,
    pub(crate) scope: MapScope,
    pub(crate) color_stops: Option<Vec<crate::core::Color>>,
    pub(crate) projection: ProjectionKind,
    pub(crate) normalization: Normalization,
    /// Optional title for the in-chart color legend.
    pub(crate) legend_title: Option<String>,
}

/// Creates a choropleth from entries.
pub fn choropleth(entries: impl IntoChoropleth) -> Choropleth {
    entries.into_choropleth()
}

/// Trait for converting various inputs into a Choropleth.
pub trait IntoChoropleth {
    fn into_choropleth(self) -> Choropleth;
}

// ── From [ChoroplethEntry; N] ────────────────────────────────────

impl<const N: usize> IntoChoropleth for [ChoroplethEntry; N] {
    fn into_choropleth(self) -> Choropleth {
        Choropleth {
            entries: self.into(),
            geo: None,
            scope: MapScope::World,
            color_stops: None,
            projection: ProjectionKind::default(),
            normalization: Normalization::default(),
            legend_title: None,
        }
    }
}

// ── From Vec<ChoroplethEntry> ────────────────────────────────────

impl IntoChoropleth for Vec<ChoroplethEntry> {
    fn into_choropleth(self) -> Choropleth {
        Choropleth {
            entries: self,
            geo: None,
            scope: MapScope::World,
            color_stops: None,
            projection: ProjectionKind::default(),
            normalization: Normalization::default(),
            legend_title: None,
        }
    }
}

// ── From [(S, V); N] where S: Into<String>, V: Into<f64> ────────

impl<S, V, const N: usize> IntoChoropleth for [(S, V); N]
where
    S: Into<crate::feature::Id>,
    V: Into<f64>,
{
    fn into_choropleth(self) -> Choropleth {
        Choropleth {
            entries: self
                .into_iter()
                .map(|(id, value)| ChoroplethEntry {
                    id: id.into(),
                    value: value.into(),
                })
                .collect(),
            geo: None,
            scope: MapScope::World,
            color_stops: None,
            projection: ProjectionKind::default(),
            normalization: Normalization::default(),
            legend_title: None,
        }
    }
}

// ── From Vec<(S, V)> where S: Into<Id>, V: Into<f64> ────────

impl<S, V> IntoChoropleth for Vec<(S, V)>
where
    S: Into<crate::feature::Id>,
    V: Into<f64>,
{
    fn into_choropleth(self) -> Choropleth {
        Choropleth {
            entries: self
                .into_iter()
                .map(|(id, value)| ChoroplethEntry {
                    id: id.into(),
                    value: value.into(),
                })
                .collect(),
            geo: None,
            scope: MapScope::World,
            color_stops: None,
            projection: ProjectionKind::default(),
            normalization: Normalization::default(),
            legend_title: None,
        }
    }
}

// ── Builder methods ──────────────────────────────────────────────

impl Choropleth {
    /// Sets the geographic data used to resolve feature geometries.
    pub fn geo(mut self, geo: Arc<GeoData>) -> Self {
        self.geo = Some(geo);
        self
    }

    /// Sets the map scope (which region of the world to show).
    pub fn scope(mut self, scope: MapScope) -> Self {
        self.scope = scope;
        self
    }

    /// Sets a named or custom color scale.
    ///
    /// Use `-scale` to reverse: `.scale(-Scale::GreenRed)`.
    pub fn scale(mut self, scale: crate::scale::Scale) -> Self {
        self.color_stops = Some(scale.stops());
        self
    }

    /// Sets raw color scale stops.
    pub fn color_range(mut self, stops: impl Into<Vec<crate::core::Color>>) -> Self {
        self.color_stops = Some(stops.into());
        self
    }

    /// Sets the title shown on the in-chart color legend.
    pub fn legend_title(mut self, title: impl Into<String>) -> Self {
        self.legend_title = Some(title.into());
        self
    }

    /// Sets the value normalization (default: Sqrt).
    ///
    /// - `Linear`: uniform data
    /// - `Sqrt`: moderately skewed data (GDP, population) — **default**
    /// - `Log`: data spanning orders of magnitude
    pub fn normalization(mut self, norm: Normalization) -> Self {
        self.normalization = norm;
        self
    }

    /// Sets the projection kind (default: Mercator).
    pub fn projection(mut self, kind: ProjectionKind) -> Self {
        self.projection = kind;
        self
    }

    /// Returns the entries.
    pub fn entries(&self) -> &[ChoroplethEntry] {
        &self.entries
    }

    /// Choropleth maps have no x-axis.
    pub fn x_axis() -> Option<crate::data::Axis> {
        None
    }

    /// Choropleth maps have no y-axis.
    pub fn y_axis() -> Option<crate::data::Axis> {
        None
    }
}

// ── Into Data ────────────────────────────────────────────────────

impl From<Choropleth> for crate::Data {
    fn from(c: Choropleth) -> Self {
        use crate::data::IntoData;
        c.into_data()
    }
}

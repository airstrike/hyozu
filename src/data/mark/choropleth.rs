use std::collections::HashSet;
use std::sync::Arc;

use crate::data::legend;
use crate::feature;
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

/// A single entry: maps a feature ID to either a numeric value (gradient
/// fill) or an explicit "available, no value" marker (muted neutral fill,
/// signalling that the feature is part of the active context even though
/// no measurement is being shown).
///
/// Features without ANY entry render as `land_fill` — the implicit
/// "out of scope / no data at all" state.
#[derive(Debug, Clone)]
pub struct ChoroplethEntry {
    pub(crate) id: crate::feature::Id,
    pub(crate) value: Option<f64>,
}

/// Creates a value-bearing choropleth entry. `id` accepts anything
/// convertible into an [`Id`](crate::feature::Id) — `&str`, `String`, or
/// an existing `Id`.
pub fn choropleth_entry(id: impl Into<crate::feature::Id>, value: impl Into<f64>) -> ChoroplethEntry {
    ChoroplethEntry {
        id: id.into(),
        value: Some(value.into()),
    }
}

/// Creates an "available, no value" choropleth entry. Renders with the
/// muted neutral `available_fill` to signal that the feature is part of
/// the active context (e.g., has data on a sibling layer like bubbles)
/// even though there's no fill value to encode on the gradient.
pub fn choropleth_entry_available(id: impl Into<crate::feature::Id>) -> ChoroplethEntry {
    ChoroplethEntry {
        id: id.into(),
        value: None,
    }
}

impl ChoroplethEntry {
    /// Feature ID this entry maps to.
    pub fn id(&self) -> &crate::feature::Id {
        &self.id
    }

    /// Numeric value if present. `None` signals "available, no value" —
    /// see [`choropleth_entry_available`].
    pub fn value(&self) -> Option<f64> {
        self.value
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
    /// Optional title for the color-scale legend.
    pub(crate) legend_title: Option<String>,
    /// Color-scale legend configuration. `None` suppresses the legend
    /// entirely. The default is an overlaid horizontal bar pinned to the
    /// bottom-right corner.
    pub(crate) legend: Option<legend::Config>,
    /// Optional active-selection set. When `Some`, only IDs in the set
    /// render at full gradient saturation; other value-bearing features
    /// blend halfway toward `land_fill` to recede into the background
    /// while remaining clickable. When `None`, every value-bearing
    /// feature renders at full saturation (no selection mode).
    pub(crate) selected: Option<HashSet<feature::Id>>,
}

fn default_legend() -> Option<legend::Config> {
    Some(legend::Config::overlay(legend::Anchor::BottomRight).orientation(legend::Orientation::Horizontal))
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
            legend: default_legend(),
            selected: None,
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
            legend: default_legend(),
            selected: None,
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
                    value: Some(value.into()),
                })
                .collect(),
            geo: None,
            scope: MapScope::World,
            color_stops: None,
            projection: ProjectionKind::default(),
            normalization: Normalization::default(),
            legend_title: None,
            legend: default_legend(),
            selected: None,
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
                    value: Some(value.into()),
                })
                .collect(),
            geo: None,
            scope: MapScope::World,
            color_stops: None,
            projection: ProjectionKind::default(),
            normalization: Normalization::default(),
            legend_title: None,
            legend: default_legend(),
            selected: None,
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

    /// Sets a named or custom color scheme.
    ///
    /// Use `-scheme` to reverse: `.scheme(-palette::Scheme::GreenRed)`.
    pub fn scheme(mut self, scheme: crate::palette::Scheme) -> Self {
        self.color_stops = Some(scheme.stops());
        self
    }

    /// Sets raw color scale stops.
    pub fn color_range(mut self, stops: impl Into<Vec<crate::core::Color>>) -> Self {
        self.color_stops = Some(stops.into());
        self
    }

    /// Sets the title shown on the color-scale legend.
    pub fn legend_title(mut self, title: impl Into<String>) -> Self {
        self.legend_title = Some(title.into());
        self
    }

    /// Configures the color-scale legend.
    ///
    /// Pass a [`legend::Config`] to customize anchor / placement / orientation /
    /// text, or `None` to suppress the legend entirely. The default is an
    /// overlaid horizontal bar pinned to the bottom-right.
    pub fn legend(mut self, legend: impl Into<Option<legend::Config>>) -> Self {
        self.legend = legend.into();
        self
    }

    /// Returns the current color-scale legend configuration.
    pub fn legend_config(&self) -> Option<&legend::Config> {
        self.legend.as_ref()
    }

    /// Returns the current color-scale legend title, if any.
    pub fn legend_title_value(&self) -> Option<&str> {
        self.legend_title.as_deref()
    }

    /// Marks the given feature IDs as the active selection.
    ///
    /// Value-bearing features in the set render at full gradient
    /// saturation; value-bearing features outside the set blend halfway
    /// toward the muted `land_fill` so they recede visually while
    /// remaining clickable. Pass `None` (or skip the call) to disable
    /// selection mode entirely — every value-bearing feature renders
    /// at full saturation.
    pub fn selected<I, S>(mut self, ids: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<feature::Id>,
    {
        self.selected = Some(ids.into_iter().map(Into::into).collect());
        self
    }

    /// Returns the active selection set, if any.
    pub fn selected_ids(&self) -> Option<&HashSet<feature::Id>> {
        self.selected.as_ref()
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

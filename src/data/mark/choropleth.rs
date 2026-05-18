use std::collections::HashSet;
use std::sync::Arc;

use crate::data::legend;
use crate::feature;
use crate::palette::Palette;
use crate::scale::ColorScale;

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
/// Each entry maps a feature ID (matching a GeoJSON property) to a
/// numeric value; the value is then mapped onto a color scale described
/// by [`ColorScale<f64>`] (domain + palette + transform + format).
#[derive(Clone)]
pub struct Choropleth {
    pub(crate) entries: Vec<ChoroplethEntry>,
    /// Color encoding scale: domain (auto-inferred when `None`), palette
    /// (mark default when `None`), transform, and optional legend tick
    /// formatter. Default is [`ColorScale::default().sqrt()`] to preserve
    /// the historical "square-root normalization" choropleth behavior.
    pub(crate) color: ColorScale<f64>,
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
    /// Optional override for decorative land features with no entry in
    /// the data. When `None`, the renderer derives a muted neutral from
    /// the chart background.
    pub(crate) land_color: Option<crate::core::Color>,
    /// Optional override for entries that are in scope but carry no
    /// value. When `None`, the renderer derives a slightly stronger
    /// neutral from `land_color`.
    pub(crate) available_color: Option<crate::core::Color>,
    /// Optional override for the fill color used on in-scope features
    /// whose entry is absent from the data. When `None`, the renderer
    /// falls back to the theme's [`Design::missing_fill`](crate::Design::missing_fill).
    pub(crate) missing_color: Option<crate::core::Color>,
    /// Optional override for the "ocean" — the decorative background
    /// painted under the map outside any feature's polygon. When `None`,
    /// the renderer falls back to the theme's
    /// [`Design::ocean_fill`](crate::Design::ocean_fill).
    pub(crate) ocean_color: Option<crate::core::Color>,
    /// Optional hover-time appearance closure. `None` leaves the
    /// renderer to use [`HoverStyle::from_theme`]; otherwise the
    /// closure runs at draw time with the resolved [`Design`] and
    /// [`palette::Resolved`] in scope, mirroring iced's
    /// `style: impl Fn(&Theme, Status) -> Style` widget pattern.
    ///
    /// [`Design`]: crate::design::Design
    /// [`palette::Resolved`]: crate::palette::Resolved
    pub(crate) hover_style: Option<HoverStyleFn>,
}

/// Theme-aware hover style closure. Receives the chart's
/// [`Design`](crate::design::Design) (for theme-derived colors)
/// and the resolved [`palette`](crate::palette::Resolved) (for
/// indexed gradient stops on choropleth, palette colors on other
/// marks). Returns the [`HoverStyle`] used for the next hover
/// frame.
pub type HoverStyleFn = Arc<dyn Fn(&dyn crate::design::Design, &crate::palette::Resolved) -> HoverStyle + Send + Sync>;

/// Hover-time appearance for a [`Choropleth`].
///
/// Mirrors iced's struct-shaped style pattern (`container::Style`
/// etc.). Build via [`HoverStyle::from_theme`] for the
/// theme-derived defaults and override fields with struct-update
/// syntax — the closure form receives [`Design`] and
/// [`palette::Resolved`] so theme changes propagate automatically.
///
/// ```ignore
/// use hyozu::Color;
/// use hyozu::mark::choropleth::HoverStyle;
///
/// // Default look — no hover_style call needed.
/// hyozu::choropleth(entries)
///
/// // Theme-derived, override one field:
/// hyozu::choropleth(entries)
///     .hover_style(|design, _palette| HoverStyle {
///         outline_width: 2.0,
///         ..HoverStyle::from_theme(design)
///     })
///
/// // Fully fixed style:
/// hyozu::choropleth(entries)
///     .hover_style(|_, _| HoverStyle {
///         outline_color: Color::WHITE,
///         outline_width: 2.5,
///     })
/// ```
///
/// [`Design`]: crate::design::Design
/// [`palette::Resolved`]: crate::palette::Resolved
#[derive(Debug, Clone, Copy)]
pub struct HoverStyle {
    /// Stroke color drawn around the hovered feature's projected
    /// polygons. Already-resolved RGBA — typically pulled from the
    /// design via [`HoverStyle::from_theme`] or set fixed
    /// (`Color::WHITE` etc.).
    pub outline_color: crate::core::Color,
    /// Stroke width in pixels.
    pub outline_width: f32,
}

impl std::fmt::Debug for Choropleth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Choropleth")
            .field("entries", &self.entries)
            .field("color", &self.color)
            .field("legend_title", &self.legend_title)
            .field("legend", &self.legend)
            .field("selected", &self.selected)
            .field("land_color", &self.land_color)
            .field("available_color", &self.available_color)
            .field("missing_color", &self.missing_color)
            .field("ocean_color", &self.ocean_color)
            .field("hover_style", &self.hover_style.as_ref().map(|_| "<closure>"))
            .finish()
    }
}

impl HoverStyle {
    /// Returns the theme-derived defaults: theme text color at
    /// 0.85 alpha, 1.25px stroke. Use as the rest of a
    /// struct-update expression to override one or two fields:
    ///
    /// ```ignore
    /// HoverStyle {
    ///     outline_width: 2.0,
    ///     ..HoverStyle::from_theme(design)
    /// }
    /// ```
    pub fn from_theme(design: &dyn crate::design::Design) -> Self {
        let bg = design.background_color();
        let text = design
            .text_color()
            .resolve(bg, design.text_pair(), &design.seed(), None);
        Self {
            outline_color: crate::core::Color { a: 0.85, ..text },
            outline_width: 1.25,
        }
    }
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
            color: ColorScale::default().sqrt(),
            legend_title: None,
            legend: default_legend(),
            selected: None,
            land_color: None,
            available_color: None,
            missing_color: None,
            ocean_color: None,
            hover_style: None,
        }
    }
}

// ── From Vec<ChoroplethEntry> ────────────────────────────────────

impl IntoChoropleth for Vec<ChoroplethEntry> {
    fn into_choropleth(self) -> Choropleth {
        Choropleth {
            entries: self,
            color: ColorScale::default().sqrt(),
            legend_title: None,
            legend: default_legend(),
            selected: None,
            land_color: None,
            available_color: None,
            missing_color: None,
            ocean_color: None,
            hover_style: None,
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
            color: ColorScale::default().sqrt(),
            legend_title: None,
            legend: default_legend(),
            selected: None,
            land_color: None,
            available_color: None,
            missing_color: None,
            ocean_color: None,
            hover_style: None,
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
            color: ColorScale::default().sqrt(),
            legend_title: None,
            legend: default_legend(),
            selected: None,
            land_color: None,
            available_color: None,
            missing_color: None,
            ocean_color: None,
            hover_style: None,
        }
    }
}

// ── Builder methods ──────────────────────────────────────────────

impl Choropleth {
    /// Sets a named or custom color scheme.
    ///
    /// Use `-scheme` to reverse: `.scheme(-palette::Scheme::GreenRed)`.
    pub fn scheme(mut self, scheme: crate::palette::Scheme) -> Self {
        let stops: Vec<crate::color::Color> = scheme.stops().into_iter().map(Into::into).collect();
        self.color.palette = Some(Palette::Gradient(stops));
        self
    }

    /// Sets raw color scale stops.
    pub fn color_range(mut self, stops: impl Into<Vec<crate::core::Color>>) -> Self {
        let stops: Vec<crate::color::Color> = stops.into().into_iter().map(Into::into).collect();
        self.color.palette = Some(Palette::Gradient(stops));
        self
    }

    /// Replaces the entire color encoding scale (domain + palette +
    /// transform + format) in one go. Use this when you've built a
    /// [`ColorScale<f64>`] elsewhere (e.g. shared across marks).
    pub fn color_scale(mut self, scale: ColorScale<f64>) -> Self {
        self.color = scale;
        self
    }

    /// Sets the explicit value domain `(lo, hi)` on the color scale.
    /// Overrides the data-derived auto-inferred range at draw time.
    pub fn color_domain(mut self, lo: f64, hi: f64) -> Self {
        self.color.domain = Some((lo, hi));
        self
    }

    /// Switches the color scale's transform to linear.
    pub fn linear(mut self) -> Self {
        self.color.transform = crate::scale::Transform::Linear;
        self
    }

    /// Switches the color scale's transform to square root. Useful for
    /// moderately skewed numeric distributions (GDP, population). This is
    /// the default for choropleth.
    pub fn sqrt(mut self) -> Self {
        self.color.transform = crate::scale::Transform::Sqrt;
        self
    }

    /// Switches the color scale's transform to logarithmic. Useful for
    /// data spanning orders of magnitude.
    pub fn log(mut self) -> Self {
        self.color.transform = crate::scale::Transform::Log;
        self
    }

    /// Sets the title shown on the color-scale legend.
    pub fn legend_title(mut self, title: impl Into<String>) -> Self {
        self.legend_title = Some(title.into());
        self
    }

    /// Configures the continuous color-scale guide.
    ///
    /// Pass a [`legend::Config`] to customize anchor / placement / orientation /
    /// text, or `None` to suppress the legend entirely. The default is an
    /// overlaid horizontal bar pinned to the bottom-right.
    pub fn scale_legend(mut self, legend: impl Into<Option<legend::Config>>) -> Self {
        self.legend = legend.into();
        self
    }

    /// Alias for [`Self::scale_legend`]. Kept because many charting
    /// APIs call every guide a legend, while continuous color encodings
    /// read more clearly as a scale guide in authored code.
    pub fn legend(self, legend: impl Into<Option<legend::Config>>) -> Self {
        self.scale_legend(legend)
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

    /// Sets the fill color used for decorative land features that do
    /// not have a choropleth entry.
    pub fn land_color(mut self, color: impl Into<crate::core::Color>) -> Self {
        self.land_color = Some(color.into());
        self
    }

    /// Sets the fill color used for features that have an explicit
    /// available/no-value entry.
    pub fn available_color(mut self, color: impl Into<crate::core::Color>) -> Self {
        self.available_color = Some(color.into());
        self
    }

    /// Sets the fill color used for in-scope features whose entry is
    /// absent from the data. Overrides the theme's
    /// [`Design::missing_fill`](crate::Design::missing_fill) default.
    /// Use this to art-direct news-graphics-style choropleths where
    /// missing/no-data needs a specific brand color.
    pub fn missing_color(mut self, color: impl Into<crate::core::Color>) -> Self {
        self.missing_color = Some(color.into());
        self
    }

    /// Sets the fill color used for the "ocean" — the decorative background
    /// painted under the map in regions that are outside any feature's
    /// polygon. Overrides the theme's
    /// [`Design::ocean_fill`](crate::Design::ocean_fill) default.
    pub fn ocean_color(mut self, color: impl Into<crate::core::Color>) -> Self {
        self.ocean_color = Some(color.into());
        self
    }

    /// Sets a theme-aware hover style closure. The closure runs at
    /// draw time with the chart's [`Design`](crate::design::Design)
    /// and resolved [`palette`](crate::palette::Resolved) in scope,
    /// matching iced's `style: impl Fn(&Theme, Status) -> Style`
    /// pattern — theme switches automatically re-evaluate the
    /// style.
    ///
    /// ```ignore
    /// use hyozu::Color;
    /// use hyozu::mark::choropleth::HoverStyle;
    ///
    /// hyozu::choropleth(entries)
    ///     .hover_style(|design, _palette| HoverStyle {
    ///         outline_width: 2.0,
    ///         ..HoverStyle::from_theme(design)
    ///     })
    /// ```
    pub fn hover_style<F>(mut self, f: F) -> Self
    where
        F: Fn(&dyn crate::design::Design, &crate::palette::Resolved) -> HoverStyle + Send + Sync + 'static,
    {
        self.hover_style = Some(Arc::new(f));
        self
    }

    /// Returns the user-supplied hover style closure, if any.
    pub fn hover_style_fn(&self) -> Option<&HoverStyleFn> {
        self.hover_style.as_ref()
    }

    /// Returns the entries.
    pub fn entries(&self) -> &[ChoroplethEntry] {
        &self.entries
    }

    /// Returns the color-scale's value-format closure if one is
    /// configured. Used by hover overlays to render values with the
    /// same units the scale legend shows.
    pub fn color_scale_format(&self) -> Option<crate::scale::Format<f64>> {
        self.color.format.clone()
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

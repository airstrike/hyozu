use std::cell::RefCell;
use std::collections::HashMap;

use crate::animation;
use crate::chart::scale_legend;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text;
use crate::widget::renderer::geometry;

use super::{Plane, geo};

/// Per-feature display state, derived from the entries map and the
/// feature's value (or absence thereof). Drives the 3-way color decision
/// in `draw`: gradient sample / muted neutral / decorative land fill.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FeatureState {
    /// Has a finite value — render at the gradient sample for `t`.
    Value(f32),
    /// Has an entry but no value (the caller signalled "available, no
    /// measurement") — render at the neutral `available_fill`.
    Available,
    /// No entry at all — render at `land_fill` (decorative background).
    Missing,
}

/// State for Choropleth -- stores the entry-derived per-frame view of the
/// chart-level [`super::geo::Plane`]: feature display states, value range,
/// legend plan, and the resolved-fill snapshot pipeline used by the
/// data-change animation.
///
/// Geometry (projected polygons, filtered ids, feature bboxes) lives on
/// the shared `geo::Plane` reachable from [`super::State::geo_plane`] —
/// not duplicated here.
///
/// The per-feature `feature_state` values, the scale legend plan, and the
/// data value range are all theme-independent and computed in `layout` so
/// `draw` doesn't re-walk `self.data.entries` or rebuild the value lookup
/// HashMap on every repaint. See the `iced layout-vs-draw discipline` note
/// for the broader principle.
pub struct State {
    /// Per-filtered-feature display state, in
    /// `geo::Plane::filtered_ids` order.
    pub feature_state: Vec<FeatureState>,
    /// Min/max of the entry values. Cached so the scale legend doesn't
    /// need to re-walk entries each draw.
    pub value_range: (f64, f64),
    /// Theme-free plan for the scale legend (pre-formatted min/max labels).
    pub legend_plan: scale_legend::Plan,
    /// Per-filtered-feature resolved fill colors, written by
    /// [`Choropleth::draw`] at the start of each repaint and read by
    /// [`crate::chart::Chart::diff`] to seed `previous_fill_colors`
    /// during a data-change replant. Aligned 1:1 with
    /// `geo::Plane::filtered_ids`. Lives in a [`RefCell`] because fill
    /// resolution is theme-dependent and only `draw` carries the
    /// theme — `layout` cannot bake fills without staling on a theme
    /// change.
    pub fill_colors: RefCell<Vec<crate::core::Color>>,
    /// Per-feature fill colors from the most recent layout before the
    /// current one, captured by [`crate::chart::Chart::diff`] when data
    /// changes so the next sweep can interpolate from previous fills to
    /// current fills. Empty on a fresh mount, in which case the
    /// animation collapses to an alpha fade-in at the final color.
    pub previous_fill_colors: Vec<crate::core::Color>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// A Choropleth series that renders geographic features filled with
/// data-driven colors using a sequential color scale.
pub struct Choropleth<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(crate) data: &'a crate::mark::choropleth::Choropleth,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out fill colors.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

/// Build the legend's tick list across `[lo, hi]`. Endpoints are
/// always present at `t = 0.0` / `t = 1.0`; interior ticks sit at
/// nice round values picked by the same Heckbert step the axis tick
/// generator uses (target ~5 ticks across the span). Ticks too close
/// to an endpoint are dropped so labels don't overprint.
fn build_legend_ticks(lo: f64, hi: f64, target: usize, format: &dyn Fn(f64) -> String) -> Vec<scale_legend::Tick> {
    // Degenerate span: collapse to a single tick at lo.
    if !(lo.is_finite() && hi.is_finite()) || hi <= lo {
        return vec![scale_legend::Tick {
            t: 0.0,
            label: format(lo),
        }];
    }
    let span = hi - lo;
    let step = crate::scale::transform::nice_step(span, target);
    let mut ticks = Vec::with_capacity(target + 2);

    // Always include the endpoints — the user reads min/max off them
    // and the bar's gradient runs from one to the other.
    ticks.push(scale_legend::Tick {
        t: 0.0,
        label: format(lo),
    });

    // Interior ticks at multiples of `step` strictly inside (lo, hi).
    // Start from the first multiple `> lo` and stop at the last one
    // `< hi`. The 0.0001-of-span guard rejects ticks within a fifth
    // of a percent of an endpoint so their labels don't overprint
    // the endpoint label.
    let edge_guard = span * 0.05;
    let mut v = (lo / step).ceil() * step;
    while v < hi - edge_guard {
        if v > lo + edge_guard {
            let t = ((v - lo) / span) as f32;
            ticks.push(scale_legend::Tick { t, label: format(v) });
        }
        v += step;
    }

    ticks.push(scale_legend::Tick {
        t: 1.0,
        label: format(hi),
    });
    ticks
}

/// Format a legend value with human-friendly abbreviations. Used as
/// the built-in fallback when the choropleth's legend / ColorScale /
/// Data chain doesn't supply a closure. Also reused by the choropleth
/// hover overlay so hover values pick up the same K/M/B units the
/// legend shows.
pub(crate) fn format_legend_value(v: f64) -> String {
    // Exact zero gets a clean "0" rather than the scientific form
    // produced by the `< 0.01` branch below — `{:.1e}` on 0.0 prints
    // "0.0e0", which is technically right and visually unacceptable.
    if v == 0.0 {
        return "0".to_string();
    }
    let abs = v.abs();
    let raw = if abs >= 1_000_000_000.0 {
        format!("{:.1}B", v / 1_000_000_000.0)
    } else if abs >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if abs >= 1_000.0 {
        format!("{:.1}K", v / 1_000.0)
    } else if abs >= 100.0 {
        format!("{:.0}", v)
    } else if abs >= 1.0 {
        format!("{:.1}", v)
    } else if abs >= 0.01 {
        format!("{:.2}", v)
    } else {
        format!("{:.1e}", v)
    };
    // Strip trailing ".0" from results like "25.0K" → "25K"
    if let Some(rest) = raw.strip_suffix(".0K") {
        format!("{rest}K")
    } else if let Some(rest) = raw.strip_suffix(".0M") {
        format!("{rest}M")
    } else if let Some(rest) = raw.strip_suffix(".0B") {
        format!("{rest}B")
    } else if let Some(rest) = raw.strip_suffix(".0") {
        rest.to_string()
    } else {
        raw
    }
}

/// Map a raw value to [0, 1] via the color scale's transform, clamping
/// the result to the unit interval so non-finite or below-domain inputs
/// can never reach the gradient sampler.
fn normalize(value: f64, v_min: f64, v_max: f64, transform: crate::scale::Transform) -> f32 {
    transform.map_to_unit(value, v_min, v_max).clamp(0.0, 1.0) as f32
}

/// Compute the data-derived `(min, max)` value range for a Choropleth.
/// Filters entries with no value (the "available, no measurement"
/// signal) and skips non-finite values so the gradient sampler never
/// sees NaN or Inf. When all entries are filtered out, returns
/// `(0.0, 1.0)` so the legend has a printable degenerate range.
pub(super) fn compute_value_range(entries: &[crate::mark::choropleth::ChoroplethEntry]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for entry in entries {
        if let Some(v) = entry.value()
            && v.is_finite()
        {
            if v < lo {
                lo = v;
            }
            if v > hi {
                hi = v;
            }
        }
    }
    if lo.is_infinite() {
        lo = 0.0;
    }
    if hi.is_infinite() || hi <= lo {
        hi = lo + 1.0;
    }
    (lo, hi)
}

/// Resolve a [`Palette`](crate::palette::Palette) into a continuous list
/// of raw RGB stops suitable for [`crate::palette::sample_gradient`].
///
/// For [`Palette::Gradient`], stops resolve through their seed slots one
/// by one — preserving the original stop count so a 3-stop "success →
/// warning → danger" gradient samples to its midpoint at `t = 0.5`. For
/// [`Palette::Sequential`], [`Palette::Tonal`], and
/// [`Palette::Categorical`], the palette is
/// discretized through [`crate::palette::Resolved::resolve`] with a
/// fixed sample count and the wrapper colors are then resolved against
/// the same seed.
///
/// Shared with [`super::heatmap`] (Heatmap's layout step needs the same
/// Palette → Vec<core::Color> bridge to feed `sample_gradient`).
pub(super) fn palette_to_continuous_stops(
    palette: &crate::palette::Palette,
    seed: &crate::palette::Seed,
) -> Vec<crate::core::Color> {
    /// Sample count used to discretize non-gradient palettes. Eight is
    /// enough for a smooth visual sweep without the perceptual banding
    /// of three or four stops.
    const DISCRETE_SAMPLES: usize = 8;
    match palette {
        crate::palette::Palette::Gradient(stops) => stops.iter().map(|c| c.resolve_seed(seed)).collect(),
        crate::palette::Palette::Diverging { low, mid, high } => {
            [*low, *mid, *high].into_iter().map(|c| c.resolve_seed(seed)).collect()
        }
        crate::palette::Palette::Sequential(_)
        | crate::palette::Palette::Tonal(_)
        | crate::palette::Palette::Categorical => {
            let resolved = crate::palette::Resolved::resolve(palette, seed, DISCRETE_SAMPLES);
            resolved.colors().iter().map(|c| c.resolve_seed(seed)).collect()
        }
    }
}

/// Default palette for a Choropleth when the user hasn't set one.
/// Continuous choropleths default to a single-hue sequential scale from
/// the active theme's primary color; semantic green/yellow/red gradients
/// are too opinionated for unknown measures.
fn default_choropleth_palette() -> crate::palette::Palette {
    crate::palette::Palette::SEQUENTIAL
}

fn color_stops_for_scale(
    scale: &crate::scale::ColorScale<f64>,
    seed: &crate::palette::Seed,
) -> Vec<crate::core::Color> {
    let palette = scale.resolved_palette(default_choropleth_palette);
    let mut stops = palette_to_continuous_stops(&palette, seed);
    if scale.reverse {
        stops.reverse();
    }
    stops
}

/// Linear-RGB blend of `a` toward `b` by `t` in `[0.0, 1.0]`. `t = 0`
/// returns `a`, `t = 1` returns `b`. Used to fade non-selected
/// value-bearing features toward the muted `land_fill` so the selection
/// reads as the focus without actually hiding the data.
fn blend_colors(a: crate::core::Color, b: crate::core::Color, t: f32) -> crate::core::Color {
    let t = t.clamp(0.0, 1.0);
    crate::core::Color {
        r: a.r * (1.0 - t) + b.r * t,
        g: a.g * (1.0 - t) + b.g * t,
        b: a.b * (1.0 - t) + b.b * t,
        a: a.a * (1.0 - t) + b.a * t,
    }
}

/// Per-channel linear interpolation between `prev` and `cur`. At
/// `progress == 0.0` returns `prev`; at `1.0` returns `cur` exactly.
fn lerp_color(prev: crate::core::Color, cur: crate::core::Color, progress: f32) -> crate::core::Color {
    crate::core::Color {
        r: prev.r + (cur.r - prev.r) * progress,
        g: prev.g + (cur.g - prev.g) * progress,
        b: prev.b + (cur.b - prev.b) * progress,
        a: prev.a + (cur.a - prev.a) * progress,
    }
}

/// Pure helper that maps each filtered feature to its display state
/// given the entries map and the value range. Pulled out so it can be
/// unit-tested without spinning up the full layout pipeline.
pub(super) fn compute_feature_states(
    entries: &[crate::mark::choropleth::ChoroplethEntry],
    filtered_ids: &[crate::feature::Id],
    lo: f64,
    hi: f64,
    transform: crate::scale::Transform,
) -> Vec<FeatureState> {
    // Entry lookup: id → value (or None to mean "available, no value").
    // NaN/Inf-valued entries get downgraded to Available so the gradient
    // path never sees a non-finite t.
    let entry_map: HashMap<&str, Option<f64>> = entries
        .iter()
        .map(|e| {
            let v = e.value.filter(|v| v.is_finite());
            (e.id.as_str(), v)
        })
        .collect();

    filtered_ids
        .iter()
        .map(|id| match entry_map.get(id.as_str()) {
            Some(Some(v)) => FeatureState::Value(normalize(*v, lo, hi, transform)),
            Some(None) => FeatureState::Available,
            None => FeatureState::Missing,
        })
        .collect()
}

impl<'a, Message, Renderer> Choropleth<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub fn new(data: &'a crate::mark::choropleth::Choropleth) -> Self {
        Self {
            data,
            animate: true,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets whether the mount/data-change sweep should run. Wired by
    /// [`super::PlotArea::with_animate`] from [`crate::Data::animate`].
    pub(crate) fn set_animate(&mut self, animate: bool) {
        self.animate = animate;
    }

    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                feature_state: Vec::new(),
                value_range: (0.0, 1.0),
                legend_plan: scale_legend::Plan::default(),
                fill_colors: RefCell::new(Vec::new()),
                previous_fill_colors: Vec::new(),
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
        }
    }

    pub(super) fn diff(&self, _tree: &mut Tree) {}

    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        _plane: &Plane,
        geo_plane: Option<&geo::Plane>,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();

        // Entry-derived state recomputed every layout. value_range,
        // feature_state, and legend strings depend on `self.data.entries`
        // and the color scale's transform, neither of which is covered by
        // the geo plane's projection dirty check. The work is O(n) on
        // entries (typically dozens, not millions), so unconditional
        // recompute is fine. Moves the per-frame walk out of `draw`.
        // Color legends target ~5 ticks; the niceing step in
        // `resolved_numeric_domain` snaps the data-derived range to clean
        // round endpoints (matches D3's `.nice()` default). Explicit
        // `.domain(lo, hi)` on the color scale bypasses nicing — the
        // user's number always wins.
        const COLOR_LEGEND_TICK_TARGET: usize = 5;
        let entries = &self.data.entries;
        let (lo, hi) = self
            .data
            .color
            .resolved_numeric_domain(|| compute_value_range(entries), COLOR_LEGEND_TICK_TARGET);
        state.value_range = (lo, hi);

        // 3-way state per filtered feature:
        //   Value(t) — entry with a finite value, gradient-encoded.
        //   Available — entry with `value = None` (caller signalled the
        //     feature is in the active context but carries no measurement;
        //     renders muted).
        //   Missing — no entry; renders as decorative land fill.
        // NaN/Inf-valued entries are downgraded to Available so non-finite
        // values can never reach `sample_gradient → from_oklch`.
        let filtered_ids: &[crate::feature::Id] = match geo_plane {
            Some(plane) => &plane.filtered_ids,
            None => &[],
        };
        state.feature_state = compute_feature_states(entries, filtered_ids, lo, hi, self.data.color.transform);

        // Legend value-format chain: a guide-level override on
        // `legend::Config.value_format` wins, then the mark-level
        // `ColorScale.format`, then the built-in `format_legend_value`
        // abbreviation. Data-level `Data::value_format` is not reachable
        // from this renderer today (no thread-through from scene); a
        // caller wanting it applied to the legend can pass the same
        // closure via `legend::Config::value_format`.
        let legend_format = self
            .data
            .legend
            .as_ref()
            .and_then(|l| l.value_format_ref().cloned())
            .or_else(|| self.data.color.format.clone());
        let format = |v: f64| -> String {
            match &legend_format {
                Some(f) => f(&v),
                None => format_legend_value(v),
            }
        };
        // Pick 3-7 nice tick values across [lo, hi] using the same
        // Heckbert step the axis ticks use, so the gradient gets the
        // same reading anchors a D3 / Vega-Lite continuous legend
        // shows. Endpoints are always present; interior ticks sit at
        // multiples of the step that fall inside `(lo, hi)`.
        const LEGEND_TICK_TARGET: usize = 5;
        state.legend_plan = scale_legend::Plan {
            ticks: build_legend_ticks(lo, hi, LEGEND_TICK_TARGET, &format),
        };

        Node::new(Size::ZERO)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw<Theme>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &crate::core::renderer::Style,
        layout: crate::core::Layout<'_>,
        _cursor: crate::core::mouse::Cursor,
        _viewport: &crate::core::Rectangle,
        _color_offset: usize,
        _palette: &crate::palette::Resolved,
        geo_plane: Option<&geo::Plane>,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        let Some(plane) = geo_plane else {
            return;
        };
        if plane.projected_polygons.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();
        let layout_bounds = layout.bounds();

        // ── Resolve color scale ───────────────────────────────────
        // Theme-dependent (uses the palette seed when no explicit
        // palette is set), so it stays in draw. The user's explicit
        // palette and direction win; otherwise the mark defaults to the
        // theme's primary sequential scale.
        let color_stops = color_stops_for_scale(&self.data.color, &seed);

        // ── Derived colors ───────────────────────────────────────
        let default_land_fill = crate::core::Color {
            r: background.r * 0.92 + 0.08 * 0.7,
            g: background.g * 0.92 + 0.08 * 0.72,
            b: background.b * 0.92 + 0.08 * 0.74,
            a: 1.0,
        };
        let land_fill = self.data.land_color.unwrap_or(default_land_fill);
        // "In scope, clickable, but no value to encode" tint — same neutral
        // hue family as `land_fill`, nudged one OKLch lightness pass toward
        // the foreground so the eye reads it as a distinct interactive
        // affordance rather than as decorative background. Stays clear of
        // the gradient's red/yellow/green channel which carries value
        // semantics.
        let available_fill = self
            .data
            .available_color
            .unwrap_or_else(|| crate::palette::shift_lightness(land_fill, background, 2));
        // "In-scope feature with no entry" — distinct slot from
        // `land_fill` (which paints purely out-of-scope decoration).
        // User override on the mark wins; otherwise fall back to the
        // theme's `missing_fill`.
        let missing_fill = self.data.missing_color.unwrap_or_else(|| theme.missing_fill());
        let border_color = theme.divider_color().resolve(background, text_pair, &seed, None);

        // ── Draw ocean background ────────────────────────────────
        let mut frame = Frame::new(renderer, layout_bounds.size());

        // Ocean color comes from the theme so a host wrapper can match
        // the choropleth's painted background via `theme.ocean_fill()`
        // without hardcoding an RGB constant that drifts when the theme
        // changes. A per-mark override on the data wins when set.
        let ocean_color = self.data.ocean_color.unwrap_or_else(|| theme.ocean_fill());

        let ocean = Path::new(|builder| {
            builder.rectangle(
                crate::core::Point::ORIGIN,
                crate::core::Size::new(layout_bounds.width, layout_bounds.height),
            );
        });
        frame.fill(&ocean, ocean_color);

        // ── Resolve target fill per feature ──────────────────────
        // Target color resolution is theme-dependent (gradient seed,
        // land_fill blend) so it lives here rather than in layout.
        // The cache is written to `state.fill_colors` so a subsequent
        // data-change replant in `chart::diff` can snapshot it onto
        // the post-rebuild tree's `previous_fill_colors`.
        let target_fills: Vec<crate::core::Color> = plane
            .projected_polygons
            .iter()
            .enumerate()
            .map(|(feat_idx, _)| match state.feature_state.get(feat_idx).copied() {
                Some(FeatureState::Value(t)) => {
                    let gradient = crate::palette::sample_gradient(&color_stops, t);
                    let id = plane.filtered_ids.get(feat_idx);
                    let is_selected = self
                        .data
                        .selected
                        .as_ref()
                        .zip(id)
                        .map(|(set, id)| set.contains(id))
                        .unwrap_or(true);
                    if is_selected {
                        gradient
                    } else {
                        blend_colors(gradient, land_fill, 0.5)
                    }
                }
                Some(FeatureState::Available) => available_fill,
                Some(FeatureState::Missing) => missing_fill,
                None => land_fill,
            })
            .collect();

        // ── Animation gating ─────────────────────────────────────
        // With a previous-layout snapshot (data change), each region's
        // fill lerps per-channel from prev to cur. Without one (fresh
        // mount), the alpha component multiplies by progress so regions
        // fade in from fully transparent to their final color. With
        // `animate = false` progress pins to `1.0` and the rendered
        // color equals `target_fills[i]` exactly. Geometry (projected
        // polygons) does not animate — projection is too expensive to
        // recompute per-frame.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let has_prev = !state.previous_fill_colors.is_empty();

        // ── Draw features ────────────────────────────────────────
        for (feat_idx, feature_rings) in plane.projected_polygons.iter().enumerate() {
            let target = target_fills[feat_idx];
            let fill = if has_prev {
                let prev = state.previous_fill_colors.get(feat_idx).copied().unwrap_or(target);
                lerp_color(prev, target, progress)
            } else {
                crate::core::Color {
                    a: target.a * progress,
                    ..target
                }
            };

            for ring in feature_rings {
                if ring.len() < 3 {
                    continue;
                }

                let path = Path::new(|builder| {
                    let (x, y) = ring[0];
                    builder.move_to(crate::core::Point::new(x, y));
                    for &(x, y) in &ring[1..] {
                        builder.line_to(crate::core::Point::new(x, y));
                    }
                    builder.close();
                });

                frame.fill(&path, fill);
                frame.stroke(&path, Stroke::default().with_color(border_color).with_width(0.5));
            }
        }

        // Snapshot the resolved targets for the next replant to seed
        // `previous_fill_colors` from. Always written, regardless of
        // `self.animate`, so toggling animation on later still has a
        // valid baseline.
        *state.fill_colors.borrow_mut() = target_fills;

        // ── Composite ────────────────────────────────────────────
        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(frame.into_geometry());
        });
    }

    /// Draws this choropleth's color-scale legend into its own frame.
    ///
    /// When `strip_rect` is `Some`, the legend renders inside that
    /// scene-local rectangle — the reserved strip carved out by
    /// [`crate::Data::scale_legend_reservations`]. When `None`, it falls
    /// through to overlay placement inside the plot area's own bounds.
    ///
    /// Called from [`super::PlotArea::draw_scale_legends`] after the plot
    /// area's own `draw` pass so the legend composites on top of the map
    /// geometry.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_scale_legend<Theme>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        plot_layout_bounds: crate::core::Rectangle,
        strip_rect: Option<crate::core::Rectangle>,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let Some(legend_config) = &self.data.legend else {
            return;
        };

        let state = tree.state.downcast_ref::<State>();

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        let color_stops = color_stops_for_scale(&self.data.color, &seed);
        let border_color = theme.divider_color().resolve(background, text_pair, &seed, None);
        let label_color = {
            let resolved = text_pair.resolve(background, None);
            crate::core::Color { a: 0.7, ..resolved }
        };

        // Frame coordinate system depends on whether we're drawing in the
        // strip (scene-local) or inside the plot area (plot-local).
        let (frame_size, translation, plot_bounds, strip_local) = match strip_rect {
            Some(strip) => (
                strip.size(),
                crate::core::Vector::new(strip.x, strip.y),
                crate::core::Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: plot_layout_bounds.width,
                    height: plot_layout_bounds.height,
                },
                Some(crate::core::Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: strip.width,
                    height: strip.height,
                }),
            ),
            None => (
                plot_layout_bounds.size(),
                crate::core::Vector::new(plot_layout_bounds.x, plot_layout_bounds.y),
                crate::core::Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: plot_layout_bounds.width,
                    height: plot_layout_bounds.height,
                },
                None,
            ),
        };

        let mut frame = Frame::new(renderer, frame_size);
        scale_legend::draw(
            &mut frame,
            &state.legend_plan,
            legend_config,
            self.data.legend_title.as_deref(),
            plot_bounds,
            strip_local,
            &color_stops,
            background,
            border_color,
            label_color,
            theme,
        );

        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(frame.into_geometry());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Color, color};
    use crate::feature::Id;
    use crate::mark::choropleth::{choropleth_entry, choropleth_entry_available};
    use crate::palette::{Seed, sample_gradient, sequential, to_oklch};
    use crate::scale::Transform;

    fn id(s: &str) -> Id {
        Id::new(s.to_owned())
    }

    #[test]
    fn feature_state_distinguishes_value_available_and_missing() {
        // Mirrors the drill_down map case: CA has a fill value, TX has
        // no fill value but is in the active context (e.g., size-metric
        // sibling layer carries data), NM has no entry at all.
        let entries = vec![choropleth_entry("CA", 4_500_000.0), choropleth_entry_available("TX")];
        let filtered_ids = vec![id("CA"), id("TX"), id("NM")];

        let states = compute_feature_states(&entries, &filtered_ids, 0.0, 4_500_000.0, Transform::Linear);

        assert_eq!(states.len(), 3);
        assert!(matches!(states[0], FeatureState::Value(_)), "CA should be Value");
        assert_eq!(states[1], FeatureState::Available, "TX should be Available");
        assert_eq!(states[2], FeatureState::Missing, "NM should be Missing");
    }

    #[test]
    fn nonfinite_entry_value_downgrades_to_available() {
        // Defence-in-depth: an entry whose value sneaks through as NaN
        // (e.g. through a missing tatami cell that wasn't filtered
        // upstream) must NOT reach the gradient sampler — that would
        // panic in iced's Color::new debug_assert. Downgrade to
        // Available so the renderer paints the muted neutral fill.
        let entries = vec![choropleth_entry("CA", f64::NAN), choropleth_entry("TX", f64::INFINITY)];
        let filtered_ids = vec![id("CA"), id("TX")];

        let states = compute_feature_states(&entries, &filtered_ids, 0.0, 1.0, Transform::Linear);

        assert_eq!(states[0], FeatureState::Available);
        assert_eq!(states[1], FeatureState::Available);
    }

    #[test]
    fn value_state_carries_normalized_t_in_unit_range() {
        let entries = vec![
            choropleth_entry("LO", 0.0),
            choropleth_entry("MID", 50.0),
            choropleth_entry("HI", 100.0),
        ];
        let filtered_ids = vec![id("LO"), id("MID"), id("HI")];

        let states = compute_feature_states(&entries, &filtered_ids, 0.0, 100.0, Transform::Linear);

        let unwrap_t = |s: FeatureState| match s {
            FeatureState::Value(t) => t,
            _ => panic!("expected Value, got {s:?}"),
        };
        assert!((unwrap_t(states[0]) - 0.0).abs() < 1e-6);
        assert!((unwrap_t(states[1]) - 0.5).abs() < 1e-6);
        assert!((unwrap_t(states[2]) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn blend_colors_at_50_percent_is_arithmetic_mean() {
        // The selection-fade blend uses linear-RGB interpolation. For a
        // black → white blend at t=0.5 the expected channels are 0.5;
        // for an orange (1, 0.5, 0) → grey (0.4, 0.4, 0.4) blend the
        // expected channels are (0.7, 0.45, 0.2). Both directions plus
        // the alpha channel are checked so a regression in any axis
        // surfaces immediately.
        let black = crate::core::Color::from_rgba(0.0, 0.0, 0.0, 1.0);
        let white = crate::core::Color::from_rgba(1.0, 1.0, 1.0, 1.0);
        let mid = blend_colors(black, white, 0.5);
        assert!((mid.r - 0.5).abs() < 1e-6);
        assert!((mid.g - 0.5).abs() < 1e-6);
        assert!((mid.b - 0.5).abs() < 1e-6);
        assert!((mid.a - 1.0).abs() < 1e-6);

        let orange = crate::core::Color::from_rgba(1.0, 0.5, 0.0, 1.0);
        let grey = crate::core::Color::from_rgba(0.4, 0.4, 0.4, 0.6);
        let blended = blend_colors(orange, grey, 0.5);
        assert!((blended.r - 0.7).abs() < 1e-6);
        assert!((blended.g - 0.45).abs() < 1e-6);
        assert!((blended.b - 0.2).abs() < 1e-6);
        assert!((blended.a - 0.8).abs() < 1e-6);
    }

    #[test]
    fn sequential_palette_produces_distinct_samples_across_t() {
        // End-to-end check for the path a caller hits when they pass a
        // single-hue palette (e.g. `palette::sequential(brand_blue)`)
        // through the chart's color scale. The renderer discretizes the
        // sequential palette to N stops via `palette_to_continuous_stops`,
        // then samples it with `sample_gradient` at each feature's `t`.
        // Adjacent t values must produce perceptibly distinct colors —
        // otherwise the choropleth flattens to a single shade.
        //
        // Property: samples at t ∈ {0.0, 0.5, 1.0} span ≥ 0.40 OKLch L on
        // both light and dark backgrounds. Lower than `generate_sequential`'s
        // own 0.45 span asserted in `palette.rs` because the sqrt domain
        // mapping squeezes the visible band slightly; 0.40 leaves headroom
        // without becoming a meaningless guard.
        let blue = color!(0x3366cc);
        let palette = sequential(blue);

        for (label, background) in [("light", Color::WHITE), ("dark", Color::BLACK)] {
            let seed = Seed {
                primary: blue,
                secondary: blue,
                success: blue,
                warning: blue,
                danger: blue,
                background,
            };
            let stops = palette_to_continuous_stops(&palette, &seed);
            let samples: Vec<Color> = [0.0, 0.5, 1.0]
                .into_iter()
                .map(|t| sample_gradient(&stops, t))
                .collect();
            let lightnesses: Vec<f32> = samples.iter().map(|c| to_oklch(*c).l).collect();
            let l_min = lightnesses.iter().copied().fold(f32::INFINITY, f32::min);
            let l_max = lightnesses.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            assert!(
                l_max - l_min >= 0.40,
                "{label} bg: sequential samples at t=0/0.5/1.0 should span ≥ 0.40 OKLch L, got {l_max} - {l_min} = {} ({:?})",
                l_max - l_min,
                lightnesses
            );
        }
    }

    #[test]
    fn reverse_scale_flips_continuous_stops() {
        let blue = color!(0x3366cc);
        let seed = Seed {
            primary: blue,
            secondary: blue,
            success: blue,
            warning: blue,
            danger: blue,
            background: Color::WHITE,
        };
        let forward = crate::scale::ColorScale::default().palette(sequential(blue));
        let reversed = forward.clone().reverse(true);

        let forward_stops = color_stops_for_scale(&forward, &seed);
        let reversed_stops = color_stops_for_scale(&reversed, &seed);

        assert_eq!(forward_stops.first(), reversed_stops.last());
        assert_eq!(forward_stops.last(), reversed_stops.first());
    }

    #[test]
    fn default_choropleth_palette_is_single_hue_primary() {
        assert_eq!(default_choropleth_palette(), crate::palette::Palette::SEQUENTIAL);
    }

    #[test]
    fn blend_colors_at_endpoints_returns_inputs() {
        let a = crate::core::Color::from_rgba(0.1, 0.2, 0.3, 0.4);
        let b = crate::core::Color::from_rgba(0.9, 0.8, 0.7, 0.6);
        let at_zero = blend_colors(a, b, 0.0);
        let at_one = blend_colors(a, b, 1.0);
        assert!((at_zero.r - a.r).abs() < 1e-6 && (at_zero.a - a.a).abs() < 1e-6);
        assert!((at_one.r - b.r).abs() < 1e-6 && (at_one.a - b.a).abs() < 1e-6);
    }
}

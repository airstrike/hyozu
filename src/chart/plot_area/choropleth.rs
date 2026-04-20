use std::collections::HashMap;
use std::sync::Arc;

use crate::chart::scale_legend;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text;
use crate::widget::renderer::geometry;

use super::Plane;

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

/// State for Choropleth -- stores projected polygon geometry, feature IDs,
/// bounding boxes, and pre-computed value normalization for each feature.
///
/// The per-feature `feature_state` values, the scale legend plan, and the
/// data value range are all theme-independent and computed in `layout` so
/// `draw` doesn't re-walk `self.data.entries` or rebuild the value lookup
/// HashMap on every repaint. See the `iced layout-vs-draw discipline` note
/// for the broader principle.
pub struct State {
    pub projected_polygons: Vec<Vec<Vec<(f32, f32)>>>,
    pub filtered_ids: Vec<crate::feature::Id>,
    pub feature_bboxes: Vec<crate::core::Rectangle>,
    /// Per-filtered-feature display state, in `filtered_ids` order.
    pub feature_state: Vec<FeatureState>,
    /// Min/max of the entry values. Cached so the scale legend doesn't
    /// need to re-walk entries each draw.
    pub value_range: (f64, f64),
    /// Theme-free plan for the scale legend (pre-formatted min/max labels).
    pub legend_plan: scale_legend::Plan,
    prev_size: (f32, f32),
    prev_scope: crate::geo::MapScope,
    prev_geo: Option<Arc<crate::geo::GeoData>>,
}

/// A Choropleth series that renders geographic features filled with
/// data-driven colors using a sequential color scale.
pub struct Choropleth<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::choropleth::Choropleth,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

/// Format a legend value with human-friendly abbreviations.
fn format_legend_value(v: f64) -> String {
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

/// Apply normalization to map a raw value to [0, 1].
fn normalize(value: f64, v_min: f64, v_max: f64, norm: crate::mark::choropleth::Normalization) -> f32 {
    use crate::mark::choropleth::Normalization;
    let range = v_max - v_min;
    if range <= 0.0 {
        return 0.5;
    }
    let t = match norm {
        Normalization::Linear => (value - v_min) / range,
        Normalization::Sqrt => ((value - v_min) / range).sqrt(),
        Normalization::Log => (1.0 + value - v_min).ln() / (1.0 + range).ln(),
    };
    t.clamp(0.0, 1.0) as f32
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

/// Pure helper that maps each filtered feature to its display state
/// given the entries map and the value range. Pulled out so it can be
/// unit-tested without spinning up the full layout pipeline.
pub(super) fn compute_feature_states(
    entries: &[crate::mark::choropleth::ChoroplethEntry],
    filtered_ids: &[crate::feature::Id],
    lo: f64,
    hi: f64,
    norm: crate::mark::choropleth::Normalization,
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
            Some(Some(v)) => FeatureState::Value(normalize(*v, lo, hi, norm)),
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
            _marker: std::marker::PhantomData,
        }
    }

    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                projected_polygons: Vec::new(),
                filtered_ids: Vec::new(),
                feature_bboxes: Vec::new(),
                feature_state: Vec::new(),
                value_range: (0.0, 1.0),
                legend_plan: scale_legend::Plan::default(),
                prev_size: (0.0, 0.0),
                prev_scope: crate::geo::MapScope::World,
                prev_geo: None,
            }),
            children: Vec::new(),
        }
    }

    pub(super) fn diff(&self, _tree: &mut Tree) {}

    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, limits: &Limits, _plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();
        let size = limits.max();
        let current_size = (size.width, size.height);

        // Dirty check for the EXPENSIVE projection step: skip re-projection
        // if size, scope, and geo are unchanged. The cheaper entry-derived
        // state (value range, normalized t, legend strings) is recomputed
        // unconditionally below — entries can change without triggering the
        // projection-level dirty bits, and the entry walk is O(n) on a
        // small collection.
        let geo_changed = match (&state.prev_geo, &self.data.geo) {
            (Some(prev), Some(cur)) => !Arc::ptr_eq(prev, cur),
            (None, None) => false,
            _ => true,
        };
        let needs_reproject = geo_changed || state.prev_size != current_size || state.prev_scope != self.data.scope;

        if needs_reproject {
            // Update dirty-check fields.
            state.prev_size = current_size;
            state.prev_scope = self.data.scope;
            state.prev_geo = self.data.geo.clone();

            match &self.data.geo {
                None => {
                    state.projected_polygons.clear();
                    state.filtered_ids.clear();
                    state.feature_bboxes.clear();
                }
                Some(geo) => {
                    // Filter features by scope and build projection.
                    let filtered = geo.filter_by_scope(self.data.scope);
                    let scope_bounds = self.data.scope.bounds();
                    let projection = crate::geo::Projection::new(self.data.projection).fit_size(
                        size.width,
                        size.height,
                        scope_bounds,
                    );

                    // Project all filtered-feature polygons into pixel space.
                    state.projected_polygons = filtered
                        .features
                        .iter()
                        .map(|feature| {
                            feature
                                .polygons
                                .iter()
                                .map(|ring| ring.iter().map(|&(lon, lat)| projection.project(lon, lat)).collect())
                                .collect()
                        })
                        .collect();

                    // Store the ID of each filtered feature in the same order.
                    state.filtered_ids = filtered.features.iter().map(|f| f.id.clone()).collect();

                    // Compute bounding boxes for each feature.
                    state.feature_bboxes = state
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
                            crate::core::Rectangle {
                                x: min_x,
                                y: min_y,
                                width: (max_x - min_x).max(0.0),
                                height: (max_y - min_y).max(0.0),
                            }
                        })
                        .collect();
                }
            }
        }

        // ── Entry-derived state (always recomputed) ──────────────────
        //
        // value_range, feature_state, and legend strings depend on
        // `self.data.entries` and `self.data.normalization`, neither of
        // which is covered by the projection dirty check. The work is O(n)
        // on entries (typically dozens, not millions), so unconditional
        // recompute is fine. Moves the per-frame walk out of `draw`.
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for entry in &self.data.entries {
            if let Some(v) = entry.value
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
        state.value_range = (lo, hi);

        // 3-way state per filtered feature:
        //   Value(t) — entry with a finite value, gradient-encoded.
        //   Available — entry with `value = None` (caller signalled the
        //     feature is in the active context but carries no measurement;
        //     renders muted).
        //   Missing — no entry; renders as decorative land fill.
        // NaN/Inf-valued entries are downgraded to Available so non-finite
        // values can never reach `sample_gradient → from_oklch`.
        state.feature_state =
            compute_feature_states(&self.data.entries, &state.filtered_ids, lo, hi, self.data.normalization);

        state.legend_plan = scale_legend::Plan {
            min_label: format_legend_value(lo),
            max_label: format_legend_value(hi),
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
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        if state.projected_polygons.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();
        let layout_bounds = layout.bounds();

        // ── Resolve color scale ───────────────────────────────────
        // Theme-dependent (uses the palette seed when no explicit stops are
        // set), so it stays in draw. Cheap — at most a 3-color clone.
        let color_stops: Vec<crate::core::Color> = if let Some(stops) = &self.data.color_stops {
            stops.clone()
        } else {
            vec![seed.success, seed.warning, seed.danger]
        };

        // ── Derived colors ───────────────────────────────────────
        let land_fill = crate::core::Color {
            r: background.r * 0.92 + 0.08 * 0.7,
            g: background.g * 0.92 + 0.08 * 0.72,
            b: background.b * 0.92 + 0.08 * 0.74,
            a: 1.0,
        };
        // "In scope, clickable, but no value to encode" tint — same neutral
        // hue family as `land_fill`, nudged one OKLch lightness pass toward
        // the foreground so the eye reads it as a distinct interactive
        // affordance rather than as decorative background. Stays clear of
        // the gradient's red/yellow/green channel which carries value
        // semantics.
        let available_fill = crate::palette::shift_lightness(land_fill, background, 2);
        let border_color = theme.divider_color().resolve(background, text_pair, &seed, None);

        // ── Draw ocean background ────────────────────────────────
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let bg_oklch = crate::palette::to_oklch(background);
        let is_dark = bg_oklch.l < 0.5;
        let ocean_color = if is_dark {
            // Dark mode: deep navy
            crate::core::Color {
                r: 0.10,
                g: 0.14,
                b: 0.20,
                a: 1.0,
            }
        } else {
            // Light mode: soft blue
            crate::core::Color {
                r: 0.83,
                g: 0.90,
                b: 0.95,
                a: 1.0,
            }
        };

        let ocean = Path::new(|builder| {
            builder.rectangle(
                crate::core::Point::ORIGIN,
                crate::core::Size::new(layout_bounds.width, layout_bounds.height),
            );
        });
        frame.fill(&ocean, ocean_color);

        // ── Draw features ────────────────────────────────────────

        for (feat_idx, feature_rings) in state.projected_polygons.iter().enumerate() {
            // Per-feature display state was computed in layout; sampling the
            // gradient (or picking a neutral fill) is the only theme-touching
            // step left here. When a selection set is active, value-bearing
            // features outside the set blend halfway toward `land_fill` so
            // the selected feature(s) read as the focus.
            let fill = match state.feature_state.get(feat_idx).copied() {
                Some(FeatureState::Value(t)) => {
                    let gradient = crate::palette::sample_gradient(&color_stops, t);
                    let id = state.filtered_ids.get(feat_idx);
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
                Some(FeatureState::Missing) | None => land_fill,
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

        let color_stops: Vec<crate::core::Color> = if let Some(stops) = &self.data.color_stops {
            stops.clone()
        } else {
            vec![seed.success, seed.warning, seed.danger]
        };
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
    use crate::feature::Id;
    use crate::mark::choropleth::{Normalization, choropleth_entry, choropleth_entry_available};

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

        let states = compute_feature_states(&entries, &filtered_ids, 0.0, 4_500_000.0, Normalization::Linear);

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

        let states = compute_feature_states(&entries, &filtered_ids, 0.0, 1.0, Normalization::Linear);

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

        let states = compute_feature_states(&entries, &filtered_ids, 0.0, 100.0, Normalization::Linear);

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
    fn blend_colors_at_endpoints_returns_inputs() {
        let a = crate::core::Color::from_rgba(0.1, 0.2, 0.3, 0.4);
        let b = crate::core::Color::from_rgba(0.9, 0.8, 0.7, 0.6);
        let at_zero = blend_colors(a, b, 0.0);
        let at_one = blend_colors(a, b, 1.0);
        assert!((at_zero.r - a.r).abs() < 1e-6 && (at_zero.a - a.a).abs() < 1e-6);
        assert!((at_one.r - b.r).abs() < 1e-6 && (at_one.a - b.a).abs() < 1e-6);
    }
}

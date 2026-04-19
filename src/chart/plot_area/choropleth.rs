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

/// State for Choropleth -- stores projected polygon geometry, feature IDs,
/// bounding boxes, and pre-computed value normalization for each feature.
///
/// The per-feature `feature_t` values, the scale legend plan, and the data
/// value range are all theme-independent and computed in `layout` so
/// `draw` doesn't re-walk `self.data.entries` or rebuild the value lookup
/// HashMap on every repaint. See the `iced layout-vs-draw discipline` note
/// for the broader principle.
pub struct State {
    pub projected_polygons: Vec<Vec<Vec<(f32, f32)>>>,
    pub filtered_ids: Vec<crate::feature::Id>,
    pub feature_bboxes: Vec<crate::core::Rectangle>,
    /// Normalized [0,1] value for each filtered feature, in `filtered_ids`
    /// order. `None` for features without an entry in `self.data.entries`
    /// (those render as land-fill).
    pub feature_t: Vec<Option<f32>>,
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
                feature_t: Vec::new(),
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
        // value_range, feature_t, and legend strings depend on
        // `self.data.entries` and `self.data.normalization`, neither of
        // which is covered by the projection dirty check. The work is O(n)
        // on entries (typically dozens, not millions), so unconditional
        // recompute is fine. Moves the per-frame walk out of `draw`.
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for entry in &self.data.entries {
            if entry.value < lo {
                lo = entry.value;
            }
            if entry.value > hi {
                hi = entry.value;
            }
        }
        if lo.is_infinite() {
            lo = 0.0;
        }
        if hi.is_infinite() || hi <= lo {
            hi = lo + 1.0;
        }
        state.value_range = (lo, hi);

        // Build the value lookup once. The HashMap lives only for the rest
        // of this layout call; feature_t materializes the result so draw
        // doesn't need it.
        let value_map: HashMap<&str, f64> = self.data.entries.iter().map(|e| (e.id.as_str(), e.value)).collect();

        state.feature_t = state
            .filtered_ids
            .iter()
            .map(|id| {
                value_map
                    .get(id.as_str())
                    .map(|&v| normalize(v, lo, hi, self.data.normalization))
            })
            .collect();

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
            // Per-feature normalized value was computed in layout. Sampling
            // the gradient is the only theme-touching step left here.
            let fill = match state.feature_t.get(feat_idx).copied().flatten() {
                Some(t) => crate::palette::sample_gradient(&color_stops, t),
                None => land_fill,
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

use std::sync::Arc;

use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

use super::Plane;

// ── State ──────────────────────────────────────────────────────────

/// Cached projection data for the bubble map renderer.
pub struct State {
    /// Pixel center and radius for each bubble (in local coordinates).
    pub bubble_circles: Vec<(crate::core::Point, f32)>,
    /// Projected polygon rings per feature, in pixel coordinates.
    pub projected_polygons: Vec<Vec<Vec<(f32, f32)>>>,
    /// Axis-aligned bounding box per feature (for future culling / hit-testing).
    pub feature_bboxes: Vec<crate::core::Rectangle>,
    /// The projection used for the current frame.
    pub projection: Option<crate::geo::Projection>,
    // Dirty-check fields
    prev_size: (f32, f32),
    prev_scope: crate::geo::MapScope,
    prev_geo: Option<Arc<crate::geo::GeoData>>,
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl State {
    pub fn new() -> Self {
        Self {
            bubble_circles: Vec::new(),
            projected_polygons: Vec::new(),
            feature_bboxes: Vec::new(),
            projection: None,
            prev_size: (0.0, 0.0),
            prev_scope: crate::geo::MapScope::World,
            prev_geo: None,
        }
    }

    /// Returns `true` when the cached polygon data needs recomputation.
    fn is_dirty(&self, size: (f32, f32), scope: crate::geo::MapScope, geo: &Option<Arc<crate::geo::GeoData>>) -> bool {
        if self.prev_size != size || self.prev_scope != scope {
            return true;
        }
        match (&self.prev_geo, geo) {
            (Some(a), Some(b)) => !Arc::ptr_eq(a, b),
            (None, None) => false,
            _ => true,
        }
    }

    fn mark_clean(&mut self, size: (f32, f32), scope: crate::geo::MapScope, geo: &Option<Arc<crate::geo::GeoData>>) {
        self.prev_size = size;
        self.prev_scope = scope;
        self.prev_geo = geo.clone();
    }
}

// ── Widget ─────────────────────────────────────────────────────────

/// A BubbleMap series that renders a geographic basemap with sized bubbles.
pub struct BubbleMap<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::bubble_map::BubbleMap,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> BubbleMap<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub fn new(data: &'a crate::mark::bubble_map::BubbleMap) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State::new()),
            children: Vec::new(),
        }
    }

    pub(super) fn diff(&self, _tree: &mut Tree) {}

    // ── Layout ─────────────────────────────────────────────────────

    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, limits: &Limits, _plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();
        let size = limits.max();
        let scope = self.data.scope;

        // Build projection for this viewport + scope.
        let projection =
            crate::geo::Projection::new(self.data.projection).fit_size(size.width, size.height, scope.bounds());
        state.projection = Some(projection);

        // Reproject basemap polygons only when inputs change.
        let size_key = (size.width, size.height);
        if state.is_dirty(size_key, scope, &self.data.geo) {
            state.projected_polygons.clear();
            state.feature_bboxes.clear();

            if let Some(geo) = &self.data.geo {
                let filtered = geo.filter_by_scope(scope);

                for feature in &filtered.features {
                    let mut projected_rings: Vec<Vec<(f32, f32)>> = Vec::new();
                    let mut bbox_min_x = f32::INFINITY;
                    let mut bbox_min_y = f32::INFINITY;
                    let mut bbox_max_x = f32::NEG_INFINITY;
                    let mut bbox_max_y = f32::NEG_INFINITY;

                    for ring in &feature.polygons {
                        let projected: Vec<(f32, f32)> = ring
                            .iter()
                            .map(|&(lon, lat)| {
                                let (px, py) = projection.project(lon, lat);
                                bbox_min_x = bbox_min_x.min(px);
                                bbox_min_y = bbox_min_y.min(py);
                                bbox_max_x = bbox_max_x.max(px);
                                bbox_max_y = bbox_max_y.max(py);
                                (px, py)
                            })
                            .collect();
                        projected_rings.push(projected);
                    }

                    state.feature_bboxes.push(crate::core::Rectangle {
                        x: bbox_min_x,
                        y: bbox_min_y,
                        width: (bbox_max_x - bbox_min_x).max(0.0),
                        height: (bbox_max_y - bbox_min_y).max(0.0),
                    });
                    state.projected_polygons.push(projected_rings);
                }
            }

            state.mark_clean(size_key, scope, &self.data.geo);
        }

        // Always recompute bubble positions (cheap).
        state.bubble_circles = self
            .data
            .points
            .iter()
            .map(|pt| {
                let (px, py) = projection.project(pt.lon as f32, pt.lat as f32);
                (crate::core::Point::new(px, py), 0.0)
            })
            .collect();

        // Pre-compute bubble radii.
        let (v_min, v_max) = value_range(&self.data.points);
        for (i, pt) in self.data.points.iter().enumerate() {
            let r = interpolate_radius(
                pt.value as f32,
                v_min,
                v_max,
                self.data.min_radius,
                self.data.max_radius,
            );
            state.bubble_circles[i].1 = r;
        }

        Node::new(Size::ZERO)
    }

    // ── Draw ───────────────────────────────────────────────────────

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
        color_offset: usize,
        palette: &crate::palette::Resolved,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();
        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();
        let layout_bounds = layout.bounds();

        // ── Land polygons ──────────────────────────────────────────
        let mut land_frame = Frame::new(renderer, layout_bounds.size());

        if self.data.show_basemap && !state.projected_polygons.is_empty() {
            let land_fill = crate::core::Color {
                r: background.r * 0.92 + 0.08 * 0.7,
                g: background.g * 0.92 + 0.08 * 0.72,
                b: background.b * 0.92 + 0.08 * 0.74,
                a: 1.0,
            };
            let border_color = theme.divider_color().resolve(background, text_pair, &seed, None);
            let border_stroke = Stroke::default().with_color(border_color).with_width(0.5);

            for rings in &state.projected_polygons {
                for ring in rings {
                    if ring.len() < 3 {
                        continue;
                    }

                    let path = Path::new(|builder| {
                        let (x0, y0) = ring[0];
                        builder.move_to(crate::core::Point::new(x0, y0));
                        for &(x, y) in &ring[1..] {
                            builder.line_to(crate::core::Point::new(x, y));
                        }
                        builder.close();
                    });

                    land_frame.fill(&path, land_fill);
                    land_frame.stroke(&path, border_stroke);
                }
            }
        }

        // ── Bubbles ────────────────────────────────────────────────
        let mut bubble_frame = Frame::new(renderer, layout_bounds.size());

        let opacity = self.data.opacity;

        for (i, pt) in self.data.points.iter().enumerate() {
            let (center, radius) = state.bubble_circles[i];

            let base_color = if let Some(c) = pt.color {
                c.resolve(background, text_pair, &seed, None)
            } else {
                palette
                    .get(color_offset + i)
                    .resolve(background, text_pair, &seed, None)
            };

            let fill_color = crate::core::Color {
                a: opacity,
                ..base_color
            };
            let stroke_color = crate::core::Color {
                a: (opacity + 0.2).min(1.0),
                ..base_color
            };

            let circle = Path::new(|builder| {
                trace_circle(builder, center.x, center.y, radius);
            });
            bubble_frame.fill(&circle, fill_color);
            bubble_frame.stroke(&circle, Stroke::default().with_color(stroke_color).with_width(1.0));
        }

        // ── Labels ─────────────────────────────────────────────────
        let mut label_frame = Frame::new(renderer, layout_bounds.size());

        let label_size = crate::core::Pixels(theme.font_size() * 0.75);

        for (i, pt) in self.data.points.iter().enumerate() {
            let label_text = match &pt.label {
                Some(l) => l.as_str(),
                None => continue,
            };

            let (center, radius) = state.bubble_circles[i];
            let label_color = text_pair.resolve(background, None);

            label_frame.fill_text(CanvasText {
                content: label_text.to_string(),
                position: crate::core::Point::new(center.x, center.y - radius - 3.0),
                color: label_color,
                size: label_size,
                font: theme.font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Bottom,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
        }

        // ── Composite ──────────────────────────────────────────────
        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);

        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(land_frame.into_geometry());
        });
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(bubble_frame.into_geometry());
        });
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(label_frame.into_geometry());
        });
    }
}

// ── Helpers ────────────────────────────────────────────────────────

/// Compute (min, max) of point values, clamping min to 0.
fn value_range(points: &[crate::mark::bubble_map::MapPoint]) -> (f32, f32) {
    let mut v_min = f32::INFINITY;
    let mut v_max = f32::NEG_INFINITY;
    for pt in points {
        let v = pt.value.abs() as f32;
        v_min = v_min.min(v);
        v_max = v_max.max(v);
    }
    if v_min.is_infinite() {
        v_min = 0.0;
    }
    if v_max.is_infinite() || v_max <= v_min {
        v_max = v_min + 1.0;
    }
    (v_min, v_max)
}

/// Map a value to a bubble radius using sqrt scaling (area-proportional).
fn interpolate_radius(value: f32, v_min: f32, v_max: f32, r_min: f32, r_max: f32) -> f32 {
    let v = value.abs();
    if v_max <= v_min {
        return (r_min + r_max) / 2.0;
    }
    let t = ((v - v_min) / (v_max - v_min)).clamp(0.0, 1.0);
    r_min + t.sqrt() * (r_max - r_min)
}

/// Trace a circle path using line segments.
fn trace_circle(builder: &mut crate::widget::canvas::path::Builder, cx: f32, cy: f32, radius: f32) {
    const SEGMENTS: usize = 32;
    let start = crate::core::Point::new(cx + radius, cy);
    builder.move_to(start);
    for i in 1..=SEGMENTS {
        let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
        builder.line_to(crate::core::Point::new(
            cx + radius * angle.cos(),
            cy + radius * angle.sin(),
        ));
    }
    builder.close();
}

use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

use super::Plane;

/// State for BubbleMap — stores projected bubble positions for hit-testing.
pub struct State {
    /// Pixel center and radius for each bubble (in local coordinates).
    pub bubble_circles: Vec<(crate::core::Point, f32)>,
}

/// A BubbleMap series that renders a world map with sized bubbles.
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
            state: tree::State::new(State {
                bubble_circles: Vec::new(),
            }),
            children: Vec::new(),
        }
    }

    pub(super) fn diff(&self, _tree: &mut Tree) {}

    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, limits: &Limits, _plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();
        let size = limits.max();

        // Compute projected positions for each point.
        state.bubble_circles = self
            .data
            .points
            .iter()
            .map(|pt| {
                let pos = project(pt.lat as f32, pt.lon as f32, size.width, size.height);
                let radius = 0.0; // computed during draw when value range is known
                (pos, radius)
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
        let layout_bounds = layout.bounds();
        let w = layout_bounds.width;
        let h = layout_bounds.height;

        // ── Land polygons ──────────────────────────────────────────
        let mut land_frame = Frame::new(renderer, layout_bounds.size());

        let land_fill = crate::core::Color {
            r: background.r * 0.92 + 0.08 * 0.7,
            g: background.g * 0.92 + 0.08 * 0.72,
            b: background.b * 0.92 + 0.08 * 0.74,
            a: 1.0,
        };
        let land_stroke_color = crate::core::Color {
            r: land_fill.r * 0.85,
            g: land_fill.g * 0.85,
            b: land_fill.b * 0.85,
            a: 0.6,
        };

        for polygon in crate::geo::world_polygons() {
            if polygon.len() < 3 {
                continue;
            }

            let path = Path::new(|builder| {
                let first = project(polygon[0].1, polygon[0].0, w, h);
                builder.move_to(first);
                for &(lon, lat) in &polygon[1..] {
                    builder.line_to(project(lat, lon, w, h));
                }
                builder.close();
            });

            land_frame.fill(&path, land_fill);
            land_frame.stroke(&path, Stroke::default().with_color(land_stroke_color).with_width(0.5));
        }

        // ── Bubbles ────────────────────────────────────────────────
        let mut bubble_frame = Frame::new(renderer, layout_bounds.size());

        let opacity = self.data.opacity;

        for (i, pt) in self.data.points.iter().enumerate() {
            let (center, radius) = state.bubble_circles[i];

            let base_color = if let Some(c) = pt.color {
                c.resolve(background, text_pair, None)
            } else {
                palette.get(color_offset + i).resolve(background, text_pair, None)
            };

            let fill_color = crate::core::Color {
                a: opacity,
                ..base_color
            };
            let stroke_color = crate::core::Color {
                a: (opacity + 0.2).min(1.0),
                ..base_color
            };

            // Filled circle
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

// ── Projection helpers ─────────────────────────────────────────────

/// Equirectangular projection: (lat, lon) → pixel (x, y).
fn project(lat: f32, lon: f32, width: f32, height: f32) -> crate::core::Point {
    // Slight padding so polygons don't touch edges
    let padding = 0.02;
    let usable_w = width * (1.0 - 2.0 * padding);
    let usable_h = height * (1.0 - 2.0 * padding);
    let x_off = width * padding;
    let y_off = height * padding;

    let x = x_off + ((lon + 180.0) / 360.0) * usable_w;
    let y = y_off + ((90.0 - lat) / 180.0) * usable_h;
    crate::core::Point::new(x, y)
}

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

use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::widget::canvas::{Frame, Path, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Number of line segments per full circle for arc approximation.
const ARC_SEGMENTS_PER_TAU: usize = 64;

/// State for Gauge — stores computed arc parameters
pub struct State {
    /// Value angle in radians (relative to arc start)
    pub value_angle: f32,
    /// Total sweep in radians
    pub sweep_rad: f32,
}

/// A Gauge series that renders gauge/meter charts.
pub struct Gauge<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::gauge::Gauge,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Gauge<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Gauge borrowing data
    pub fn new(data: &'a crate::mark::gauge::Gauge) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Gauge
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                value_angle: 0.0,
                sweep_rad: 0.0,
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Gauge state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the gauge — compute angles
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        _plane: &Plane,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();

        let range = self.data.max - self.data.min;
        let proportion = if range > 0.0 {
            ((self.data.value - self.data.min) / range).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        state.sweep_rad = self.data.sweep.to_radians();
        state.value_angle = proportion * state.sweep_rad;

        Node::new(Size::ZERO)
    }

    /// Draws the gauge chart
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
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let palette = theme.data_colors();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let cx = layout_bounds.width / 2.0;
        let cy = layout_bounds.height / 2.0;
        let radius = layout_bounds.width.min(layout_bounds.height) / 2.0 * 0.85;
        let thickness = radius * self.data.thickness;
        let inner_radius = radius - thickness;

        // Arc starts at bottom-left and sweeps clockwise
        // For a 270° gauge, gap is at the bottom
        let gap_rad = std::f32::consts::TAU - state.sweep_rad;
        let start_angle = std::f32::consts::FRAC_PI_2 + gap_rad / 2.0;

        // Draw background track arc — use text color at low opacity
        let track_color = crate::core::Color {
            a: 0.12,
            ..text_pair.on_light
        };

        draw_arc_segment(
            &mut frame,
            cx,
            cy,
            inner_radius,
            radius,
            start_angle,
            start_angle + state.sweep_rad,
            track_color,
        );

        // Draw zone arcs if configured
        let range = self.data.max - self.data.min;
        if range > 0.0 {
            for zone in &self.data.zones {
                let zone_start_prop = ((zone.from - self.data.min) / range)
                    .clamp(0.0, 1.0)
                    as f32;
                let zone_end_prop =
                    ((zone.to - self.data.min) / range).clamp(0.0, 1.0) as f32;

                let zone_start =
                    start_angle + zone_start_prop * state.sweep_rad;
                let zone_end = start_angle + zone_end_prop * state.sweep_rad;

                let zone_color =
                    zone.color.resolve(background, text_pair, None);

                draw_arc_segment(
                    &mut frame,
                    cx,
                    cy,
                    inner_radius,
                    radius,
                    zone_start,
                    zone_end,
                    zone_color,
                );
            }
        }

        // Draw value arc
        let value_color = palette
            .first()
            .map(|c| c.resolve(background, text_pair, None))
            .unwrap_or(background);

        if state.value_angle > 0.001 {
            draw_arc_segment(
                &mut frame,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                start_angle + state.value_angle,
                value_color,
            );
        }

        // Draw center value text
        if self.data.show_value {
            let value_text = if let Some(fmt) = &self.data.format {
                (fmt)(self.data.value)
            } else {
                format!("{}", self.data.value)
            };
            let text_color = text_pair.on_light;
            let font_size = radius * 0.35;

            frame.fill_text(CanvasText {
                content: value_text,
                position: crate::core::Point::new(cx, cy),
                color: text_color,
                size: font_size.into(),
                font: theme.font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Center,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });

            // Draw unit label below value
            if let Some(unit) = &self.data.unit {
                let unit_size = font_size * 0.5;
                frame.fill_text(CanvasText {
                    content: unit.clone(),
                    position: crate::core::Point::new(cx, cy + font_size * 0.5),
                    color: crate::core::Color {
                        a: 0.6,
                        ..text_color
                    },
                    size: unit_size.into(),
                    font: theme.font(),
                    align_x: crate::core::alignment::Horizontal::Center.into(),
                    align_y: crate::core::alignment::Vertical::Center,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }

        // Draw min/max labels at arc ends
        if self.data.show_min_max {
            let label_size = theme.font_size();
            let label_color = crate::core::Color {
                a: 0.5,
                ..text_pair.on_light
            };
            let label_r = radius + label_size * 0.8;

            let format_val = |v: f64| {
                if let Some(fmt) = &self.data.format {
                    (fmt)(v)
                } else {
                    format!("{}", v)
                }
            };

            // Min label at start of arc
            let min_pos = crate::core::Point::new(
                cx + label_r * start_angle.cos(),
                cy + label_r * start_angle.sin(),
            );
            frame.fill_text(CanvasText {
                content: format_val(self.data.min),
                position: min_pos,
                color: label_color,
                size: label_size.into(),
                font: theme.font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Center,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });

            // Max label at end of arc
            let end_angle = start_angle + state.sweep_rad;
            let max_pos = crate::core::Point::new(
                cx + label_r * end_angle.cos(),
                cy + label_r * end_angle.sin(),
            );
            frame.fill_text(CanvasText {
                content: format_val(self.data.max),
                position: max_pos,
                color: label_color,
                size: label_size.into(),
                font: theme.font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Center,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(
            crate::core::Vector::new(layout_bounds.x, layout_bounds.y),
            |renderer| {
                renderer.draw_geometry(geometry);
            },
        );
    }
}

/// Draw a filled arc segment (wedge between inner and outer radius) using line segments.
#[allow(clippy::too_many_arguments)]
fn draw_arc_segment<R: geometry::Renderer>(
    frame: &mut Frame<R>,
    cx: f32,
    cy: f32,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    end_angle: f32,
    color: crate::core::Color,
) {
    let path = Path::new(|builder| {
        // Start at inner radius
        let inner_start = crate::core::Point::new(
            cx + inner_radius * start_angle.cos(),
            cy + inner_radius * start_angle.sin(),
        );
        let outer_start = crate::core::Point::new(
            cx + outer_radius * start_angle.cos(),
            cy + outer_radius * start_angle.sin(),
        );

        builder.move_to(inner_start);
        builder.line_to(outer_start);

        // Trace outer arc forward
        trace_arc(builder, cx, cy, outer_radius, start_angle, end_angle);

        // Line from outer end to inner end
        let inner_end = crate::core::Point::new(
            cx + inner_radius * end_angle.cos(),
            cy + inner_radius * end_angle.sin(),
        );
        builder.line_to(inner_end);

        // Trace inner arc backward
        trace_arc(builder, cx, cy, inner_radius, end_angle, start_angle);

        builder.close();
    });

    frame.fill(&path, color);
}

/// Trace an arc using line segments.
fn trace_arc(
    builder: &mut crate::widget::canvas::path::Builder,
    cx: f32,
    cy: f32,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
) {
    let sweep = end_angle - start_angle;
    let segments = ((sweep.abs() / std::f32::consts::TAU)
        * ARC_SEGMENTS_PER_TAU as f32)
        .ceil() as usize;
    let segments = segments.max(1);

    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let angle = start_angle + sweep * t;
        builder.line_to(crate::core::Point::new(
            cx + radius * angle.cos(),
            cy + radius * angle.sin(),
        ));
    }
}

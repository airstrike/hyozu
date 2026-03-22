use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::data::axis::tick;
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
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, _plane: &Plane) -> Node {
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
        color_offset: usize,
        palette: &crate::palette::Resolved,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        let background = theme.background_color();
        let text_pair = theme.text_pair();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let cx = layout_bounds.width / 2.0;
        let cy = layout_bounds.height / 2.0;

        // Reserve space for tick labels outside the arc
        let half = layout_bounds.width.min(layout_bounds.height) / 2.0;
        let tick_margin = if self.data.ticks.is_some() && self.data.show_tick_labels {
            // Estimate: mark_length + scaled label size
            let mark_len = self.data.ticks.as_ref().map_or(6.0, |t| t.mark_length);
            mark_len + half * 0.09
        } else if self.data.ticks.is_some() {
            self.data.ticks.as_ref().map_or(6.0, |t| t.mark_length)
        } else {
            0.0
        };
        let radius = (half - tick_margin).max(half * 0.5);
        let thickness = radius * self.data.thickness;
        let inner_radius = radius - thickness;

        // Arc starts at bottom-left and sweeps clockwise
        // For a 270° gauge, gap is at the bottom
        let gap_rad = std::f32::consts::TAU - state.sweep_rad;
        let start_angle = std::f32::consts::FRAC_PI_2 + gap_rad / 2.0;

        // Tick label font size scales with radius
        let tick_font_size = radius * 0.1;

        let range = self.data.max - self.data.min;
        let has_zones = !self.data.zones.is_empty() && range > 0.0;

        // --- Background track ---
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

        // --- Arc rendering ---
        if has_zones && self.data.gradient {
            // Gradient mode with zones: interpolate colors across mini-segments
            let stops = build_zone_stops(self.data, range, background, text_pair);
            draw_gradient_arc(
                &mut frame,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                state.sweep_rad,
                &stops,
                self.data.color_stops,
            );
            // Dimming overlay on unfilled portion
            draw_dimming_overlay(
                &mut frame,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                state.value_angle,
                state.sweep_rad,
                background,
                self.data.dim_opacity,
            );
        } else if has_zones {
            // Zone mode: draw all zones at full opacity, then dim unfilled portion
            for zone in &self.data.zones {
                let zone_start_prop = ((zone.from - self.data.min) / range).clamp(0.0, 1.0) as f32;
                let zone_end_prop = ((zone.to - self.data.min) / range).clamp(0.0, 1.0) as f32;

                let zone_start = start_angle + zone_start_prop * state.sweep_rad;
                let zone_end = start_angle + zone_end_prop * state.sweep_rad;
                let zone_color = zone.color.resolve(background, text_pair, None);

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
            // Dimming overlay on unfilled portion
            draw_dimming_overlay(
                &mut frame,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                state.value_angle,
                state.sweep_rad,
                background,
                self.data.dim_opacity,
            );
        } else if self.data.gradient {
            // Gradient mode without zones: interpolate from palette color to desaturated
            let base = palette.get(color_offset).resolve(background, text_pair, None);
            let light = crate::core::Color {
                r: base.r * 0.4 + 0.6,
                g: base.g * 0.4 + 0.6,
                b: base.b * 0.4 + 0.6,
                a: base.a,
            };
            let stops = vec![(0.0_f32, base), (1.0, light)];
            draw_gradient_arc(
                &mut frame,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                state.sweep_rad,
                &stops,
                self.data.color_stops,
            );
            draw_dimming_overlay(
                &mut frame,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                state.value_angle,
                state.sweep_rad,
                background,
                self.data.dim_opacity,
            );
        } else {
            // Simple value arc (original behavior)
            let value_color = palette.get(color_offset).resolve(background, text_pair, None);
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
        }

        // --- Tick marks ---
        if let Some(ticks) = &self.data.ticks {
            draw_ticks(
                &mut frame,
                self.data,
                ticks,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                state.sweep_rad,
                range,
                text_pair,
                theme,
                tick_font_size,
            );
        }

        // --- Needle ---
        if self.data.show_needle {
            draw_needle(
                &mut frame,
                self.data,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                state.value_angle,
                text_pair,
                background,
            );
        }

        // --- Center value text ---
        let arc_top = cy - inner_radius;
        let arc_bottom = cy + inner_radius * (gap_rad / 2.0).cos();
        let text_cy = if self.data.show_needle {
            // When needle is present, position text well below the pivot
            cy + inner_radius * 0.35
        } else {
            // Center text in the arc bowl
            (arc_top + arc_bottom) / 2.0
        };

        let text_color = text_pair.on_light;
        let font_size = radius * 0.35;
        let mut label_bottom = text_cy;

        if self.data.show_value {
            let value_text = if let Some(fmt) = &self.data.format {
                (fmt)(self.data.value)
            } else {
                format!("{}", self.data.value)
            };

            frame.fill_text(CanvasText {
                content: value_text,
                position: crate::core::Point::new(cx, text_cy),
                color: text_color,
                size: font_size.into(),
                font: theme.font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Center,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });

            label_bottom = text_cy + font_size * 0.5 + self.data.label_spacing;

            // Draw unit label below value
            if let Some(unit) = &self.data.unit {
                let unit_size = font_size * 0.5;
                frame.fill_text(CanvasText {
                    content: unit.clone(),
                    position: crate::core::Point::new(cx, label_bottom),
                    color: crate::core::Color { a: 0.6, ..text_color },
                    size: unit_size.into(),
                    font: theme.font(),
                    align_x: crate::core::alignment::Horizontal::Center.into(),
                    align_y: crate::core::alignment::Vertical::Center,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });

                label_bottom += unit_size * 0.5 + self.data.label_spacing;
            }
        }

        // --- Subtitle ---
        if let Some(subtitle) = &self.data.subtitle {
            let sub_size = font_size * 0.3;
            frame.fill_text(CanvasText {
                content: subtitle.clone(),
                position: crate::core::Point::new(cx, label_bottom),
                color: crate::core::Color { a: 0.5, ..text_color },
                size: sub_size.into(),
                font: theme.font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Center,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }
}

// ---------------------------------------------------------------------------
// Helper: needle
// ---------------------------------------------------------------------------

/// Draws a needle indicator: a line from the center to the inner arc edge,
/// and a hollow pivot circle at the center.
#[allow(clippy::too_many_arguments)]
fn draw_needle<R: geometry::Renderer>(
    frame: &mut Frame<R>,
    data: &crate::mark::gauge::Gauge,
    cx: f32,
    cy: f32,
    inner_radius: f32,
    radius: f32,
    start_angle: f32,
    value_angle: f32,
    text_pair: crate::color::Pair,
    background: crate::core::Color,
) {
    let angle = start_angle + value_angle;
    let needle_len = inner_radius * data.needle_length;

    let color = if let Some(ref c) = data.needle_color {
        c.resolve(background, text_pair, None)
    } else {
        text_pair.on_light
    };

    // Needle line with rounded cap
    let tip = crate::core::Point::new(cx + needle_len * angle.cos(), cy + needle_len * angle.sin());
    let path = Path::new(|builder| {
        builder.move_to(crate::core::Point::new(cx, cy));
        builder.line_to(tip);
    });
    frame.stroke(&path, crate::widget::canvas::Stroke {
        width: data.needle_width,
        style: crate::widget::canvas::Style::Solid(color),
        line_cap: crate::widget::canvas::LineCap::Round,
        ..Default::default()
    });

    // Hollow pivot circle
    let pivot_r = radius * data.pivot_radius;
    if pivot_r > 0.5 {
        let center = crate::core::Point::new(cx, cy);
        let pivot = Path::circle(center, pivot_r);
        frame.fill(&pivot, background);
        frame.stroke(&pivot, crate::widget::canvas::Stroke {
            width: 1.5,
            style: crate::widget::canvas::Style::Solid(color),
            ..Default::default()
        });
    }

    // Optional filled tip dot
    if data.show_needle_tip {
        let tip_r = data.needle_width * 1.5;
        let tip_circle = Path::circle(tip, tip_r);
        frame.fill(&tip_circle, color);
    }
}

// ---------------------------------------------------------------------------
// Helper: dimming overlay
// ---------------------------------------------------------------------------

/// Draws a semi-transparent overlay from value_angle to the end of the sweep,
/// muting the unfilled portion of the arc so zones behind it appear dimmed.
#[allow(clippy::too_many_arguments)]
fn draw_dimming_overlay<R: geometry::Renderer>(
    frame: &mut Frame<R>,
    cx: f32,
    cy: f32,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    value_angle: f32,
    sweep_rad: f32,
    background: crate::core::Color,
    dim_opacity: f32,
) {
    let unfilled = sweep_rad - value_angle;
    if unfilled < 0.001 {
        return;
    }
    let dim_color = crate::core::Color {
        a: dim_opacity,
        ..background
    };
    draw_arc_segment(
        frame,
        cx,
        cy,
        inner_radius,
        outer_radius,
        start_angle + value_angle,
        start_angle + sweep_rad,
        dim_color,
    );
}

// ---------------------------------------------------------------------------
// Helper: gradient arc
// ---------------------------------------------------------------------------

/// Color stops: (proportion 0..1, color)
type ColorStops = Vec<(f32, crate::core::Color)>;

/// Build color stops from zone definitions.
///
/// Places one stop per zone at its midpoint so each zone's color dominates
/// at the center and transitions happen naturally between zones.
/// Extends to 0.0 and 1.0 using the first/last zone colors.
fn build_zone_stops(
    data: &crate::mark::gauge::Gauge,
    range: f64,
    background: crate::core::Color,
    text_pair: crate::color::Pair,
) -> ColorStops {
    let mut stops: ColorStops = Vec::new();
    for zone in &data.zones {
        let start_prop = ((zone.from - data.min) / range).clamp(0.0, 1.0) as f32;
        let end_prop = ((zone.to - data.min) / range).clamp(0.0, 1.0) as f32;
        let mid = (start_prop + end_prop) / 2.0;
        let color = zone.color.resolve(background, text_pair, None);
        stops.push((mid, color));
    }
    // Extend to arc edges using first/last zone colors
    if let Some(&(_, first_color)) = stops.first()
        && stops[0].0 > 0.001
    {
        stops.insert(0, (0.0, first_color));
    }
    if let Some(&(_, last_color)) = stops.last()
        && stops.last().unwrap().0 < 0.999
    {
        stops.push((1.0, last_color));
    }
    stops
}

/// Interpolate a color at the given proportion from sorted color stops (OKLCh).
fn interpolate_color(stops: &ColorStops, t: f32) -> crate::core::Color {
    use crate::palette::{Oklch, from_oklch, to_oklch};

    if stops.is_empty() {
        return crate::core::Color::WHITE;
    }
    if t <= stops[0].0 {
        return stops[0].1;
    }
    if t >= stops[stops.len() - 1].0 {
        return stops[stops.len() - 1].1;
    }
    for window in stops.windows(2) {
        let (t0, c0) = window[0];
        let (t1, c1) = window[1];
        if t >= t0 && t <= t1 {
            let local = if (t1 - t0).abs() < 0.0001 {
                0.0
            } else {
                (t - t0) / (t1 - t0)
            };
            let a = to_oklch(c0);
            let b = to_oklch(c1);

            // Shortest-path hue interpolation
            let mut dh = b.h - a.h;
            if dh > std::f32::consts::PI {
                dh -= std::f32::consts::TAU;
            } else if dh < -std::f32::consts::PI {
                dh += std::f32::consts::TAU;
            }

            return from_oklch(Oklch {
                l: a.l + (b.l - a.l) * local,
                c: a.c + (b.c - a.c) * local,
                h: a.h + dh * local,
                a: a.a + (b.a - a.a) * local,
            });
        }
    }
    stops[stops.len() - 1].1
}

/// Draw a gradient arc by breaking it into many small segments with interpolated colors.
#[allow(clippy::too_many_arguments)]
fn draw_gradient_arc<R: geometry::Renderer>(
    frame: &mut Frame<R>,
    cx: f32,
    cy: f32,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    sweep_rad: f32,
    stops: &ColorStops,
    segments: usize,
) {
    for i in 0..segments {
        let t0 = i as f32 / segments as f32;
        let t1 = (i + 1) as f32 / segments as f32;
        let mid_t = (t0 + t1) / 2.0;
        let color = interpolate_color(stops, mid_t);

        let a0 = start_angle + t0 * sweep_rad;
        let a1 = start_angle + t1 * sweep_rad;

        draw_arc_segment(frame, cx, cy, inner_radius, outer_radius, a0, a1, color);
    }
}

// ---------------------------------------------------------------------------
// Helper: tick marks
// ---------------------------------------------------------------------------

/// Compute tick positions as data values.
fn compute_tick_values(data: &crate::mark::gauge::Gauge, ticks: &tick::Ticks, range: f64) -> Vec<f64> {
    match &ticks.frequency {
        tick::Frequency::Custom(vals) => vals.clone(),
        tick::Frequency::FirstAndLast => vec![data.min, data.max],
        tick::Frequency::EveryNthItem(n) => {
            let step = *n as f64;
            let mut vals = Vec::new();
            let mut v = data.min;
            while v <= data.max + 1e-9 {
                vals.push(v);
                v += step;
            }
            vals
        }
        tick::Frequency::EveryItem => {
            // Auto nice ticks: ~5 ticks
            let raw_step = range / 5.0;
            let magnitude = 10.0_f64.powf(raw_step.log10().floor());
            let residual = raw_step / magnitude;
            let nice_step = if residual <= 1.5 {
                magnitude
            } else if residual <= 3.5 {
                2.0 * magnitude
            } else if residual <= 7.5 {
                5.0 * magnitude
            } else {
                10.0 * magnitude
            };

            let mut vals = Vec::new();
            let start = (data.min / nice_step).ceil() * nice_step;
            let mut v = start;
            while v <= data.max + 1e-9 {
                vals.push(v);
                v += nice_step;
            }
            vals
        }
    }
}

/// Format a tick value compactly.
fn format_tick_value(v: f64) -> String {
    if v == v.floor() {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}

/// Draw tick marks and optional labels on the gauge arc.
#[allow(clippy::too_many_arguments)]
fn draw_ticks<R, Theme>(
    frame: &mut Frame<R>,
    data: &crate::mark::gauge::Gauge,
    ticks: &tick::Ticks,
    cx: f32,
    cy: f32,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    sweep_rad: f32,
    range: f64,
    text_pair: crate::color::Pair,
    theme: &Theme,
    tick_font_size: f32,
) where
    R: geometry::Renderer,
    Theme: crate::design::Design + ?Sized,
{
    let tick_values = compute_tick_values(data, ticks, range);
    let mark_len = ticks.mark_length;

    let tick_color = crate::core::Color {
        a: 0.40,
        ..text_pair.on_light
    };

    for v in &tick_values {
        let proportion = ((v - data.min) / range).clamp(0.0, 1.0) as f32;
        let angle = start_angle + proportion * sweep_rad;

        // Compute radial line endpoints based on tick style
        let (r_start, r_end) = match ticks.style {
            tick::Style::Outset => (outer_radius, outer_radius + mark_len),
            tick::Style::Inset => (inner_radius - mark_len, inner_radius),
            tick::Style::Cross => (inner_radius - mark_len / 2.0, outer_radius + mark_len / 2.0),
            tick::Style::None => continue,
        };

        let p0 = crate::core::Point::new(cx + r_start * angle.cos(), cy + r_start * angle.sin());
        let p1 = crate::core::Point::new(cx + r_end * angle.cos(), cy + r_end * angle.sin());

        let path = Path::new(|builder| {
            builder.move_to(p0);
            builder.line_to(p1);
        });

        frame.stroke(&path, crate::widget::canvas::Stroke {
            width: 1.5,
            style: crate::widget::canvas::Style::Solid(tick_color),
            ..Default::default()
        });

        // Tick label
        if data.show_tick_labels {
            let label_r = match ticks.style {
                tick::Style::Outset | tick::Style::Cross => outer_radius + mark_len + tick_font_size * 0.6,
                tick::Style::Inset => inner_radius - mark_len - tick_font_size * 0.6,
                tick::Style::None => continue,
            };
            let label_pos = crate::core::Point::new(cx + label_r * angle.cos(), cy + label_r * angle.sin());

            frame.fill_text(CanvasText {
                content: format_tick_value(*v),
                position: label_pos,
                color: crate::core::Color {
                    a: 0.5,
                    ..text_pair.on_light
                },
                size: tick_font_size.into(),
                font: theme.font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Center,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Core arc drawing primitives
// ---------------------------------------------------------------------------

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
        let inner_end =
            crate::core::Point::new(cx + inner_radius * end_angle.cos(), cy + inner_radius * end_angle.sin());
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
    let segments = ((sweep.abs() / std::f32::consts::TAU) * ARC_SEGMENTS_PER_TAU as f32).ceil() as usize;
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

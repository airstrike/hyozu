use super::Plane;
use crate::animation;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::data::axis::tick;
use crate::widget::canvas::{Frame, Path, Text};

use crate::core::text;
use crate::widget::renderer::geometry;

/// State for Gauge — stores computed arc parameters
pub struct State {
    /// Value angle in radians (relative to arc start)
    pub value_angle: f32,
    /// Total sweep in radians
    pub sweep_rad: f32,
    /// Value angle from the most recent layout before the current one,
    /// captured by [`crate::chart::Chart::diff`] when data changes so the
    /// next sweep can interpolate from the previous angle to the current
    /// one. `0.0` on a fresh mount, in which case the animation collapses
    /// to a `0 → value_angle` sweep.
    pub previous_value_angle: f32,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// A Gauge series that renders gauge/meter charts.
pub struct Gauge<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::gauge::Gauge,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
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
            animate: true,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets whether the mount/data-change sweep should run. Wired by
    /// [`super::PlotArea::with_animate`] from [`crate::Data::animate`].
    pub(crate) fn set_animate(&mut self, animate: bool) {
        self.animate = animate;
    }

    /// Returns the initial tree state for this Gauge
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                value_angle: 0.0,
                sweep_rad: 0.0,
                previous_value_angle: 0.0,
                tick: animation::Tick::new(),
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
        // Treat non-finite value/min/range as a 0 proportion so a stray NaN
        // doesn't propagate into the value angle and panic the tessellator.
        let proportion = if range > 0.0 && self.data.value.is_finite() && self.data.min.is_finite() {
            ((self.data.value - self.data.min) / range).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };

        let sweep_rad = self.data.sweep.to_radians();
        state.sweep_rad = if sweep_rad.is_finite() { sweep_rad } else { 0.0 };
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
        corners: crate::core::border::Radius,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

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

        // Gauge rounding is binary: any positive corner becomes a perfect
        // semicircular cap (radius = half the ring thickness), so the tip
        // reads the same at every gauge size. `0` stays square. All arc
        // layers (track, value, zones, gradient, dimming) use this one
        // value, so their caps align instead of leaving slivers.
        let corner = if corners.top_left > 0.0 { thickness * 0.5 } else { 0.0 };

        // Arc starts at bottom-left and sweeps clockwise
        // For a 270° gauge, gap is at the bottom
        let gap_rad = std::f32::consts::TAU - state.sweep_rad;
        let start_angle = std::f32::consts::FRAC_PI_2 + gap_rad / 2.0;

        // Tick label font size scales with radius
        let tick_font_size = radius * 0.1;

        let range = self.data.max - self.data.min;
        let has_zones = !self.data.zones.is_empty() && range > 0.0;

        // Sweep: the visible value angle interpolates from the previous
        // layout's value angle toward the current one. With no previous
        // value (fresh mount) `previous_value_angle` is `0.0`, so the
        // expression collapses to a `0 → value_angle` mount sweep. When
        // animation is opted out, progress pins to `1.0` and
        // `animated_value_angle` equals `state.value_angle` exactly.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animating = progress < 1.0 - f32::EPSILON;
        let animated_value_angle =
            state.previous_value_angle + (state.value_angle - state.previous_value_angle) * progress;

        // Background track
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
            corner,
            corner,
        );

        // Arc rendering
        if has_zones && self.data.gradient {
            // Gradient mode with zones: interpolate colors across mini-segments
            let stops = build_zone_stops(self.data, range, background, text_pair, &seed);
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
                corner,
            );
            // Dimming overlay on unfilled portion
            draw_dimming_overlay(
                &mut frame,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                animated_value_angle,
                state.sweep_rad,
                background,
                self.data.dim_opacity,
                corner,
            );
        } else if has_zones {
            // Zone mode: draw all zones at full opacity, then dim unfilled
            // portion. Only the outermost ends round (first zone's leading
            // cap, last zone's trailing cap) so adjacent zones stay flush.
            let last_zone = self.data.zones.len().saturating_sub(1);
            for (zi, zone) in self.data.zones.iter().enumerate() {
                let zone_start_prop = ((zone.from - self.data.min) / range).clamp(0.0, 1.0) as f32;
                let zone_end_prop = ((zone.to - self.data.min) / range).clamp(0.0, 1.0) as f32;

                let zone_start = start_angle + zone_start_prop * state.sweep_rad;
                let zone_end = start_angle + zone_end_prop * state.sweep_rad;
                let zone_color = zone.color.resolve(background, text_pair, &seed, None);

                draw_arc_segment(
                    &mut frame,
                    cx,
                    cy,
                    inner_radius,
                    radius,
                    zone_start,
                    zone_end,
                    zone_color,
                    if zi == 0 { corner } else { 0.0 },
                    if zi == last_zone { corner } else { 0.0 },
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
                animated_value_angle,
                state.sweep_rad,
                background,
                self.data.dim_opacity,
                corner,
            );
        } else if self.data.gradient {
            // Gradient mode without zones: interpolate from palette color to desaturated
            let base = palette.get(color_offset).resolve(background, text_pair, &seed, None);
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
                corner,
            );
            draw_dimming_overlay(
                &mut frame,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                animated_value_angle,
                state.sweep_rad,
                background,
                self.data.dim_opacity,
                corner,
            );
        } else {
            // Simple value arc (original behavior)
            let value_color = palette.get(color_offset).resolve(background, text_pair, &seed, None);
            if animated_value_angle > 0.001 {
                draw_arc_segment(
                    &mut frame,
                    cx,
                    cy,
                    inner_radius,
                    radius,
                    start_angle,
                    start_angle + animated_value_angle,
                    value_color,
                    corner,
                    corner,
                );
            }
        }

        // Tick marks and labels — suppressed mid-sweep so labels don't
        // pop in over arc segments that haven't grown into their final
        // positions yet.
        if let Some(ticks) = &self.data.ticks
            && !animating
        {
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
                animated_value_angle,
                text_pair,
                &seed,
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

        // Center value, unit, and subtitle — suppressed mid-sweep so
        // text doesn't pop in before the arc reaches its final position.
        if self.data.show_value && !animating {
            let value_text = if let Some(fmt) = &self.data.format {
                (fmt)(self.data.value)
            } else {
                format!("{}", self.data.value)
            };

            frame.fill_text(Text {
                content: value_text,
                position: crate::core::Point::new(cx, text_cy),
                color: text_color,
                size: font_size.into(),
                font: theme.font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Center,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..Text::default()
            });

            label_bottom = text_cy + font_size * 0.5 + self.data.label_spacing;

            // Draw unit label below value
            if let Some(unit) = &self.data.unit {
                let unit_size = font_size * 0.5;
                frame.fill_text(Text {
                    content: unit.clone(),
                    position: crate::core::Point::new(cx, label_bottom),
                    color: crate::core::Color { a: 0.6, ..text_color },
                    size: unit_size.into(),
                    font: theme.font(),
                    align_x: crate::core::alignment::Horizontal::Center.into(),
                    align_y: crate::core::alignment::Vertical::Center,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..Text::default()
                });

                label_bottom += unit_size * 0.5 + self.data.label_spacing;
            }
        }

        // --- Subtitle ---
        if let Some(subtitle) = &self.data.subtitle
            && !animating
        {
            let sub_size = font_size * 0.3;
            frame.fill_text(Text {
                content: subtitle.clone(),
                position: crate::core::Point::new(cx, label_bottom),
                color: crate::core::Color { a: 0.5, ..text_color },
                size: sub_size.into(),
                font: theme.font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Center,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..Text::default()
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
    seed: &crate::palette::Seed,
    background: crate::core::Color,
) {
    let angle = start_angle + value_angle;
    let needle_len = inner_radius * data.needle_length;

    let color = if let Some(ref c) = data.needle_color {
        c.resolve(background, text_pair, seed, None)
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
    corner: f32,
) {
    let unfilled = sweep_rad - value_angle;
    if unfilled < 0.001 {
        return;
    }
    let dim_color = crate::core::Color {
        a: dim_opacity,
        ..background
    };
    // Round only the arc terminal (`a1`) to match the track/zone end; the
    // value boundary stays flat.
    draw_arc_segment(
        frame,
        cx,
        cy,
        inner_radius,
        outer_radius,
        start_angle + value_angle,
        start_angle + sweep_rad,
        dim_color,
        0.0,
        corner,
    );
}

/// Color stops: (proportion 0..1, color)
type Stops = Vec<(f32, crate::core::Color)>;

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
    seed: &crate::palette::Seed,
) -> Stops {
    let mut stops: Stops = Vec::new();
    for zone in &data.zones {
        let start_prop = ((zone.from - data.min) / range).clamp(0.0, 1.0) as f32;
        let end_prop = ((zone.to - data.min) / range).clamp(0.0, 1.0) as f32;
        let mid = (start_prop + end_prop) / 2.0;
        let color = zone.color.resolve(background, text_pair, seed, None);
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
fn interpolate_color(stops: &Stops, t: f32) -> crate::core::Color {
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
    stops: &Stops,
    segments: usize,
    corner: f32,
) {
    if segments == 0 {
        return;
    }
    let sd = sweep_rad.signum();
    let abs_sweep = sweep_rad.abs();

    // Reserve a cap-sized wedge at each end: a single 1/segments mini-slice
    // is too narrow to host a half-thickness fillet, so the cap is split
    // off as its own segment whose angular width is exactly the fillet
    // inset. That makes the rounded tip align with the track and dimming
    // overlay (which round to the same radius) instead of poking through.
    let cap = corner.max(0.0).min((outer_radius - inner_radius) * 0.5);
    let cap_angle = if cap > 0.0 && outer_radius > cap {
        (cap / (outer_radius - cap)).asin().min(abs_sweep * 0.5)
    } else {
        0.0
    };

    // Draws one mini-segment, sampling the gradient color at its midpoint.
    let seg = |frame: &mut Frame<R>, a0: f32, a1: f32, c0: f32, c1: f32| {
        let mid_prop = (((a0 + a1) * 0.5 - start_angle) / sweep_rad).clamp(0.0, 1.0);
        let color = interpolate_color(stops, mid_prop);
        draw_arc_segment(frame, cx, cy, inner_radius, outer_radius, a0, a1, color, c0, c1);
    };

    if cap_angle <= 0.0 || abs_sweep <= 2.0 * cap_angle {
        // No rounding, or no room to split caps off: round only the first
        // segment's start and the last segment's end, rest square.
        for i in 0..segments {
            let a0 = start_angle + (i as f32 / segments as f32) * sweep_rad;
            let a1 = start_angle + ((i + 1) as f32 / segments as f32) * sweep_rad;
            seg(
                frame,
                a0,
                a1,
                if i == 0 { corner } else { 0.0 },
                if i + 1 == segments { corner } else { 0.0 },
            );
        }
        return;
    }

    // Rounded start cap, interpolated middle, rounded end cap.
    let mid_start = start_angle + sd * cap_angle;
    let mid_end = start_angle + sweep_rad - sd * cap_angle;
    let mid_sweep = mid_end - mid_start;
    seg(frame, start_angle, mid_start, corner, 0.0);
    for i in 0..segments {
        let a0 = mid_start + (i as f32 / segments as f32) * mid_sweep;
        let a1 = mid_start + ((i + 1) as f32 / segments as f32) * mid_sweep;
        seg(frame, a0, a1, 0.0, 0.0);
    }
    seg(frame, mid_end, start_angle + sweep_rad, 0.0, corner);
}

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

            frame.fill_text(Text {
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
                ..Text::default()
            });
        }
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
    corner_start: f32,
    corner_end: f32,
) {
    // Non-finite inputs (NaN value, NaN zone bound, ±∞ sweep) propagate
    // through cos/sin and panic the tessellator. Bail at the rendering
    // boundary so any caller is shielded.
    if !cx.is_finite()
        || !cy.is_finite()
        || !inner_radius.is_finite()
        || !outer_radius.is_finite()
        || !start_angle.is_finite()
        || !end_angle.is_finite()
    {
        return;
    }
    let path = Path::new(|builder| {
        super::sector::push_sector_path_ends(
            builder,
            cx,
            cy,
            inner_radius,
            outer_radius,
            start_angle,
            end_angle,
            corner_start,
            corner_end,
        );
    });

    frame.fill(&path, color);
}

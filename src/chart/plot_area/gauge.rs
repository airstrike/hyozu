use super::Domain;
use crate::animation;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::data::axis::tick;
use crate::widget::canvas::{Frame, Path};

use crate::core::text::{self, paragraph};
use crate::widget::renderer::geometry;

/// State for Gauge — stores computed arc parameters
pub struct State<P>
where
    P: text::Paragraph,
{
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
    /// Center value, unit, and subtitle label paragraphs plus the
    /// per-tick label paragraphs, shaped once in [`Gauge::layout`] and
    /// rendered in [`Gauge::draw`] via
    /// [`crate::core::text::Renderer::fill_paragraph`]. The arc radius (and
    /// hence every font size and tick label set) is derived from the same
    /// `limits.max()` the draw pass derives from `layout.bounds()`, which
    /// are equal for the polar gauge node, so the shaped paragraphs match
    /// the draw geometry. Hidden / absent elements keep a default (empty)
    /// paragraph; `draw` skips them with the same guards.
    pub labels: Labels<P>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// The shaped label paragraphs for a gauge: the three center texts and the
/// arc tick labels. Each is shaped in [`Gauge::layout`] and rendered in
/// [`Gauge::draw`]; absent ones stay default (empty) and are skipped.
pub struct Labels<P>
where
    P: text::Paragraph,
{
    /// Center value text (the big number).
    pub value: paragraph::Plain<P>,
    /// Unit label below the value.
    pub unit: paragraph::Plain<P>,
    /// Subtitle below the unit.
    pub subtitle: paragraph::Plain<P>,
    /// One paragraph per arc tick label, index-aligned with the tick
    /// values [`compute_tick_values`] produces.
    pub ticks: Vec<paragraph::Plain<P>>,
}

impl<P> Default for Labels<P>
where
    P: text::Paragraph,
{
    fn default() -> Self {
        Self {
            value: paragraph::Plain::default(),
            unit: paragraph::Plain::default(),
            subtitle: paragraph::Plain::default(),
            ticks: Vec::new(),
        }
    }
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
    Renderer: text::Renderer<Font = crate::core::Font> + geometry::Renderer,
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
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                value_angle: 0.0,
                sweep_rad: 0.0,
                previous_value_angle: 0.0,
                labels: Labels::default(),
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
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
        _domain: &Domain,
        _rect: crate::core::Rectangle,
        design: Option<&dyn crate::design::Design>,
    ) -> Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

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

        self.shape_labels(state, renderer, limits.max(), range, design);

        Node::new(Size::ZERO)
    }

    /// Shapes the gauge's center value/unit/subtitle and tick labels,
    /// theme-free (font from `renderer.default_font()`, sizes scaled from the
    /// arc radius). The radius is derived from `limits.max()` via
    /// [`arc_radius`], the same value `draw` derives from `layout.bounds()`
    /// (equal for the polar gauge node), so the shaped glyph sizes match the
    /// rendered geometry. Hidden / absent elements keep a default (empty)
    /// paragraph; `draw` skips them with the same `show_value` / `unit` /
    /// `subtitle` / `show_tick_labels` guards.
    fn shape_labels(
        &self,
        state: &mut State<Renderer::Paragraph>,
        renderer: &Renderer,
        size: Size,
        range: f64,
        design: Option<&dyn crate::design::Design>,
    ) {
        // The gauge's glyph sizes stay radius-proportional; the design only
        // supplies the font family/weight base (12 px / default font when the
        // chart has no design).
        let default_font = design
            .map(|d| d.data_label_text().resolved_font(renderer.default_font()))
            .unwrap_or_else(|| renderer.default_font());
        let hint_factor = renderer.scale_factor();

        let shape = |paragraph: &mut paragraph::Plain<Renderer::Paragraph>, content: &str, px: f32| {
            let _ = paragraph.update(text::Text {
                content,
                bounds: Size::INFINITE,
                size: crate::core::Pixels(px),
                line_height: text::LineHeight::default(),
                font: default_font,
                align_x: text::Alignment::Left,
                align_y: crate::core::alignment::Vertical::Top,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
                ellipsis: text::Ellipsis::default(),
                hint_factor,
                font_features: Vec::new(),
                font_variations: Vec::new(),
                letter_spacing: Default::default(),
                weight: None,
            });
        };

        let radius = arc_radius(self.data, size);
        let font_size = radius * 0.35;

        // Center value, unit, subtitle.
        if self.data.show_value {
            let value_text = if let Some(fmt) = &self.data.format {
                (fmt)(self.data.value)
            } else {
                format!("{}", self.data.value)
            };
            shape(&mut state.labels.value, &value_text, font_size);

            if let Some(unit) = &self.data.unit {
                shape(&mut state.labels.unit, unit, font_size * 0.5);
            }
        }
        if let Some(subtitle) = &self.data.subtitle {
            shape(&mut state.labels.subtitle, subtitle, font_size * 0.3);
        }

        // Tick labels — one paragraph per tick value, index-aligned with
        // `compute_tick_values`. Shaped whenever ticks + labels are enabled,
        // regardless of tick `style` (draw skips the `None` style per tick).
        let tick_font_size = radius * 0.1;
        if let Some(ticks) = &self.data.ticks
            && self.data.show_tick_labels
            && range > 0.0
        {
            let tick_values = compute_tick_values(self.data, ticks, range);
            while state.labels.ticks.len() < tick_values.len() {
                state.labels.ticks.push(paragraph::Plain::default());
            }
            state.labels.ticks.truncate(tick_values.len());
            for (i, v) in tick_values.iter().enumerate() {
                let content = format_tick_value(*v);
                shape(&mut state.labels.ticks[i], &content, tick_font_size);
            }
        } else {
            state.labels.ticks.clear();
        }
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
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

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
        // Tick-label render list: `(tick index, anchor, color)`. The marks
        // (lines) go into the geometry frame; the labels are collected here
        // (paired with the cached tick paragraphs by index) and drawn after
        // the frame via `fill_paragraph`.
        let mut tick_label_draws: Vec<(usize, crate::core::Point, crate::core::Color)> = Vec::new();
        if let Some(ticks) = &self.data.ticks
            && !animating
        {
            draw_ticks(
                &mut frame,
                self.data,
                ticks,
                &state.labels.ticks,
                cx,
                cy,
                inner_radius,
                radius,
                start_angle,
                state.sweep_rad,
                range,
                text_pair,
                &mut tick_label_draws,
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

        // Center value, unit, subtitle, and tick labels are drawn directly
        // via the cached paragraphs (`fill_paragraph`) after the geometry
        // frame, so the glyphs sit on top of the arc. Each entry pairs a
        // cached paragraph (`PieceText` discriminant) with its Center/Center
        // anchor and color. The `label_bottom` advancement matches the
        // previous `fill_text` layout exactly.
        let mut center_draws: Vec<(CenterPiece, crate::core::Point, crate::core::Color)> = Vec::new();

        // Computes the top-left anchor for a Center/Center-aligned paragraph
        // about `(px, py)` from the cached paragraph's `min_bounds`.
        let center_anchor = |paragraph: &paragraph::Plain<Renderer::Paragraph>, px: f32, py: f32| {
            let bounds = paragraph.min_bounds();
            crate::core::Point::new(px - bounds.width / 2.0, py - bounds.height / 2.0)
        };

        // Center value, unit, and subtitle — suppressed mid-sweep so
        // text doesn't pop in before the arc reaches its final position.
        if self.data.show_value && !animating {
            let anchor = center_anchor(&state.labels.value, cx, text_cy);
            center_draws.push((CenterPiece::Value, anchor, text_color));

            label_bottom = text_cy + font_size * 0.5 + self.data.label_spacing;

            // Unit label below value
            if self.data.unit.is_some() {
                let unit_size = font_size * 0.5;
                let anchor = center_anchor(&state.labels.unit, cx, label_bottom);
                center_draws.push((CenterPiece::Unit, anchor, crate::core::Color { a: 0.6, ..text_color }));

                label_bottom += unit_size * 0.5 + self.data.label_spacing;
            }
        }

        // --- Subtitle ---
        if self.data.subtitle.is_some() && !animating {
            let anchor = center_anchor(&state.labels.subtitle, cx, label_bottom);
            center_draws.push((CenterPiece::Subtitle, anchor, crate::core::Color {
                a: 0.5,
                ..text_color
            }));
        }

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);
        let geometry = frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(geometry);
        });

        // Tick labels on top of the arc.
        for (i, anchor, color) in tick_label_draws {
            if let Some(paragraph) = state.labels.ticks.get(i) {
                renderer.fill_paragraph(paragraph.raw(), anchor + translation, color, layout_bounds);
            }
        }

        // Center value / unit / subtitle on top.
        for (piece, anchor, color) in center_draws {
            let paragraph = match piece {
                CenterPiece::Value => &state.labels.value,
                CenterPiece::Unit => &state.labels.unit,
                CenterPiece::Subtitle => &state.labels.subtitle,
            };
            renderer.fill_paragraph(paragraph.raw(), anchor + translation, color, layout_bounds);
        }
    }
}

/// Discriminant for the three center-text paragraphs cached on
/// [`State::labels`], used to collect their render order in `draw` before
/// the geometry frame is consumed.
enum CenterPiece {
    Value,
    Unit,
    Subtitle,
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

/// The arc radius for a gauge of the given pixel `size`. The arc reserves a
/// margin outside itself for the tick marks/labels, then takes the largest
/// circle that fits, floored at half the available half-extent so a heavy
/// tick reservation can't collapse the arc. Shared by [`Gauge::layout`] (to
/// derive font sizes for label shaping) and [`Gauge::draw`] (to place the
/// geometry), so the shaped paragraphs match the rendered arc — the gauge's
/// `limits.max()` at layout equals `layout.bounds().size()` at draw.
fn arc_radius(data: &crate::mark::gauge::Gauge, size: Size) -> f32 {
    let half = size.width.min(size.height) / 2.0;
    let tick_margin = if data.ticks.is_some() && data.show_tick_labels {
        let mark_len = data.ticks.as_ref().map_or(6.0, |t| t.mark_length);
        mark_len + half * 0.09
    } else if data.ticks.is_some() {
        data.ticks.as_ref().map_or(6.0, |t| t.mark_length)
    } else {
        0.0
    };
    (half - tick_margin).max(half * 0.5)
}

/// Format a tick value compactly.
fn format_tick_value(v: f64) -> String {
    if v == v.floor() {
        format!("{}", v as i64)
    } else {
        format!("{:.1}", v)
    }
}

/// Draw tick mark lines into `frame` and collect the Center/Center anchors
/// for the (already-shaped) tick labels into `label_draws`, paired by tick
/// index with `tick_labels`. The glyphs themselves are drawn by the caller
/// via `fill_paragraph` after the frame is consumed, so the labels sit on
/// top of the arc.
#[allow(clippy::too_many_arguments)]
fn draw_ticks<R, P>(
    frame: &mut Frame<R>,
    data: &crate::mark::gauge::Gauge,
    ticks: &tick::Ticks,
    tick_labels: &[paragraph::Plain<P>],
    cx: f32,
    cy: f32,
    inner_radius: f32,
    outer_radius: f32,
    start_angle: f32,
    sweep_rad: f32,
    range: f64,
    text_pair: crate::color::Pair,
    label_draws: &mut Vec<(usize, crate::core::Point, crate::core::Color)>,
) where
    R: geometry::Renderer,
    P: text::Paragraph,
{
    let tick_values = compute_tick_values(data, ticks, range);
    let mark_len = ticks.mark_length;
    // Tick label font size scales with radius, matching [`Gauge::shape_labels`].
    let tick_font_size = outer_radius * 0.1;

    let tick_color = crate::core::Color {
        a: 0.40,
        ..text_pair.on_light
    };

    for (i, v) in tick_values.iter().enumerate() {
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

            // `fill_text` centered the glyph box on `label_pos`; shift by half
            // the cached paragraph's `min_bounds` so `fill_paragraph`'s
            // top-left origin reproduces Center/Center.
            if let Some(paragraph) = tick_labels.get(i) {
                let bounds = paragraph.min_bounds();
                let anchor =
                    crate::core::Point::new(label_pos.x - bounds.width / 2.0, label_pos.y - bounds.height / 2.0);
                let color = crate::core::Color {
                    a: 0.5,
                    ..text_pair.on_light
                };
                label_draws.push((i, anchor, color));
            }
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

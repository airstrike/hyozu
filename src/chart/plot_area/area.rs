use super::line::{
    alignment_for_position, compute_label_rect, dash_segments, draw_drop_lines, draw_markers, place_label, stroke_runs,
};
use super::{Domain, to_pixel};
use crate::animation;
use crate::core::layout::Limits;
use crate::core::text::{self, paragraph};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size};
use crate::data::Datum;
use crate::line::label::{Position, Show};
use crate::line::{Interpolation, LineStyle, Nulls};
use crate::widget::canvas::gradient::Linear;
use crate::widget::canvas::{Fill, Frame, LineCap, LineDash, Path, Stroke};
use crate::widget::renderer::geometry;

pub struct State<P>
where
    P: text::Paragraph,
{
    /// Upper line pixel points per series
    pub series_points: Vec<Vec<Point>>,
    /// Baseline pixel points per series (for fill polygon)
    pub series_baselines: Vec<Vec<Point>>,
    /// The drawn upper edge per series: the runs the missing-value policy
    /// keeps, traced by the chart's interpolation.
    pub series_strokes: Vec<Vec<Vec<Point>>>,
    /// The fill per series: one closed ring per run, along the traced upper
    /// edge and back along the traced floor.
    pub series_rings: Vec<Vec<Vec<Point>>>,
    /// Label texts per series
    pub series_label_texts: Vec<Vec<String>>,
    /// Data-label paragraphs, shaped once in `layout` and rendered in `draw`
    /// via [`crate::core::text::Renderer::fill_paragraph`]. Keyed the same as
    /// `series_label_texts` (outer: series, inner: drawn-label order) so the
    /// draw pass indexes a label's paragraph directly.
    pub series_labels: Vec<Vec<paragraph::Plain<P>>>,
    /// Resolved label positions per series
    pub series_label_positions: Vec<Vec<Position>>,
    /// Pixel rectangles for each label per series
    pub series_label_rects: Vec<Vec<Rectangle>>,
    /// Pixel y of the y-axis lower bound (plot area bottom). Used as the
    /// transparent end of vertical fill gradients so they respect the
    /// axis scale even when 0 is outside it.
    pub axis_floor_y: f32,
    /// Pixel y of the value axis minimum, where drop lines end.
    pub drop_floor_y: f32,
    /// Upper envelope points from the most recent layout before the current
    /// one, captured by [`crate::chart::Chart::diff`] when data changes so
    /// the next sweep can interpolate from previous positions to current
    /// positions. Empty on a fresh mount, in which case the animation
    /// collapses to a left-to-right envelope sweep.
    pub previous_series_points: Vec<Vec<Point>>,
    /// Baseline points from the most recent layout before the current one,
    /// kept in lockstep with `previous_series_points` so the closed
    /// envelope path stays well-formed at every interpolation step.
    pub previous_series_baselines: Vec<Vec<Point>>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

pub struct Area<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub(crate) data: &'a crate::mark::area::Area,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Area<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub fn new(data: &'a crate::mark::area::Area) -> Self {
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
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                series_points: Vec::new(),
                series_baselines: Vec::new(),
                series_strokes: Vec::new(),
                series_rings: Vec::new(),
                series_label_texts: Vec::new(),
                series_labels: Vec::new(),
                series_label_positions: Vec::new(),
                series_label_rects: Vec::new(),
                axis_floor_y: 0.0,
                drop_floor_y: 0.0,
                previous_series_points: Vec::new(),
                previous_series_baselines: Vec::new(),
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
            ..Tree::empty()
        }
    }

    pub(super) fn diff(&self, _tree: &mut Tree) {}

    #[allow(clippy::too_many_arguments)]
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        _limits: &Limits,
        domain: &Domain,
        rect: Rectangle,
        obstacles: &[Rectangle],
        design: Option<&dyn crate::design::Design>,
    ) {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        let zero_y = to_pixel(domain, rect, Datum::ORIGIN).y;
        state.axis_floor_y = rect.y + rect.height;
        state.drop_floor_y = super::line::floor_y(domain, rect);

        match self.data.layout {
            crate::mark::area::Layout::Overlaid => {
                state.series_points = self
                    .data
                    .series
                    .iter()
                    .map(|s| s.points.iter().map(|p| to_pixel(domain, rect, *p)).collect())
                    .collect();

                state.series_baselines = self
                    .data
                    .series
                    .iter()
                    .map(|s| {
                        s.points
                            .iter()
                            .map(|p| {
                                let px = to_pixel(domain, rect, *p);
                                Point::new(px.x, zero_y)
                            })
                            .collect()
                    })
                    .collect();
            }
            crate::mark::area::Layout::Stacked => {
                let num_points = self.data.series.iter().map(|s| s.points.len()).max().unwrap_or(0);
                let mut cumulative_y = vec![0.0_f64; num_points];

                state.series_points.clear();
                state.series_baselines.clear();

                for series in &self.data.series {
                    let mut upper = Vec::new();
                    let mut baseline = Vec::new();

                    for (i, point) in series.points.iter().enumerate() {
                        if i < num_points {
                            let base_y = cumulative_y[i];
                            // NaN/non-finite contribution doesn't poison the
                            // cumulative chain — subsequent series still see a
                            // finite baseline. The gap is signalled by emitting
                            // a NaN-y upper pixel so the polygon splits at this
                            // index instead of pinching to the baseline.
                            let inc = if point.y.is_finite() { point.y } else { 0.0 };
                            let new_y = base_y + inc;

                            let base_pixel = to_pixel(domain, rect, Datum::new(point.x, base_y));
                            let upper_pixel = if point.y.is_finite() {
                                to_pixel(domain, rect, Datum::new(point.x, new_y))
                            } else {
                                Point::new(base_pixel.x, f32::NAN)
                            };

                            baseline.push(base_pixel);
                            upper.push(upper_pixel);

                            cumulative_y[i] = new_y;
                        }
                    }

                    state.series_baselines.push(baseline);
                    state.series_points.push(upper);
                }
            }
        }

        (state.series_strokes, state.series_rings) = envelopes(
            &state.series_points,
            &state.series_baselines,
            self.data.nulls,
            self.data.interpolation,
        );

        // Build label info per series. Labels avoid:
        //   - every series' upper envelope (so a later series' label does
        //     not overlap an earlier series' line)
        //   - the sibling-axis gutter obstacles
        //   - labels already placed in this or earlier series
        state.series_label_texts.clear();
        state.series_labels.clear();
        state.series_label_positions.clear();
        state.series_label_rects.clear();
        state.series_label_texts.resize_with(self.data.series.len(), Vec::new);
        state.series_labels.resize_with(self.data.series.len(), Vec::new);
        state
            .series_label_positions
            .resize_with(self.data.series.len(), Vec::new);
        state.series_label_rects.resize_with(self.data.series.len(), Vec::new);

        let default_font = design
            .map(|d| d.data_label_text().resolved_font(renderer.font()))
            .unwrap_or_else(|| renderer.font());
        let default_size = design
            .and_then(|d| d.data_label_text().size)
            .map(|p| p.0)
            .unwrap_or(12.0);
        let hint_factor = renderer.hint_factor();

        let all_segments: Vec<(Point, Point)> = state
            .series_strokes
            .iter()
            .flatten()
            .flat_map(|run| run.windows(2).map(|w| (w[0], w[1])))
            .collect();

        let mut placed_rects: Vec<Rectangle> = Vec::new();

        for (series_idx, series) in self.data.series.iter().enumerate() {
            let Some(label_config) = &series.label else {
                continue;
            };

            let upper = &state.series_points[series_idx];
            let num_points = upper.len();
            if num_points == 0 {
                continue;
            }

            let label_size = label_config.text.size.map(|p| p.0).unwrap_or(default_size);
            let label_font = label_config.text.resolved_font(default_font);

            let minmax_indices: Vec<usize> = match label_config.show {
                Show::MinMaxFirst | Show::MinMaxAll | Show::MinMaxLast => {
                    let (min_val, max_val) = series
                        .points
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), p| {
                            (min.min(p.y), max.max(p.y))
                        });

                    match label_config.show {
                        Show::MinMaxFirst => {
                            let min_idx = series.points.iter().position(|p| p.y == min_val);
                            let max_idx = series.points.iter().position(|p| p.y == max_val);
                            [min_idx, max_idx].into_iter().flatten().collect()
                        }
                        Show::MinMaxLast => {
                            let min_idx = series.points.iter().rposition(|p| p.y == min_val);
                            let max_idx = series.points.iter().rposition(|p| p.y == max_val);
                            [min_idx, max_idx].into_iter().flatten().collect()
                        }
                        Show::MinMaxAll => series
                            .points
                            .iter()
                            .enumerate()
                            .filter(|(_, p)| p.y == min_val || p.y == max_val)
                            .map(|(i, _)| i)
                            .collect(),
                        _ => vec![],
                    }
                }
                _ => vec![],
            };

            for (idx, pixel_point) in upper.iter().enumerate() {
                if !pixel_point.x.is_finite() || !pixel_point.y.is_finite() {
                    continue;
                }
                let should_show = match label_config.show {
                    Show::Any => true,
                    Show::FirstOnly => idx == 0,
                    Show::LastOnly => idx == num_points - 1,
                    Show::FirstAndLast => idx == 0 || idx == num_points - 1,
                    Show::MinMaxFirst | Show::MinMaxAll | Show::MinMaxLast => minmax_indices.contains(&idx),
                };

                if !should_show {
                    continue;
                }

                // For stacked layout, the displayed value should be the
                // original (unstacked) series value, not the cumulative.
                let Some(data_point) = series.points.get(idx) else {
                    continue;
                };
                let label_text = (label_config.format)(data_point.y);

                let char_width = label_size * 0.6;
                let label_width = label_text.len() as f32 * char_width;
                let label_height = label_size * 1.2;

                let (resolved_position, label_rect) = place_label(
                    *pixel_point,
                    label_width,
                    label_height,
                    &all_segments,
                    &placed_rects,
                    obstacles,
                    Some(rect),
                );

                // If the user requested a fixed position, prefer it but still
                // let the placement fall back when the ideal slot is taken.
                let resolved_position = match label_config.position {
                    Position::Auto => resolved_position,
                    other => other,
                };
                let label_rect = if label_config.position == Position::Auto {
                    label_rect
                } else {
                    let preferred = compute_label_rect(*pixel_point, label_width, label_height, label_config.position);
                    // Only honor the fixed position if it doesn't collide.
                    if placed_rects
                        .iter()
                        .any(|r| crate::geometry::intersect::rects(*r, preferred))
                    {
                        label_rect
                    } else {
                        preferred
                    }
                };
                // Guarantee the label stays inside the plot area even at the
                // first/last data point, where the centered ideal rect would
                // otherwise bleed past the edge and get clipped.

                // Shape the label paragraph theme-free, keyed in lockstep with
                // `series_label_texts`. Rendered in `draw` via `fill_paragraph`.
                let mut paragraph = paragraph::Plain::default();
                let _ = paragraph.update(text::Text {
                    content: &label_text,
                    bounds: Size::INFINITE,
                    size: label_size.into(),
                    line_height: text::LineHeight::default(),
                    font: label_font,
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

                placed_rects.push(label_rect);
                state.series_label_texts[series_idx].push(label_text);
                state.series_labels[series_idx].push(paragraph);
                state.series_label_positions[series_idx].push(resolved_position);
                state.series_label_rects[series_idx].push(label_rect);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn draw<Theme>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &crate::core::renderer::Style,
        layout: crate::core::Layout,
        _cursor: crate::core::mouse::Cursor,
        _viewport: &crate::core::Rectangle,
        color_offset: usize,
        palette: &crate::palette::Resolved,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let mut labels = crate::chart::labels::Layer::new(crate::chart::labels::Space::new(layout.bounds()));
        self.draw_with_labels(
            tree,
            renderer,
            theme,
            _style,
            layout,
            _cursor,
            _viewport,
            color_offset,
            palette,
            &mut labels,
        );
        labels.draw(renderer, _viewport);
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn draw_with_labels<'p, Theme>(
        &self,
        tree: &'p crate::core::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &crate::core::renderer::Style,
        layout: crate::core::Layout,
        _cursor: crate::core::mouse::Cursor,
        _viewport: &crate::core::Rectangle,
        color_offset: usize,
        palette: &crate::palette::Resolved,
        labels: &mut crate::chart::labels::Layer<'p, Renderer::Paragraph>,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();
        let layout_bounds = layout.bounds();

        let mut fill_frame = Frame::new(renderer, layout_bounds.size());
        let mut stroke_frame = Frame::new(renderer, layout_bounds.size());

        // Sweep: each series' envelope (upper + baseline) interpolates from
        // its previous-layout envelope toward its current envelope. Without
        // a snapshot (fresh mount), upper and baseline are truncated in
        // lockstep at `floor(progress * len)` plus a partial endpoint, so
        // the closed `upper ++ reversed(baseline)` path stays well-formed
        // at every step. With `animate = false` progress pins to `1.0`,
        // collapsing the helper to identity returns of the laid-out
        // points and reproducing today's geometry exactly.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animating = progress < 1.0 - f32::EPSILON;
        let animated_series_points: Vec<Vec<Point>>;
        let animated_series_baselines: Vec<Vec<Point>>;
        {
            let mut points_out = Vec::with_capacity(state.series_points.len());
            let mut baselines_out = Vec::with_capacity(state.series_baselines.len());
            for (s, (cur_upper, cur_baseline)) in state
                .series_points
                .iter()
                .zip(state.series_baselines.iter())
                .enumerate()
            {
                let prev_upper = state.previous_series_points.get(s).map(Vec::as_slice).unwrap_or(&[]);
                let prev_baseline = state.previous_series_baselines.get(s).map(Vec::as_slice).unwrap_or(&[]);
                let (upper_anim, baseline_anim) =
                    animate_envelope(cur_upper, prev_upper, cur_baseline, prev_baseline, progress);
                points_out.push(upper_anim);
                baselines_out.push(baseline_anim);
            }
            animated_series_points = points_out;
            animated_series_baselines = baselines_out;
        }

        // Settled, the traced envelopes are the ones laid out; mid-sweep
        // they are traced afresh from the moving vertices.
        let swept;
        let (strokes, rings) = if animating {
            swept = envelopes(
                &animated_series_points,
                &animated_series_baselines,
                self.data.nulls,
                self.data.interpolation,
            );
            (&swept.0, &swept.1)
        } else {
            (&state.series_strokes, &state.series_rings)
        };

        for (series_idx, series) in self.data.series.iter().enumerate() {
            let (Some(series_strokes), Some(series_rings)) = (strokes.get(series_idx), rings.get(series_idx)) else {
                continue;
            };
            if series_rings.is_empty() && series_strokes.is_empty() {
                continue;
            }

            let base_color = if let Some(c) = series.color {
                c.resolve(background, text_pair, &seed, None)
            } else {
                palette
                    .get(color_offset + series_idx)
                    .resolve(background, text_pair, &seed, None)
            };

            let fill_color = crate::core::Color {
                a: base_color.a * series.opacity,
                ..base_color
            };

            // Each run of finite columns is its own closed polygon, so a
            // missing value leaves a gap instead of crashing the tessellator.
            let fill_path = Path::new(|builder| {
                for ring in series_rings {
                    let Some((first, rest)) = ring.split_first() else {
                        continue;
                    };
                    builder.move_to(*first);
                    for p in rest {
                        builder.line_to(*p);
                    }
                    builder.close();
                }
            });

            if let Some(pattern) = &series.pattern {
                let ink = crate::pattern::Ink::resolve(pattern, base_color, background, text_pair, &seed);
                pattern.fill(&mut fill_frame, &fill_path, ink);
            } else if self.data.gradient {
                // Vertical linear gradient: series color at the top of the
                // upper envelope, fully transparent at the y-axis floor.
                // Anchoring to the axis floor (not the polygon baseline)
                // keeps the gradient aligned with the visible scale when
                // 0 is outside the y-axis range.
                let top_y = series_rings.iter().flatten().map(|p| p.y).fold(f32::INFINITY, f32::min);
                let bottom_y = state.axis_floor_y;

                let gradient = Linear::new(Point::new(0.0, top_y), Point::new(0.0, bottom_y))
                    .add_stop(0.0, fill_color)
                    .add_stop(1.0, crate::core::Color { a: 0.0, ..base_color });

                fill_frame.fill(&fill_path, Fill::from(gradient));
            } else {
                fill_frame.fill(&fill_path, fill_color);
            }

            if let Some(stroke_width) = series.stroke {
                let segments = dash_segments(&series.style);
                let mut stroke = Stroke::default().with_width(stroke_width).with_color(base_color);
                stroke.line_dash = LineDash { segments, offset: 0 };
                if matches!(series.style, LineStyle::Dotted) {
                    stroke = stroke.with_line_cap(LineCap::Round);
                }
                stroke_runs(&mut stroke_frame, series_strokes, stroke);
            }
        }

        // Drop lines run from each settled point to the plot floor, over the
        // fills and under the strokes.
        if let Some(connector) = &self.data.drop_lines
            && !animating
        {
            let ink = text_pair.resolve(background, None);
            for upper in &state.series_points {
                draw_drop_lines(
                    &mut fill_frame,
                    upper,
                    state.drop_floor_y,
                    connector,
                    ink,
                    text_pair,
                    &seed,
                    background,
                );
            }
        }

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);

        let fill_geometry = fill_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(fill_geometry);
        });

        let stroke_geometry = stroke_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(stroke_geometry);
        });

        // Markers anchor at vertices the envelope might not have grown to
        // yet; suppress them until the sweep settles, matching the line
        // renderer.
        let needs_marker_frame = !animating && self.data.series.iter().any(|s| s.marker.is_some());
        if needs_marker_frame {
            let mut marker_frame = Frame::new(renderer, layout_bounds.size());
            for (series_idx, series) in self.data.series.iter().enumerate() {
                let Some(marker_config) = &series.marker else {
                    continue;
                };
                let upper = &state.series_points[series_idx];
                if upper.is_empty() {
                    continue;
                }
                let base_color = if let Some(c) = series.color {
                    c.resolve(background, text_pair, &seed, None)
                } else {
                    palette
                        .get(color_offset + series_idx)
                        .resolve(background, text_pair, &seed, None)
                };
                draw_markers(
                    &mut marker_frame,
                    upper,
                    &series.points,
                    marker_config,
                    base_color,
                    background,
                    text_pair,
                    &seed,
                );
            }
            let marker_geometry = marker_frame.into_geometry();
            renderer.with_translation(translation, |renderer| {
                renderer.draw_geometry(marker_geometry);
            });
        }

        // Draw data labels per series. All series share one frame so that
        // we only emit a single `draw_geometry` for label background fills
        // regardless of how many series have labels configured. The text
        // glyphs are drawn afterward via the cached paragraphs
        // (`fill_paragraph`) so they sit on top. Suppressed mid-sweep so
        // labels don't pop in over an envelope still growing into place.
        let mut text_draws: Vec<(usize, usize, Point, crate::core::Color)> = Vec::new();

        if !animating {
            for (series_idx, series) in self.data.series.iter().enumerate() {
                let Some(label_config) = &series.label else {
                    continue;
                };

                let base_color = if let Some(c) = series.color {
                    c.resolve(background, text_pair, &seed, None)
                } else {
                    palette
                        .get(color_offset + series_idx)
                        .resolve(background, text_pair, &seed, None)
                };

                let label_color = if let Some(label_color_spec) = label_config.color {
                    label_color_spec.resolve(background, text_pair, &seed, None)
                } else {
                    base_color
                };

                let label_fill_color = label_config
                    .fill
                    .map(|spec| spec.resolve(background, text_pair, &seed, None));

                let paragraphs = &state.series_labels[series_idx];
                let positions = &state.series_label_positions[series_idx];
                let rects = &state.series_label_rects[series_idx];

                for (drawn_idx, ((label_rect, paragraph), resolved_pos)) in
                    rects.iter().zip(paragraphs.iter()).zip(positions.iter()).enumerate()
                {
                    if let Some(fill_color) = label_fill_color {
                        labels.background(
                            *label_rect + crate::core::Vector::new(layout_bounds.x, layout_bounds.y),
                            fill_color,
                        );
                    }

                    let (align_x, align_y) = alignment_for_position(*resolved_pos);

                    let (anchor_x, anchor_y) = match resolved_pos {
                        Position::Auto | Position::Above => {
                            (label_rect.x + label_rect.width / 2.0, label_rect.y + label_rect.height)
                        }
                        Position::Below => (label_rect.x + label_rect.width / 2.0, label_rect.y),
                        Position::Left => (label_rect.x + label_rect.width, label_rect.y + label_rect.height / 2.0),
                        Position::Right => (label_rect.x, label_rect.y + label_rect.height / 2.0),
                    };

                    // `fill_text` shifted the glyph box from the anchor by the
                    // paragraph's `min_bounds` per alignment (Center: −½,
                    // Right/Bottom: −1); apply the identical shift against the
                    // cached paragraph so `fill_paragraph` (top-left origin)
                    // lands pixel-identically.
                    let bounds = paragraph.min_bounds();
                    let px = match align_x {
                        crate::core::alignment::Horizontal::Left => anchor_x,
                        crate::core::alignment::Horizontal::Center => anchor_x - bounds.width / 2.0,
                        crate::core::alignment::Horizontal::Right => anchor_x - bounds.width,
                    };
                    let py = match align_y {
                        crate::core::alignment::Vertical::Top => anchor_y,
                        crate::core::alignment::Vertical::Center => anchor_y - bounds.height / 2.0,
                        crate::core::alignment::Vertical::Bottom => anchor_y - bounds.height,
                    };
                    text_draws.push((series_idx, drawn_idx, Point::new(px, py), label_color));
                }
            }
        }

        // Draw the label text on top of any background fills via the cached
        // paragraphs.
        for (series_idx, drawn_idx, anchor, color) in text_draws {
            if let Some(paragraph) = state
                .series_labels
                .get(series_idx)
                .and_then(|labels| labels.get(drawn_idx))
            {
                labels.push(paragraph.raw(), anchor + translation, color);
            }
        }
    }
}

/// Traces every series' drawn edge and fill from its upper and baseline
/// pixel points: the runs the missing-value policy keeps, each traced by the
/// chart's interpolation. A fill ring runs along the traced top and back
/// along the traced floor; a run of one column draws nothing.
#[allow(clippy::type_complexity)]
fn envelopes(
    uppers: &[Vec<Point>],
    baselines: &[Vec<Point>],
    nulls: Nulls,
    interpolation: Interpolation,
) -> (Vec<Vec<Vec<Point>>>, Vec<Vec<Vec<Point>>>) {
    uppers
        .iter()
        .zip(baselines)
        .map(|(upper, baseline)| {
            let runs: Vec<_> = nulls
                .runs(upper, baseline)
                .into_iter()
                .filter(|run| run.upper.len() >= 2)
                .collect();
            let strokes: Vec<Vec<Point>> = runs.iter().map(|run| interpolation.trace(&run.upper)).collect();
            let rings = runs
                .iter()
                .zip(&strokes)
                .map(|(run, top)| {
                    let mut ring = top.clone();
                    ring.extend(interpolation.trace(&run.lower).into_iter().rev());
                    ring
                })
                .collect();
            (strokes, rings)
        })
        .unzip()
}

/// Returns the upper and baseline polylines for one series at the
/// current sweep progress.
///
/// With `prev_*` snapshots (data change), every vertex lerps
/// component-wise from `prev[i]` to `cur[i]`. Vertices appended past
/// the prev length anchor at the cur arrays' leading edges so they
/// emerge from the envelope's left edge as `progress` advances. Both
/// arrays iterate the same index range so the closed
/// `upper ++ reversed(baseline)` path keeps matching upper/baseline
/// vertex counts at every step.
///
/// Without one (fresh mount), upper and baseline are truncated in
/// lockstep at `floor(progress * len)` full vertices plus one partial
/// segment endpoint appended to each, producing a left-to-right
/// envelope sweep. Stacked layouts rely on this lockstep: each
/// series' baseline is the previous series' upper, and pointwise
/// lerping (or pointwise truncation) on both keeps the segments
/// aligned without extra bookkeeping.
///
/// At `progress >= 1.0 - EPSILON` (or empty input) the result equals
/// `(cur_upper, cur_baseline)` exactly, so disabling animation
/// reproduces today's geometry.
fn animate_envelope(
    cur_upper: &[Point],
    prev_upper: &[Point],
    cur_baseline: &[Point],
    prev_baseline: &[Point],
    progress: f32,
) -> (Vec<Point>, Vec<Point>) {
    debug_assert_eq!(
        cur_upper.len(),
        cur_baseline.len(),
        "area envelope upper and baseline must have matching length"
    );

    if cur_upper.is_empty() || progress >= 1.0 - f32::EPSILON {
        return (cur_upper.to_vec(), cur_baseline.to_vec());
    }

    if !prev_upper.is_empty() || !prev_baseline.is_empty() {
        let upper_anchor = cur_upper[0];
        let baseline_anchor = cur_baseline[0];
        let upper = cur_upper
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let p = prev_upper.get(i).copied().unwrap_or(upper_anchor);
                Point {
                    x: p.x + (c.x - p.x) * progress,
                    y: p.y + (c.y - p.y) * progress,
                }
            })
            .collect();
        let baseline = cur_baseline
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let p = prev_baseline.get(i).copied().unwrap_or(baseline_anchor);
                Point {
                    x: p.x + (c.x - p.x) * progress,
                    y: p.y + (c.y - p.y) * progress,
                }
            })
            .collect();
        return (upper, baseline);
    }

    let len = cur_upper.len();
    let scaled = progress * len as f32;
    let len_visible = (scaled.floor() as usize).min(len);
    if len_visible >= len {
        return (cur_upper.to_vec(), cur_baseline.to_vec());
    }

    let partial_t = scaled - len_visible as f32;
    let partial_endpoint = |full: &[Point], next: Point| -> Point {
        match len_visible.checked_sub(1).and_then(|i| full.get(i)).copied() {
            Some(last_full) => Point {
                x: last_full.x + (next.x - last_full.x) * partial_t,
                y: last_full.y + (next.y - last_full.y) * partial_t,
            },
            None => next,
        }
    };

    let mut upper_out: Vec<Point> = cur_upper[..len_visible].to_vec();
    upper_out.push(partial_endpoint(cur_upper, cur_upper[len_visible]));

    let mut baseline_out: Vec<Point> = cur_baseline[..len_visible].to_vec();
    baseline_out.push(partial_endpoint(cur_baseline, cur_baseline[len_visible]));

    (upper_out, baseline_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(ys: &[f32]) -> Vec<Point> {
        ys.iter()
            .enumerate()
            .map(|(i, &y)| Point::new(i as f32 * 30.0, y))
            .collect()
    }

    #[test]
    fn stacked_layers_share_their_traced_edge() {
        let floor = row(&[200.0; 5]);
        let lower = row(&[150.0, 120.0, 160.0, 90.0, 140.0]);
        let upper = row(&[100.0, 60.0, 110.0, 40.0, 80.0]);
        for interpolation in [
            Interpolation::Monotone,
            Interpolation::Natural,
            Interpolation::Step(crate::line::Step::Mid),
        ] {
            let (strokes, rings) = envelopes(
                &[lower.clone(), upper.clone()],
                &[floor.clone(), lower.clone()],
                Nulls::Gap,
                interpolation,
            );
            let edge = &strokes[0][0];
            let ring = &rings[1][0];
            let floor_of_upper: Vec<Point> = ring[ring.len() - edge.len()..].iter().rev().copied().collect();
            assert_eq!(&floor_of_upper, edge, "{interpolation:?} opens a seam");
        }
    }

    #[test]
    fn gaps_split_fills_and_connect_bridges_them() {
        let floor = row(&[200.0; 5]);
        let upper = row(&[100.0, 90.0, f32::NAN, 80.0, 70.0]);
        let (_, gap) = envelopes(
            std::slice::from_ref(&upper),
            std::slice::from_ref(&floor),
            Nulls::Gap,
            Interpolation::Linear,
        );
        assert_eq!(gap[0].len(), 2);
        let (_, connect) = envelopes(
            std::slice::from_ref(&upper),
            std::slice::from_ref(&floor),
            Nulls::Connect,
            Interpolation::Linear,
        );
        assert_eq!(connect[0].len(), 1);
        assert_eq!(connect[0][0].len(), 8);
        let (strokes, zero) = envelopes(&[upper], &[floor], Nulls::Zero, Interpolation::Linear);
        assert_eq!(zero[0].len(), 1);
        assert_eq!(strokes[0][0][2], Point::new(60.0, 200.0));
    }
}

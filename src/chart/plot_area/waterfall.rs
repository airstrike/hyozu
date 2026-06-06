use super::{Domain, to_pixel};
use crate::animation;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Rectangle, Size};
use crate::data::Datum;
use crate::mark::waterfall::EntryKind;
use crate::mark::waterfall::label::Position;
use crate::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// State for Waterfall — stores pre-calculated bar rectangles and running totals
pub struct State {
    /// Pixel rectangles for each bar
    pub rects: Vec<Rectangle>,
    /// The kind of each entry (for coloring)
    pub kinds: Vec<EntryKind>,
    /// Top Y position of each bar (for connector lines)
    pub tops: Vec<f32>,
    /// Estimated label rectangles (None = no label drawn for that entry)
    pub label_rects: Vec<Option<Rectangle>>,
    /// Bar rectangles from the most recent layout before the current one,
    /// captured by [`crate::chart::Chart::diff`] when data changes so the
    /// next sweep can interpolate from previous bounds to current bounds.
    /// Empty on a fresh mount, in which case the animation collapses to a
    /// baseline-anchored grow-in.
    pub previous_rects: Vec<Rectangle>,
    /// Connector top y-coordinates from the previous layout, snapshotted
    /// alongside [`Self::previous_rects`] so the joining lines lerp from
    /// where they were to where they are. Empty on a fresh mount.
    pub previous_tops: Vec<f32>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// A Waterfall series that renders waterfall charts.
pub struct Waterfall<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::waterfall::Waterfall,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

/// A label resolved for a single waterfall entry.
struct LabelDraw {
    text: String,
    position: Position,
    size: f32,
}

impl<'a, Message, Renderer> Waterfall<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Waterfall borrowing data
    pub fn new(data: &'a crate::mark::waterfall::Waterfall) -> Self {
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

    /// Returns the initial tree state for this Waterfall
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                rects: Vec::new(),
                kinds: Vec::new(),
                tops: Vec::new(),
                label_rects: Vec::new(),
                previous_rects: Vec::new(),
                previous_tops: Vec::new(),
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Waterfall state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the waterfall — compute bar positions from running totals
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        domain: &Domain,
        rect: Rectangle,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();

        if self.data.entries.is_empty() {
            state.rects.clear();
            state.kinds.clear();
            state.tops.clear();
            state.label_rects.clear();
            return Node::new(Size::ZERO);
        }

        let num_entries = self.data.entries.len();
        let total_width = rect.width;
        let bin_width = total_width / num_entries as f32;
        let bar_width = bin_width * 0.75;
        let zero_y = to_pixel(domain, rect, Datum::ORIGIN).y;

        let mut running_total: f64 = 0.0;
        state.rects.clear();
        state.kinds.clear();
        state.tops.clear();

        for (i, entry) in self.data.entries.iter().enumerate() {
            let bar_center_x = to_pixel(domain, rect, Datum::x(i as f64)).x;
            let x = bar_center_x - bar_width / 2.0;

            // Non-finite entry value = gap: don't poison the running total
            // and don't emit a renderable rect. The sentinel NaN rect is
            // skipped by the bar/label/connector loops in `draw`.
            if !entry.value.is_finite() {
                state.rects.push(Rectangle {
                    x: f32::NAN,
                    y: f32::NAN,
                    width: f32::NAN,
                    height: f32::NAN,
                });
                state.tops.push(f32::NAN);
                state.kinds.push(entry.kind);
                continue;
            }

            match entry.kind {
                EntryKind::Total => {
                    running_total = entry.value;
                    let top_y = to_pixel(domain, rect, Datum::new(0.0, entry.value)).y;
                    let height = (zero_y - top_y).abs();
                    let y = top_y.min(zero_y);

                    state.rects.push(Rectangle {
                        x,
                        y,
                        width: bar_width,
                        height,
                    });
                    state.tops.push(top_y);
                }
                EntryKind::Increase => {
                    let prev_total = running_total;
                    running_total += entry.value;
                    let bottom_y = to_pixel(domain, rect, Datum::new(0.0, prev_total)).y;
                    let top_y = to_pixel(domain, rect, Datum::new(0.0, running_total)).y;
                    let height = (bottom_y - top_y).abs();
                    let y = top_y.min(bottom_y);

                    state.rects.push(Rectangle {
                        x,
                        y,
                        width: bar_width,
                        height,
                    });
                    state.tops.push(top_y);
                }
                EntryKind::Decrease => {
                    let prev_total = running_total;
                    running_total += entry.value;
                    let top_y = to_pixel(domain, rect, Datum::new(0.0, prev_total)).y;
                    let bottom_y = to_pixel(domain, rect, Datum::new(0.0, running_total)).y;
                    let height = (bottom_y - top_y).abs();
                    let y = top_y.min(bottom_y);

                    state.rects.push(Rectangle {
                        x,
                        y,
                        width: bar_width,
                        height,
                    });
                    state.tops.push(bottom_y);
                }
            }

            state.kinds.push(entry.kind);
        }

        state.label_rects = self.compute_label_rects(state);

        Node::new(Size::ZERO)
    }

    fn compute_label_rects(&self, state: &State) -> Vec<Option<Rectangle>> {
        let total_entries = self.data.entries.len();
        let chart_label = self.data.label.as_ref();
        let default_size = 12.0;

        self.data
            .entries
            .iter()
            .zip(state.rects.iter())
            .enumerate()
            .map(|(i, (entry, rect))| {
                let LabelDraw { text, position, size } = resolve_label(entry, chart_label, default_size)?;

                if !chart_label
                    .map(|l| l.show.allows(i, total_entries, entry.kind))
                    .unwrap_or(true)
                {
                    return None;
                }

                let (lx, ly, align_x, align_y) = label_position(rect, position, entry.kind, entry.value);

                let char_width = size * 0.6;
                let text_width = text.len() as f32 * char_width + 6.0;
                let text_height = size * 1.2 + 4.0;

                let x = match align_x {
                    crate::core::alignment::Horizontal::Left => lx,
                    crate::core::alignment::Horizontal::Center => lx - text_width / 2.0,
                    crate::core::alignment::Horizontal::Right => lx - text_width,
                };
                let y = match align_y {
                    crate::core::alignment::Vertical::Top => ly,
                    crate::core::alignment::Vertical::Center => ly - text_height / 2.0,
                    crate::core::alignment::Vertical::Bottom => ly - text_height,
                };

                Some(Rectangle {
                    x,
                    y,
                    width: text_width,
                    height: text_height,
                })
            })
            .collect()
    }

    /// Draws the waterfall chart
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

        if state.rects.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let increase_color = crate::color::Color::Success.resolve(background, text_pair, &seed, None);
        let decrease_color = crate::color::Color::Danger.resolve(background, text_pair, &seed, None);
        let total_color = crate::color::Color::Primary.resolve(background, text_pair, &seed, None);

        let mut bar_colors: Vec<crate::core::Color> = Vec::with_capacity(state.rects.len());

        // Sweep with a left-to-right "running-total walk": each bar gets
        // its own local progress fraction so the staircase reveals itself
        // from the opening Total to the closing one, instead of every bar
        // animating in lockstep. Without staggering, tall Total bars
        // dominate the visible motion and the small Increase/Decrease bars
        // appear to flash in instantly. Each bar's window covers
        // `BAR_DURATION_FRACTION` of the total duration; consecutive bars
        // are offset by `step` so bar 0 spans `[0, BAR_DURATION_FRACTION]`
        // and the last bar finishes at `1.0`. `animating` stays gated on
        // the global progress so labels/connectors only appear once every
        // bar has settled. With `animate = false` progress pins to `1.0`
        // and the animated geometry equals `state.rects` / `state.tops`
        // exactly, reproducing today's geometry.
        const BAR_DURATION_FRACTION: f32 = 0.5;
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animating = progress < 1.0 - f32::EPSILON;
        let n = state.rects.len();
        let step = if n > 1 {
            (1.0 - BAR_DURATION_FRACTION) / (n - 1) as f32
        } else {
            0.0
        };
        let local_progress = |i: usize| -> f32 {
            if !self.animate || n <= 1 {
                return progress;
            }
            ((progress - i as f32 * step) / BAR_DURATION_FRACTION).clamp(0.0, 1.0)
        };
        let animated_rects: Vec<Rectangle> = state
            .rects
            .iter()
            .enumerate()
            .map(|(i, cur)| {
                let prev = state.previous_rects.get(i).copied();
                let kind = state.kinds.get(i).copied().unwrap_or(EntryKind::Increase);
                animate_rect(*cur, prev, local_progress(i), kind)
            })
            .collect();
        let animated_tops: Vec<f32> = state
            .tops
            .iter()
            .zip(state.rects.iter())
            .enumerate()
            .map(|(i, (&cur_top, &cur_rect))| {
                let kind = state.kinds.get(i).copied().unwrap_or(EntryKind::Increase);
                animate_top(
                    cur_top,
                    cur_rect,
                    state.previous_tops.get(i).copied(),
                    local_progress(i),
                    kind,
                )
            })
            .collect();

        // Draw bars
        for (i, (rect, entry)) in animated_rects.iter().zip(self.data.entries.iter()).enumerate() {
            let color = if let Some(entry_color) = entry.color {
                entry_color.resolve(background, text_pair, &seed, None)
            } else {
                match state.kinds[i] {
                    EntryKind::Increase => increase_color,
                    EntryKind::Decrease => decrease_color,
                    EntryKind::Total => total_color,
                }
            };
            bar_colors.push(color);

            if !rect.x.is_finite() || !rect.y.is_finite() || !rect.width.is_finite() || !rect.height.is_finite() {
                continue;
            }

            let path = Path::new(|builder| {
                builder.rectangle(
                    crate::core::Point::new(rect.x, rect.y),
                    crate::core::Size::new(rect.width, rect.height),
                );
            });
            frame.fill(&path, color);
        }

        // Connector lines — suppressed mid-sweep so the running-total
        // interpretation doesn't read at intermediate bar heights, and
        // so a long horizontal line doesn't sit at the baseline before
        // the bars have grown into their final positions.
        if self.data.connector && animated_rects.len() > 1 && !animating {
            let connector_color = crate::core::Color {
                a: 0.4,
                ..text_pair.on_light
            };

            for i in 0..animated_rects.len() - 1 {
                let from_rect = &animated_rects[i];
                let to_rect = &animated_rects[i + 1];
                let y = animated_tops[i];

                // Skip connectors that touch a gap entry (NaN rect or top).
                if !y.is_finite() || !from_rect.x.is_finite() || !from_rect.width.is_finite() || !to_rect.x.is_finite()
                {
                    continue;
                }

                let path = Path::new(|builder| {
                    builder.move_to(crate::core::Point::new(from_rect.x + from_rect.width, y));
                    builder.line_to(crate::core::Point::new(to_rect.x, y));
                });

                frame.stroke(&path, Stroke::default().with_width(1.0).with_color(connector_color));
            }
        }

        // Labels — suppressed mid-sweep so they don't pop in over bars
        // that haven't grown into their final positions yet.
        let chart_label = self.data.label.as_ref();
        let theme_default_size = theme.font_size();
        let total_entries = self.data.entries.len();

        if !animating {
            for (i, (rect, entry)) in state.rects.iter().zip(self.data.entries.iter()).enumerate() {
                if !rect.x.is_finite() || !rect.y.is_finite() || !rect.width.is_finite() || !rect.height.is_finite() {
                    continue;
                }
                let LabelDraw {
                    text: label_text,
                    position,
                    size,
                } = match resolve_label(entry, chart_label, theme_default_size) {
                    Some(d) => d,
                    None => continue,
                };

                if !chart_label
                    .map(|l| l.show.allows(i, total_entries, entry.kind))
                    .unwrap_or(true)
                {
                    continue;
                }

                let (lx, ly, align_x, align_y) = label_position(rect, position, entry.kind, entry.value);

                // Background fill
                if let Some(label) = chart_label
                    && let Some(fill_spec) = label.fill
                    && let Some(Some(lr)) = state.label_rects.get(i)
                {
                    let fill_resolved = fill_spec.resolve(background, text_pair, &seed, None);
                    let fill_path = Path::new(|b| {
                        b.rectangle(
                            crate::core::Point::new(lr.x, lr.y),
                            crate::core::Size::new(lr.width, lr.height),
                        );
                    });
                    frame.fill(&fill_path, fill_resolved);
                }

                let label_color_spec = chart_label
                    .and_then(|l| l.color)
                    .unwrap_or(crate::color::Color::CONTRAST);
                let label_color = match position {
                    Position::Above => label_color_spec.resolve(background, text_pair, &seed, None),
                    _ => label_color_spec.resolve(bar_colors[i], text_pair, &seed, Some(background)),
                };

                let base_font = theme.data_label_text().resolved_font(theme.font());
                let mut font = chart_label
                    .map(|l| l.text.resolved_font(base_font))
                    .unwrap_or(base_font);
                if let Some(w) = chart_label.and_then(|l| l.text.weight) {
                    font.weight = w;
                }
                if let Some(s) = chart_label.and_then(|l| l.text.style) {
                    font.style = s;
                }

                frame.fill_text(CanvasText {
                    content: label_text,
                    position: crate::core::Point::new(lx, ly),
                    color: label_color,
                    size: size.into(),
                    font,
                    align_x: align_x.into(),
                    align_y,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }
}

/// Resolves the rendered text, position, and font size for an entry's label.
///
/// Per-entry static `text` overrides chart-wide `Label.format(value)`. When
/// neither source produces text the entry has no label.
fn resolve_label(
    entry: &crate::mark::waterfall::Entry,
    chart_label: Option<&crate::mark::waterfall::Label>,
    default_size: f32,
) -> Option<LabelDraw> {
    let position = chart_label.map(|l| l.position).unwrap_or(Position::Above);
    let size = chart_label
        .and_then(|l| l.text.size.map(|p| p.0))
        .unwrap_or(default_size);

    if let Some(static_text) = &entry.text {
        return Some(LabelDraw {
            text: static_text.clone(),
            position,
            size,
        });
    }

    let label = chart_label?;
    let text = (label.format)(entry.value);
    if text.is_empty() {
        return None;
    }
    Some(LabelDraw { text, position, size })
}

/// Computes label position and alignment for a waterfall bar.
///
/// `Above` flips below the bar for negative values so labels always sit on the
/// outside edge (above for positive bars, below for negative).
fn label_position(
    rect: &Rectangle,
    position: Position,
    kind: EntryKind,
    value: f64,
) -> (
    f32,
    f32,
    crate::core::alignment::Horizontal,
    crate::core::alignment::Vertical,
) {
    use crate::core::alignment::{Horizontal, Vertical};

    let negative = match kind {
        EntryKind::Decrease => true,
        EntryKind::Increase => false,
        EntryKind::Total => value < 0.0,
    };
    let cx = rect.x + rect.width / 2.0;

    match position {
        Position::Above => {
            if negative {
                (cx, rect.y + rect.height + 4.0, Horizontal::Center, Vertical::Top)
            } else {
                (cx, rect.y - 4.0, Horizontal::Center, Vertical::Bottom)
            }
        }
        Position::End => {
            if negative {
                (cx, rect.y + rect.height - 4.0, Horizontal::Center, Vertical::Bottom)
            } else {
                (cx, rect.y + 4.0, Horizontal::Center, Vertical::Top)
            }
        }
        Position::Center => (cx, rect.y + rect.height / 2.0, Horizontal::Center, Vertical::Center),
        Position::Base => {
            if negative {
                (cx, rect.y + 4.0, Horizontal::Center, Vertical::Top)
            } else {
                (cx, rect.y + rect.height - 4.0, Horizontal::Center, Vertical::Bottom)
            }
        }
    }
}

/// Computes the on-screen rectangle for a waterfall bar at the current
/// sweep progress. With a `prev` rect (data change), x/y/width/height
/// each lerp linearly from `prev` to `cur`. Without one (fresh mount),
/// the bar grows from a kind-specific anchor edge:
///
/// - `Increase` and `Total` bars grow upward from their bottom edge
///   (`cur.y + cur.height`), since that edge represents the running
///   total before the bar's value is applied.
/// - `Decrease` bars grow downward from their top edge (`cur.y`), which
///   represents the running total before the decrement. Anchoring at
///   the top makes the bar visually emerge from the previous total
///   rather than from below the new total.
///
/// At `progress == 1.0` the result equals `cur` exactly in every
/// branch, so disabling animation reproduces today's geometry.
fn animate_rect(cur: Rectangle, prev: Option<Rectangle>, progress: f32, kind: EntryKind) -> Rectangle {
    if let Some(prev) = prev {
        let x = prev.x + (cur.x - prev.x) * progress;
        let y = prev.y + (cur.y - prev.y) * progress;
        let width = prev.width + (cur.width - prev.width) * progress;
        let height = prev.height + (cur.height - prev.height) * progress;
        Rectangle { x, y, width, height }
    } else {
        match kind {
            EntryKind::Decrease => Rectangle {
                x: cur.x,
                y: cur.y,
                width: cur.width,
                height: cur.height * progress,
            },
            EntryKind::Increase | EntryKind::Total => Rectangle {
                x: cur.x,
                y: cur.y + cur.height * (1.0 - progress),
                width: cur.width,
                height: cur.height * progress,
            },
        }
    }
}

/// Computes the connector y-coordinate for bar `i` at the current sweep
/// progress. With a `prev_top` (data change), lerps linearly from
/// `prev_top` to `cur_top`. Without one (fresh mount), tracks the
/// animated bar's running-total edge: for `Increase` / `Total` the
/// anchor is the bottom edge (`cur_rect.y + cur_rect.height`); for
/// `Decrease` it's the top edge (`cur_rect.y`). The connector slides
/// from that anchor toward `cur_top` so it stays glued to the animated
/// bar's running-total edge across the sweep. At `progress == 1.0` the
/// result equals `cur_top` exactly in every branch.
fn animate_top(cur_top: f32, cur_rect: Rectangle, prev_top: Option<f32>, progress: f32, kind: EntryKind) -> f32 {
    if let Some(prev_top) = prev_top {
        prev_top + (cur_top - prev_top) * progress
    } else {
        let anchor = match kind {
            EntryKind::Decrease => cur_rect.y,
            EntryKind::Increase | EntryKind::Total => cur_rect.y + cur_rect.height,
        };
        anchor + (cur_top - anchor) * progress
    }
}

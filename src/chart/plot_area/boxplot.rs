use super::{Domain, to_pixel};
use crate::animation;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size};
use crate::data::Datum;
use crate::mark::boxplot::Direction;
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Outlier circle radius in pixels.
const OUTLIER_RADIUS: f32 = 3.0;

/// Layout data for a single box plot entry.
#[derive(Clone)]
pub struct EntryLayout {
    pub center_x: f32,
    pub min_y: f32,
    pub q1_y: f32,
    pub median_y: f32,
    pub q3_y: f32,
    pub max_y: f32,
    pub box_width: f32,
    pub outlier_ys: Vec<f32>,
}

/// State for BoxPlot — stores pre-calculated layout positions.
pub struct State {
    /// Pixel rectangles for Q1-Q3 boxes (for hit-testing)
    pub box_rects: Vec<Rectangle>,
    /// Full layout data for each entry
    pub entries_layout: Vec<EntryLayout>,
    /// Per-entry layout snapshot from the most recent layout before the
    /// current one, captured by [`crate::chart::Chart::diff`] when data
    /// changes so the next sweep can interpolate every component
    /// (box, whiskers, median, outliers) from previous to current.
    /// Empty on a fresh mount, in which case the animation collapses to
    /// a median-axis-anchored grow-out.
    pub previous_entries: Vec<EntryLayout>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// A BoxPlot series that renders box-and-whisker charts.
pub struct BoxPlot<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::boxplot::BoxPlot,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> BoxPlot<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new BoxPlot borrowing data
    pub fn new(data: &'a crate::mark::boxplot::BoxPlot) -> Self {
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

    /// Returns the initial tree state for this BoxPlot
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                box_rects: Vec::new(),
                entries_layout: Vec::new(),
                previous_entries: Vec::new(),
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current BoxPlot state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the box plot — calculate positions and sizes
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
            state.box_rects.clear();
            state.entries_layout.clear();
            return Node::new(Size::ZERO);
        }

        let num_entries = self.data.entries.len();
        state.box_rects.clear();
        state.entries_layout.clear();

        match self.data.direction {
            Direction::Vertical => {
                let total_width = rect.width;
                let bin_width = total_width / num_entries as f32;
                let box_width = bin_width * self.data.width;

                for (i, entry) in self.data.entries.iter().enumerate() {
                    let center_x = to_pixel(domain, rect, Datum::x(i as f64)).x;

                    let q1_y = to_pixel(domain, rect, Datum::new(i as f64, entry.q1)).y;
                    let q3_y = to_pixel(domain, rect, Datum::new(i as f64, entry.q3)).y;
                    let min_y = to_pixel(domain, rect, Datum::new(i as f64, entry.min)).y;
                    let max_y = to_pixel(domain, rect, Datum::new(i as f64, entry.max)).y;
                    let median_y = to_pixel(domain, rect, Datum::new(i as f64, entry.median)).y;

                    let outlier_ys: Vec<f32> = entry
                        .outliers
                        .iter()
                        .map(|&o| to_pixel(domain, rect, Datum::new(i as f64, o)).y)
                        .collect();

                    // q3 is higher value -> lower y pixel
                    let rect_y = q3_y.min(q1_y);
                    let rect_height = (q1_y - q3_y).abs();

                    state.box_rects.push(Rectangle {
                        x: center_x - box_width / 2.0,
                        y: rect_y,
                        width: box_width,
                        height: rect_height,
                    });

                    state.entries_layout.push(EntryLayout {
                        center_x,
                        min_y,
                        q1_y,
                        median_y,
                        q3_y,
                        max_y,
                        box_width,
                        outlier_ys,
                    });
                }
            }
            Direction::Horizontal => {
                let total_height = rect.height;
                let bin_height = total_height / num_entries as f32;
                let box_height = bin_height * self.data.width;

                for (i, entry) in self.data.entries.iter().enumerate() {
                    let center_y = to_pixel(domain, rect, Datum::y(i as f64)).y;

                    let q1_x = to_pixel(domain, rect, Datum::new(entry.q1, i as f64)).x;
                    let q3_x = to_pixel(domain, rect, Datum::new(entry.q3, i as f64)).x;
                    let min_x = to_pixel(domain, rect, Datum::new(entry.min, i as f64)).x;
                    let max_x = to_pixel(domain, rect, Datum::new(entry.max, i as f64)).x;
                    let median_x = to_pixel(domain, rect, Datum::new(entry.median, i as f64)).x;

                    let outlier_xs: Vec<f32> = entry
                        .outliers
                        .iter()
                        .map(|&o| to_pixel(domain, rect, Datum::new(o, i as f64)).x)
                        .collect();

                    let rect_x = q1_x.min(q3_x);
                    let rect_width = (q3_x - q1_x).abs();

                    state.box_rects.push(Rectangle {
                        x: rect_x,
                        y: center_y - box_height / 2.0,
                        width: rect_width,
                        height: box_height,
                    });

                    // Reuse EntryLayout — for horizontal, we store x values in the y fields
                    // center_x -> center_y, min_y -> min_x, etc.
                    state.entries_layout.push(EntryLayout {
                        center_x: center_y,
                        min_y: min_x,
                        q1_y: q1_x,
                        median_y: median_x,
                        q3_y: q3_x,
                        max_y: max_x,
                        box_width: box_height,
                        outlier_ys: outlier_xs,
                    });
                }
            }
        }

        Node::new(Size::ZERO)
    }

    /// Draws the box plot using pre-calculated layout data
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

        if state.entries_layout.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        // Sweep: every entry component (box rect, whisker endpoints,
        // median line, outlier centers, outlier radius) interpolates
        // from its previous-layout value toward its current value.
        // With no previous (fresh mount), every Q/min/max/outlier
        // collapses to the entry's median axis at progress 0 and grows
        // out to its laid-out position at progress 1; outlier radius
        // scales 0 -> full at the entry's final outlier center,
        // mirroring Xy. When animation is opted out, progress pins to
        // `1.0` and every animated value equals the laid-out one.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animated: Vec<(EntryLayout, Vec<f32>)> = state
            .entries_layout
            .iter()
            .enumerate()
            .map(|(i, cur)| {
                let prev = state.previous_entries.get(i);
                animate_entry(cur, prev, progress)
            })
            .collect();

        match self.data.direction {
            Direction::Vertical => {
                self.draw_vertical(
                    &animated,
                    &mut frame,
                    color_offset,
                    palette,
                    background,
                    text_pair,
                    &seed,
                );
            }
            Direction::Horizontal => {
                self.draw_horizontal(
                    &animated,
                    &mut frame,
                    color_offset,
                    palette,
                    background,
                    text_pair,
                    &seed,
                );
            }
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_vertical(
        &self,
        animated: &[(EntryLayout, Vec<f32>)],
        frame: &mut Frame<Renderer>,
        color_offset: usize,
        palette: &crate::palette::Resolved,
        background: crate::core::Color,
        text_pair: crate::color::Pair,
        seed: &crate::palette::Seed,
    ) {
        for (i, (entry, (el, outlier_radii))) in self.data.entries.iter().zip(animated.iter()).enumerate() {
            // Box quartiles are interrelated — if any is non-finite, drop
            // the entry entirely rather than render a half-formed shape.
            // Outliers are per-point and handled below.
            if !entry_is_finite(el) {
                continue;
            }
            // Resolve color
            let base_color = if let Some(entry_color) = entry.color {
                entry_color.resolve(background, text_pair, seed, None)
            } else {
                palette.get(color_offset + i).resolve(background, text_pair, seed, None)
            };

            let fill_color = crate::core::Color {
                a: base_color.a * 0.3,
                ..base_color
            };

            let stroke_color = base_color;

            // Build box rect from animated quartile coords; q3 is higher
            // value -> lower y pixel. At progress 0 with no prev, q1/q3
            // both equal median_y, so the box collapses to a flat line
            // at the median axis.
            let rect_y = el.q3_y.min(el.q1_y);
            let rect_height = (el.q1_y - el.q3_y).abs();
            let rect = Rectangle {
                x: el.center_x - el.box_width / 2.0,
                y: rect_y,
                width: el.box_width,
                height: rect_height,
            };

            // Draw box (filled rect Q1-Q3)
            let box_path = Path::new(|b| {
                b.rectangle(Point::new(rect.x, rect.y), Size::new(rect.width, rect.height));
            });
            frame.fill(&box_path, fill_color);
            frame.stroke(&box_path, Stroke::default().with_width(1.5).with_color(stroke_color));

            // Median line
            let median_path = Path::new(|b| {
                b.move_to(Point::new(rect.x, el.median_y));
                b.line_to(Point::new(rect.x + rect.width, el.median_y));
            });
            frame.stroke(&median_path, Stroke::default().with_width(2.0).with_color(stroke_color));

            // Whiskers (vertical lines from box to min/max)
            let whisker_path = Path::new(|b| {
                // Upper whisker: Q3 to max
                b.move_to(Point::new(el.center_x, el.q3_y));
                b.line_to(Point::new(el.center_x, el.max_y));
                // Lower whisker: Q1 to min
                b.move_to(Point::new(el.center_x, el.q1_y));
                b.line_to(Point::new(el.center_x, el.min_y));
            });
            frame.stroke(
                &whisker_path,
                Stroke::default().with_width(1.0).with_color(stroke_color),
            );

            // Whisker caps
            let cap_width = el.box_width * 0.4;
            let cap_path = Path::new(|b| {
                // Max cap
                b.move_to(Point::new(el.center_x - cap_width / 2.0, el.max_y));
                b.line_to(Point::new(el.center_x + cap_width / 2.0, el.max_y));
                // Min cap
                b.move_to(Point::new(el.center_x - cap_width / 2.0, el.min_y));
                b.line_to(Point::new(el.center_x + cap_width / 2.0, el.min_y));
            });
            frame.stroke(&cap_path, Stroke::default().with_width(1.0).with_color(stroke_color));

            // Outliers
            for (&oy, &radius) in el.outlier_ys.iter().zip(outlier_radii.iter()) {
                if radius <= 0.0 || !oy.is_finite() {
                    continue;
                }
                let outlier_path = Path::new(|b| {
                    b.circle(Point::new(el.center_x, oy), radius);
                });
                frame.fill(&outlier_path, stroke_color);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_horizontal(
        &self,
        animated: &[(EntryLayout, Vec<f32>)],
        frame: &mut Frame<Renderer>,
        color_offset: usize,
        palette: &crate::palette::Resolved,
        background: crate::core::Color,
        text_pair: crate::color::Pair,
        seed: &crate::palette::Seed,
    ) {
        for (i, (entry, (el, outlier_radii))) in self.data.entries.iter().zip(animated.iter()).enumerate() {
            if !entry_is_finite(el) {
                continue;
            }
            // Resolve color
            let base_color = if let Some(entry_color) = entry.color {
                entry_color.resolve(background, text_pair, seed, None)
            } else {
                palette.get(color_offset + i).resolve(background, text_pair, seed, None)
            };

            let fill_color = crate::core::Color {
                a: base_color.a * 0.3,
                ..base_color
            };

            let stroke_color = base_color;

            // For horizontal: el.center_x is actually center_y,
            // el.min_y is min_x, etc.
            let center_y = el.center_x;
            let min_x = el.min_y;
            let q1_x = el.q1_y;
            let median_x = el.median_y;
            let q3_x = el.q3_y;
            let max_x = el.max_y;
            let box_height = el.box_width;

            // Build box rect from animated quartile coords. At progress
            // 0 with no prev, q1/q3 both equal median_x, so the box
            // collapses to a flat line at the median axis.
            let rect_x = q1_x.min(q3_x);
            let rect_width = (q3_x - q1_x).abs();
            let rect = Rectangle {
                x: rect_x,
                y: center_y - box_height / 2.0,
                width: rect_width,
                height: box_height,
            };

            // Draw box (filled rect Q1-Q3)
            let box_path = Path::new(|b| {
                b.rectangle(Point::new(rect.x, rect.y), Size::new(rect.width, rect.height));
            });
            frame.fill(&box_path, fill_color);
            frame.stroke(&box_path, Stroke::default().with_width(1.5).with_color(stroke_color));

            // Median line (vertical in horizontal mode)
            let median_path = Path::new(|b| {
                b.move_to(Point::new(median_x, rect.y));
                b.line_to(Point::new(median_x, rect.y + rect.height));
            });
            frame.stroke(&median_path, Stroke::default().with_width(2.0).with_color(stroke_color));

            // Whiskers (horizontal lines from box to min/max)
            let whisker_path = Path::new(|b| {
                // Left whisker: Q1 to min
                b.move_to(Point::new(q1_x, center_y));
                b.line_to(Point::new(min_x, center_y));
                // Right whisker: Q3 to max
                b.move_to(Point::new(q3_x, center_y));
                b.line_to(Point::new(max_x, center_y));
            });
            frame.stroke(
                &whisker_path,
                Stroke::default().with_width(1.0).with_color(stroke_color),
            );

            // Whisker caps (vertical in horizontal mode)
            let cap_height = box_height * 0.4;
            let cap_path = Path::new(|b| {
                // Min cap
                b.move_to(Point::new(min_x, center_y - cap_height / 2.0));
                b.line_to(Point::new(min_x, center_y + cap_height / 2.0));
                // Max cap
                b.move_to(Point::new(max_x, center_y - cap_height / 2.0));
                b.line_to(Point::new(max_x, center_y + cap_height / 2.0));
            });
            frame.stroke(&cap_path, Stroke::default().with_width(1.0).with_color(stroke_color));

            // Outliers
            for (&ox, &radius) in el.outlier_ys.iter().zip(outlier_radii.iter()) {
                if radius <= 0.0 || !ox.is_finite() {
                    continue;
                }
                let outlier_path = Path::new(|b| {
                    b.circle(Point::new(ox, center_y), radius);
                });
                frame.fill(&outlier_path, stroke_color);
            }
        }
    }
}

/// Whether all the box-defining coordinates of an entry are finite. Outlier
/// coordinates are excluded — they're checked individually so a single NaN
/// outlier doesn't suppress the whole entry.
fn entry_is_finite(el: &EntryLayout) -> bool {
    el.center_x.is_finite()
        && el.min_y.is_finite()
        && el.q1_y.is_finite()
        && el.median_y.is_finite()
        && el.q3_y.is_finite()
        && el.max_y.is_finite()
        && el.box_width.is_finite()
}

/// Computes the on-screen entry layout at the current sweep progress
/// and the matching per-outlier radius. With a `prev` layout (data
/// change), every scalar lerps linearly from `prev` to `cur`; outliers
/// shared with `prev` lerp positions and keep their full radius, while
/// any extra current-only outliers grow at their final positions
/// mirroring the mount path. Without a `prev` layout (fresh mount),
/// every Q/min/max/outlier collapses to the entry's median axis at
/// progress 0 and reaches the laid-out value at progress 1; outlier
/// radius scales `OUTLIER_RADIUS * progress`.
///
/// At `progress == 1.0` the result equals `(cur, full radii)` exactly
/// in every branch, so disabling animation reproduces today's geometry.
fn animate_entry(cur: &EntryLayout, prev: Option<&EntryLayout>, progress: f32) -> (EntryLayout, Vec<f32>) {
    if let Some(prev) = prev {
        let lerp = |a: f32, b: f32| a + (b - a) * progress;
        let prev_outliers = prev.outlier_ys.len();
        let median_y = lerp(prev.median_y, cur.median_y);
        let outlier_ys: Vec<f32> = cur
            .outlier_ys
            .iter()
            .enumerate()
            .map(|(i, &cur_o)| {
                if i < prev_outliers {
                    lerp(prev.outlier_ys[i], cur_o)
                } else {
                    cur_o
                }
            })
            .collect();
        let outlier_radii: Vec<f32> = (0..cur.outlier_ys.len())
            .map(|i| {
                if i < prev_outliers {
                    OUTLIER_RADIUS
                } else {
                    OUTLIER_RADIUS * progress
                }
            })
            .collect();
        let entry = EntryLayout {
            center_x: lerp(prev.center_x, cur.center_x),
            min_y: lerp(prev.min_y, cur.min_y),
            q1_y: lerp(prev.q1_y, cur.q1_y),
            median_y,
            q3_y: lerp(prev.q3_y, cur.q3_y),
            max_y: lerp(prev.max_y, cur.max_y),
            box_width: lerp(prev.box_width, cur.box_width),
            outlier_ys,
        };
        (entry, outlier_radii)
    } else {
        let m = cur.median_y;
        let collapse = |v: f32| m + (v - m) * progress;
        let entry = EntryLayout {
            center_x: cur.center_x,
            min_y: collapse(cur.min_y),
            q1_y: collapse(cur.q1_y),
            median_y: cur.median_y,
            q3_y: collapse(cur.q3_y),
            max_y: collapse(cur.max_y),
            box_width: cur.box_width,
            outlier_ys: cur.outlier_ys.clone(),
        };
        let outlier_radii: Vec<f32> = (0..cur.outlier_ys.len()).map(|_| OUTLIER_RADIUS * progress).collect();
        (entry, outlier_radii)
    }
}

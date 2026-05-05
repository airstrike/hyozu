use super::Plane;
use crate::animation;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size};
use crate::data::Datum;
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Box plot layout in pixel coordinates
#[derive(Clone)]
pub struct BoxLayout {
    pub min_y: f32,
    pub q1_y: f32,
    pub median_y: f32,
    pub q3_y: f32,
    pub max_y: f32,
}

/// Layout for a single violin entry
#[derive(Clone)]
pub struct EntryLayout {
    /// Right (or top) edge points of the mirrored shape
    pub right_points: Vec<Point>,
    /// Left (or bottom) edge points -- mirror of right
    pub left_points: Vec<Point>,
    /// Center x position
    pub center_x: f32,
    /// Box stats pixel positions (if show_box)
    pub box_layout: Option<BoxLayout>,
}

/// State for Violin -- stores pre-calculated entry layouts
pub struct State {
    pub entries: Vec<EntryLayout>,
    /// Per-entry layout snapshot from the most recent layout before the
    /// current one, captured by [`crate::chart::Chart::diff`] when data
    /// changes so the next sweep can interpolate every component
    /// (silhouette outline, box stats) from previous to current.
    /// Empty on a fresh mount, in which case the animation collapses to
    /// a center-axis-anchored grow-out.
    pub previous_entries: Vec<EntryLayout>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// A Violin series that renders violin charts.
pub struct Violin<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::violin::Violin,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Violin<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Violin borrowing data
    pub fn new(data: &'a crate::mark::violin::Violin) -> Self {
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

    /// Returns the initial tree state for this Violin
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                entries: Vec::new(),
                previous_entries: Vec::new(),
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Violin state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the violin -- compute mirrored density shape positions
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();

        let num_entries = self.data.entries.len();
        if num_entries == 0 {
            state.entries.clear();
            return Node::new(Size::ZERO);
        }

        let total_width = plane.bounds.width;
        let bin_width = total_width / num_entries as f32;
        let max_half_width = (bin_width * self.data.width) / 2.0;

        state.entries.clear();

        for (i, entry) in self.data.entries.iter().enumerate() {
            let center_x = plane.to_pixel(Datum::x(i as f64)).x;

            if entry.density.is_empty() {
                state.entries.push(EntryLayout {
                    right_points: Vec::new(),
                    left_points: Vec::new(),
                    center_x,
                    box_layout: None,
                });
                continue;
            }

            // Find max density for normalization
            let max_density = entry.density.iter().map(|(_, d)| *d).fold(0.0_f64, f64::max);

            if max_density <= 0.0 {
                state.entries.push(EntryLayout {
                    right_points: Vec::new(),
                    left_points: Vec::new(),
                    center_x,
                    box_layout: None,
                });
                continue;
            }

            // Build mirrored shape
            let mut right_points = Vec::new();
            let mut left_points = Vec::new();

            for &(value, density) in &entry.density {
                let y = plane.to_pixel(Datum::new(i as f64, value)).y;
                let half_w = (density / max_density) as f32 * max_half_width;

                right_points.push(Point::new(center_x + half_w, y));
                left_points.push(Point::new(center_x - half_w, y));
            }

            // Box stats layout
            let box_layout = if self.data.show_box {
                entry.stats.as_ref().map(|stats| BoxLayout {
                    min_y: plane.to_pixel(Datum::new(i as f64, stats.min)).y,
                    q1_y: plane.to_pixel(Datum::new(i as f64, stats.q1)).y,
                    median_y: plane.to_pixel(Datum::new(i as f64, stats.median)).y,
                    q3_y: plane.to_pixel(Datum::new(i as f64, stats.q3)).y,
                    max_y: plane.to_pixel(Datum::new(i as f64, stats.max)).y,
                })
            } else {
                None
            };

            state.entries.push(EntryLayout {
                right_points,
                left_points,
                center_x,
                box_layout,
            });
        }

        Node::new(Size::ZERO)
    }

    /// Draws the violin chart
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

        if state.entries.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        let layout_bounds = layout.bounds();
        let mut fill_frame = Frame::new(renderer, layout_bounds.size());
        let mut stroke_frame = Frame::new(renderer, layout_bounds.size());

        // Sweep: every entry's silhouette outline (left + right outline
        // points) and box overlay stats interpolate from their
        // previous-layout values toward their current values. With no
        // previous (fresh mount), every outline point collapses to the
        // entry's vertical center axis at progress 0 and reaches its
        // laid-out half-width at progress 1; box stats collapse to the
        // entry's median axis. When animation is opted out, progress
        // pins to `1.0` and the result equals the laid-out geometry
        // exactly.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animated: Vec<EntryLayout> = state
            .entries
            .iter()
            .enumerate()
            .map(|(i, cur)| animate_entry(cur, state.previous_entries.get(i), progress))
            .collect();

        for (entry_idx, (entry_data, entry_layout)) in self.data.entries.iter().zip(animated.iter()).enumerate() {
            if entry_layout.right_points.is_empty() {
                continue;
            }

            let base_color = if let Some(c) = entry_data.color {
                c.resolve(background, text_pair, &seed, None)
            } else {
                palette
                    .get(color_offset + entry_idx)
                    .resolve(background, text_pair, &seed, None)
            };

            let fill_color = crate::core::Color {
                a: base_color.a * 0.4,
                ..base_color
            };

            // Violin shape: forward right, backward left
            let shape = Path::new(|b| {
                if let Some(first) = entry_layout.right_points.first() {
                    b.move_to(*first);
                }
                for p in entry_layout.right_points.iter().skip(1) {
                    b.line_to(*p);
                }
                for p in entry_layout.left_points.iter().rev() {
                    b.line_to(*p);
                }
                b.close();
            });

            fill_frame.fill(&shape, fill_color);
            stroke_frame.stroke(&shape, Stroke::default().with_width(1.0).with_color(base_color));

            // Mini box plot overlay
            if let Some(box_layout) = &entry_layout.box_layout {
                let box_half = 3.0; // thin box
                let cx = entry_layout.center_x;

                // Whisker line (min to max)
                let whisker = Path::new(|b| {
                    b.move_to(Point::new(cx, box_layout.min_y));
                    b.line_to(Point::new(cx, box_layout.max_y));
                });
                stroke_frame.stroke(&whisker, Stroke::default().with_width(1.0).with_color(base_color));

                // Box (Q1 to Q3)
                let q1 = box_layout.q1_y;
                let q3 = box_layout.q3_y;
                let top = q1.min(q3);
                let height = (q1 - q3).abs();

                let box_path = Path::new(|b| {
                    b.rectangle(Point::new(cx - box_half, top), Size::new(box_half * 2.0, height));
                });
                fill_frame.fill(&box_path, base_color);

                // Median dot (white)
                let median_dot = Path::new(|b| {
                    b.circle(Point::new(cx, box_layout.median_y), 2.5);
                });
                fill_frame.fill(&median_dot, crate::core::Color::WHITE);
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
    }
}

/// Computes the on-screen entry layout at the current sweep progress.
/// With a `prev` layout (data change), every outline point and box
/// stat lerps linearly from `prev` to `cur`; outline points without a
/// `prev` counterpart (cur has more samples than prev) grow from
/// `cur.center_x` mirroring the mount path. Without a `prev` layout
/// (fresh mount), every outline point's x collapses to `cur.center_x`
/// at progress 0 and reaches its laid-out x at progress 1; box stats
/// collapse to the entry's median axis.
///
/// At `progress == 1.0` the result equals `cur` exactly in every
/// branch, so disabling animation reproduces today's geometry.
fn animate_entry(cur: &EntryLayout, prev: Option<&EntryLayout>, progress: f32) -> EntryLayout {
    let cx = cur.center_x;
    let lerp = |a: f32, b: f32| a + (b - a) * progress;

    let collapse_outline_to_center = |pts: &[Point]| -> Vec<Point> {
        pts.iter()
            .map(|p| Point::new(cx + (p.x - cx) * progress, p.y))
            .collect()
    };

    let lerp_outline = |prev_pts: &[Point], cur_pts: &[Point]| -> Vec<Point> {
        cur_pts
            .iter()
            .enumerate()
            .map(|(i, &cp)| {
                if let Some(pp) = prev_pts.get(i) {
                    Point::new(lerp(pp.x, cp.x), lerp(pp.y, cp.y))
                } else {
                    // Cur has more samples than prev: grow extras from
                    // the entry's center axis, matching the mount path.
                    Point::new(cx + (cp.x - cx) * progress, cp.y)
                }
            })
            .collect()
    };

    let (right_points, left_points, box_layout) = match prev {
        Some(prev) => {
            let right_points = lerp_outline(&prev.right_points, &cur.right_points);
            let left_points = lerp_outline(&prev.left_points, &cur.left_points);
            let box_layout = match (&prev.box_layout, &cur.box_layout) {
                (Some(prev_box), Some(cur_box)) => Some(BoxLayout {
                    min_y: lerp(prev_box.min_y, cur_box.min_y),
                    q1_y: lerp(prev_box.q1_y, cur_box.q1_y),
                    median_y: lerp(prev_box.median_y, cur_box.median_y),
                    q3_y: lerp(prev_box.q3_y, cur_box.q3_y),
                    max_y: lerp(prev_box.max_y, cur_box.max_y),
                }),
                (None, Some(cur_box)) => {
                    let m = cur_box.median_y;
                    let collapse = |v: f32| m + (v - m) * progress;
                    Some(BoxLayout {
                        min_y: collapse(cur_box.min_y),
                        q1_y: collapse(cur_box.q1_y),
                        median_y: cur_box.median_y,
                        q3_y: collapse(cur_box.q3_y),
                        max_y: collapse(cur_box.max_y),
                    })
                }
                (Some(_), None) => None,
                (None, None) => None,
            };
            (right_points, left_points, box_layout)
        }
        None => {
            let right_points = collapse_outline_to_center(&cur.right_points);
            let left_points = collapse_outline_to_center(&cur.left_points);
            let box_layout = cur.box_layout.as_ref().map(|cur_box| {
                let m = cur_box.median_y;
                let collapse = |v: f32| m + (v - m) * progress;
                BoxLayout {
                    min_y: collapse(cur_box.min_y),
                    q1_y: collapse(cur_box.q1_y),
                    median_y: cur_box.median_y,
                    q3_y: collapse(cur_box.q3_y),
                    max_y: collapse(cur_box.max_y),
                }
            });
            (right_points, left_points, box_layout)
        }
    };

    EntryLayout {
        right_points,
        left_points,
        center_x: cur.center_x,
        box_layout,
    }
}

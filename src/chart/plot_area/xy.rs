use super::{Plane, geo};
use crate::animation;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size};
use crate::line::marker::Shape;
use crate::mark::xy::CoordKind;
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text;
use crate::widget::renderer::geometry;

/// State for Xy — stores pixel positions and resolved per-point sizes
pub struct State {
    /// Pixel coordinates for each point
    pub pixel_points: Vec<Point>,
    /// Resolved marker diameter per point. Computed in `layout` so the
    /// `size_by` encoding only re-runs when the layout invalidates, not on
    /// every repaint. When no encoding is configured this is empty and the
    /// renderer falls back to `marker.size` per point.
    pub resolved_sizes: Vec<f32>,
    /// Pixel coordinates from the most recent layout before the current
    /// one, captured by [`crate::chart::Chart::diff`] when data changes
    /// so the next sweep can interpolate from previous positions to
    /// current positions. Empty on a fresh mount, in which case the
    /// animation collapses to a 0 → final-size scale-in at the final
    /// position.
    pub previous_pixel_points: Vec<Point>,
    /// Resolved per-marker diameters from the previous layout, paired
    /// 1:1 with `previous_pixel_points`. Captured by
    /// [`crate::chart::Chart::diff`] so the size-by encoding's
    /// pixel-space output lerps alongside positions.
    pub previous_resolved_sizes: Vec<f32>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// An Xy series that renders scatter charts.
pub struct Xy<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(crate) data: &'a crate::mark::xy::Xy,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Xy<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Xy borrowing data
    pub fn new(data: &'a crate::mark::xy::Xy) -> Self {
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

    /// Returns the initial tree state for this Xy
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                pixel_points: Vec::new(),
                resolved_sizes: Vec::new(),
                previous_pixel_points: Vec::new(),
                previous_resolved_sizes: Vec::new(),
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Xy state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the scatter — transform data to pixel coordinates and resolve
    /// per-point marker diameters from the `size_by` encoding (if any).
    /// Both vectors are stored on `State` so `draw` is a pure read.
    ///
    /// `geo_plane` is consulted only when `self.data.coord_kind ==
    /// CoordKind::Geo`. Geo-coord Xy with no geo plane (misconfiguration)
    /// emits an empty `pixel_points`, which makes `draw` short-circuit.
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        plane: &Plane,
        geo_plane: Option<&geo::Plane>,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();

        state.pixel_points = match self.data.coord_kind {
            CoordKind::Cartesian => self.data.points.iter().map(|p| plane.to_pixel(*p)).collect(),
            CoordKind::Geo => match geo_plane {
                Some(plane) => self
                    .data
                    .points
                    .iter()
                    .map(|p| plane.project_point(p.x as f32, p.y as f32))
                    .collect(),
                None => Vec::new(),
            },
        };

        // Resolve per-point sizes once per layout. Encoding misses (None)
        // and the no-encoding case both fall back to `marker.size`, so the
        // renderer can index `resolved_sizes` unconditionally without a
        // per-point Option check.
        let fallback = self.data.marker.size;
        state.resolved_sizes = match &self.data.size_by {
            Some(enc) => enc
                .resolve_size(&self.data.points)
                .into_iter()
                .map(|s| s.unwrap_or(fallback))
                .collect(),
            None => vec![fallback; self.data.points.len()],
        };

        Node::new(Size::ZERO)
    }

    /// Draws the scatter chart
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

        if state.pixel_points.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        let layout_bounds = layout.bounds();

        let color = if let Some(data_color) = self.data.color {
            data_color.resolve(background, text_pair, &seed, None)
        } else {
            palette.get(color_offset).resolve(background, text_pair, &seed, None)
        };

        let marker_config = &self.data.marker;
        let mut frame = Frame::new(renderer, layout_bounds.size());

        // Sweep: with a previous-layout snapshot (data change), each
        // marker's center and diameter lerp from prev to cur. Markers
        // appended past the prev length (newly-added points) anchor
        // their position at `pixel_points[0]` so they emerge from the
        // path's leading edge, and use a `0` previous size so the new
        // marker scales in like a fresh mount. Without a snapshot
        // (fresh mount), each marker stays at its final position and
        // its diameter scales from `0` to `cur`. With `animate = false`
        // progress pins to `1.0` and the animated geometry equals the
        // laid-out values exactly.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let has_prev = !state.previous_pixel_points.is_empty();
        let prev_anchor = state.pixel_points[0];

        // Per-point sizes were resolved in layout and cached on state.
        for (i, pixel_point) in state.pixel_points.iter().enumerate() {
            // Non-finite pixel or size = gap; skip the marker.
            if !pixel_point.x.is_finite() || !pixel_point.y.is_finite() {
                continue;
            }
            let cur_size = state.resolved_sizes.get(i).copied().unwrap_or(marker_config.size);
            if !cur_size.is_finite() {
                continue;
            }
            let (center, size) = if has_prev {
                let prev_pt = state.previous_pixel_points.get(i).copied().unwrap_or(prev_anchor);
                let prev_size = state.previous_resolved_sizes.get(i).copied().unwrap_or(0.0);
                animate_marker(*pixel_point, cur_size, Some(prev_pt), Some(prev_size), progress)
            } else {
                animate_marker(*pixel_point, cur_size, None, None, progress)
            };

            if size <= 0.0 {
                continue;
            }
            let half = size / 2.0;

            let path = Path::new(|builder| match marker_config.shape {
                Shape::Circle => {
                    builder.circle(center, half);
                }
                Shape::Square => {
                    builder.rectangle(
                        Point::new(center.x - half, center.y - half),
                        crate::core::Size::new(size, size),
                    );
                }
                Shape::Diamond => {
                    builder.move_to(Point::new(center.x, center.y - half));
                    builder.line_to(Point::new(center.x + half, center.y));
                    builder.line_to(Point::new(center.x, center.y + half));
                    builder.line_to(Point::new(center.x - half, center.y));
                    builder.close();
                }
                Shape::Triangle => {
                    builder.move_to(Point::new(center.x, center.y - half));
                    builder.line_to(Point::new(center.x + half, center.y + half));
                    builder.line_to(Point::new(center.x - half, center.y + half));
                    builder.close();
                }
                Shape::TriangleDown => {
                    builder.move_to(Point::new(center.x, center.y + half));
                    builder.line_to(Point::new(center.x + half, center.y - half));
                    builder.line_to(Point::new(center.x - half, center.y - half));
                    builder.close();
                }
                Shape::Cross => {
                    let arm = half * 0.3;
                    builder.move_to(Point::new(center.x - arm, center.y - half));
                    builder.line_to(Point::new(center.x + arm, center.y - half));
                    builder.line_to(Point::new(center.x + arm, center.y - arm));
                    builder.line_to(Point::new(center.x + half, center.y - arm));
                    builder.line_to(Point::new(center.x + half, center.y + arm));
                    builder.line_to(Point::new(center.x + arm, center.y + arm));
                    builder.line_to(Point::new(center.x + arm, center.y + half));
                    builder.line_to(Point::new(center.x - arm, center.y + half));
                    builder.line_to(Point::new(center.x - arm, center.y + arm));
                    builder.line_to(Point::new(center.x - half, center.y + arm));
                    builder.line_to(Point::new(center.x - half, center.y - arm));
                    builder.line_to(Point::new(center.x - arm, center.y - arm));
                    builder.close();
                }
                Shape::X => {
                    let diag = half * 0.707;
                    builder.move_to(Point::new(center.x - diag, center.y - diag));
                    builder.line_to(Point::new(center.x + diag, center.y + diag));
                    builder.move_to(Point::new(center.x + diag, center.y - diag));
                    builder.line_to(Point::new(center.x - diag, center.y + diag));
                }
            });

            // Fill (except X shape)
            let marker_color = if let Some(color_spec) = marker_config.color {
                color_spec.resolve(background, text_pair, &seed, None)
            } else {
                color
            };

            // Apply the per-mark opacity multiplier on top of the resolved
            // fill alpha. Stroke keeps its full alpha so the outline reads
            // sharply over translucent fills (matches the bubble-map
            // convention this Xy path replaces).
            let fill_color = crate::core::Color {
                a: marker_color.a * self.data.opacity.clamp(0.0, 1.0),
                ..marker_color
            };

            if marker_config.shape != Shape::X {
                frame.fill(&path, fill_color);
            }

            // Stroke
            if let Some(stroke_color_spec) = marker_config.stroke {
                let stroke_color = stroke_color_spec.resolve(background, text_pair, &seed, None);
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_width(marker_config.stroke_width)
                        .with_color(stroke_color),
                );
            } else if marker_config.shape == Shape::X {
                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_width(marker_config.stroke_width.max(2.0))
                        .with_color(marker_color),
                );
            }
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }
}

/// Returns a marker's animated center point and diameter at the current
/// sweep progress.
///
/// With both `prev_pt` and `prev_size` provided (data change), the
/// center lerps component-wise from `prev_pt` to `cur_pt` and the
/// diameter lerps from `prev_size` to `cur_size`. Newly-added markers
/// (no entry at this index in the previous snapshot) pass `prev_pt =
/// Some(cur[0])` and `prev_size = Some(0.0)` so they emerge from the
/// path's leading edge and scale in like a fresh mount.
///
/// With neither provided (fresh mount), the center stays at `cur_pt`
/// and the diameter scales from `0` to `cur_size`.
///
/// At `progress == 1.0` the result equals `(cur_pt, cur_size)` exactly
/// in every branch, so disabling animation reproduces today's geometry.
fn animate_marker(
    cur_pt: Point,
    cur_size: f32,
    prev_pt: Option<Point>,
    prev_size: Option<f32>,
    progress: f32,
) -> (Point, f32) {
    if let (Some(prev_pt), Some(prev_size)) = (prev_pt, prev_size) {
        let x = prev_pt.x + (cur_pt.x - prev_pt.x) * progress;
        let y = prev_pt.y + (cur_pt.y - prev_pt.y) * progress;
        let size = prev_size + (cur_size - prev_size) * progress;
        (Point { x, y }, size)
    } else {
        (cur_pt, cur_size * progress)
    }
}

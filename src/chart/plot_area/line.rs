use super::Plane;
use crate::animation;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::line::LineStyle;
use crate::widget::canvas::{Frame, LineCap, LineDash, Path, Stroke, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

use crate::core::{Point, Rectangle};
use crate::line::label::{Position, Show};
use crate::line::marker;

/// State for a Line - stores positioned line points and label info
pub struct State {
    /// Pixel coordinates of line points (relative to plane origin)
    pub pixel_points: Vec<Point>,
    /// Label texts for points that have labels
    pub label_texts: Vec<String>,
    /// Resolved positions for each label (after hit-testing)
    pub label_positions: Vec<Position>,
    /// Pixel rectangles for each label (for drawing)
    pub label_rects: Vec<Rectangle>,
    /// Pixel points from the most recent layout before the current one,
    /// captured by [`crate::chart::Chart::diff`] when data changes so the
    /// next sweep can interpolate from previous positions to current
    /// positions. Empty on a fresh mount, in which case the animation
    /// collapses to a left-to-right path sweep.
    pub previous_pixel_points: Vec<Point>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// A Line series that renders connected line segments.
///
/// Like Guide and Title, this is widget-like but doesn't implement Widget.
pub struct Line<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(crate) data: &'a crate::line::Line,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Line<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Line borrowing data
    pub fn new(data: &'a crate::line::Line) -> Self {
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

    /// Returns the initial tree state for this Line
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                pixel_points: Vec::new(),
                label_texts: Vec::new(),
                label_positions: Vec::new(),
                label_rects: Vec::new(),
                previous_pixel_points: Vec::new(),
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Line state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the line - transforms data coordinates to pixel coordinates
    /// and creates child nodes for labels
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        plane: &Plane,
        obstacles: &[Rectangle],
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();

        // Transform all data points to pixel coordinates
        state.pixel_points = self.data.points.iter().map(|p| plane.to_pixel(*p)).collect();

        // Build label info if configured
        state.label_texts.clear();
        state.label_positions.clear();
        state.label_rects.clear();

        if let Some(label_config) = &self.data.label {
            let label_size = label_config.text.resolved_size(12.0);
            let num_points = state.pixel_points.len();

            // Find min/max Y indices for MinMax modes
            let minmax_indices: Vec<usize> = match label_config.show {
                Show::MinMaxFirst | Show::MinMaxAll | Show::MinMaxLast => {
                    let (min_val, max_val) = self
                        .data
                        .points
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), p| {
                            (min.min(p.y), max.max(p.y))
                        });

                    match label_config.show {
                        Show::MinMaxFirst => {
                            let min_idx = self.data.points.iter().position(|p| p.y == min_val);
                            let max_idx = self.data.points.iter().position(|p| p.y == max_val);
                            [min_idx, max_idx].into_iter().flatten().collect()
                        }
                        Show::MinMaxLast => {
                            let min_idx = self.data.points.iter().rposition(|p| p.y == min_val);
                            let max_idx = self.data.points.iter().rposition(|p| p.y == max_val);
                            [min_idx, max_idx].into_iter().flatten().collect()
                        }
                        Show::MinMaxAll => self
                            .data
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

            // Collect line segments for hit-testing
            let segments: Vec<(Point, Point)> = state.pixel_points.windows(2).map(|w| (w[0], w[1])).collect();

            for (idx, (pixel_point, data_point)) in state.pixel_points.iter().zip(self.data.points.iter()).enumerate() {
                // Non-finite coordinates are gaps — no label, no placement
                // search (which would otherwise let a NaN rect bypass the
                // bounds check and crash `fill_text`).
                if !pixel_point.x.is_finite() || !pixel_point.y.is_finite() {
                    continue;
                }
                // Check if this point should show a label
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

                let label_text = (label_config.format)(data_point.y);

                // Estimate label bounds (approximate: ~0.6 * size per char)
                let char_width = label_size * 0.6;
                let label_width = label_text.len() as f32 * char_width;
                let label_height = label_size * 1.2;

                // Resolve position and rect - either auto with hit-testing or fixed
                let (resolved_position, label_rect) = match label_config.position {
                    Position::Auto => place_label(
                        *pixel_point,
                        label_width,
                        label_height,
                        &segments,
                        &state.label_rects,
                        obstacles,
                        Some(plane.bounds),
                    ),
                    other => {
                        // For fixed positions, use pathfinding too to avoid obstacles
                        let start_rect = compute_label_rect(*pixel_point, label_width, label_height, other);
                        let rect = search_from_position(
                            start_rect,
                            &segments,
                            &state.label_rects,
                            obstacles,
                            Some(plane.bounds),
                            10,
                        )
                        .map(|(rect, _)| rect)
                        .unwrap_or(start_rect);
                        // Guarantee visibility at edges even if no valid slot exists.
                        (other, clamp_to_bounds(rect, plane.bounds))
                    }
                };

                state.label_texts.push(label_text);
                state.label_positions.push(resolved_position);
                state.label_rects.push(label_rect);
            }
        }

        // Line takes no space - it's rendered within the plane
        Node::new(Size::ZERO)
    }

    /// Draws the line using pre-calculated pixel coordinates
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

        if state.pixel_points.len() < 2 {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        // Get the layout bounds to offset the line to its actual screen position
        let layout_bounds = layout.bounds();

        // Determine color for this line
        let color = if let Some(data_color) = self.data.color {
            data_color.resolve(background, text_pair, &seed, None)
        } else {
            palette.get(color_offset).resolve(background, text_pair, &seed, None)
        };

        // Sweep: with a previous-layout snapshot (data change), each
        // vertex lerps component-wise from its prev position to its cur
        // position; vertices appended past the prev length anchor at
        // the path's leading edge (`cur[0]`) so they emerge from the
        // start. Without one (fresh mount), the polyline terminates at
        // `floor(progress * len)` full vertices plus a partial segment
        // toward the next vertex, producing a left-to-right path
        // sweep. With `animate = false` progress pins to `1.0` and the
        // animated points equal `state.pixel_points` exactly,
        // reproducing today's geometry.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animating = progress < 1.0 - f32::EPSILON;
        let animated_pixel_points = animate_pixel_points(&state.pixel_points, &state.previous_pixel_points, progress);

        if animated_pixel_points.len() < 2 {
            return;
        }

        let mut frame = Frame::new(renderer, layout_bounds.size());

        // Build a path from all points, breaking the line at any non-finite
        // coordinate. A NaN y (from a missing or error cell flowing through
        // the series unchanged) used to panic lyon_path's `move_to`; now it
        // ends the current segment and the next finite point starts a fresh
        // one.
        let path = Path::new(|builder| {
            let mut in_segment = false;
            for point in &animated_pixel_points {
                let finite = point.x.is_finite() && point.y.is_finite();
                if !finite {
                    in_segment = false;
                    continue;
                }
                let p = crate::core::Point::new(point.x, point.y);
                if in_segment {
                    builder.line_to(p);
                } else {
                    builder.move_to(p);
                    in_segment = true;
                }
            }
        });

        let dash_stack = dash_segments(&self.data.style);
        let mut stroke = Stroke::default().with_width(self.data.width).with_color(color);
        // Dash overlays suppressed mid-sweep so dashes don't crawl
        // along a line that's still extending into place.
        if !animating {
            stroke.line_dash = LineDash {
                segments: dash_stack,
                offset: 0,
            };
            if matches!(self.data.style, LineStyle::Dotted) {
                stroke = stroke.with_line_cap(LineCap::Round);
            }
        }

        frame.stroke(&path, stroke);

        // Draw the geometry at the layout position
        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });

        if let Some(marker_config) = &self.data.marker
            && !animating
        {
            let mut marker_frame = Frame::new(renderer, layout_bounds.size());
            draw_markers(
                &mut marker_frame,
                &state.pixel_points,
                &self.data.points,
                marker_config,
                color,
                background,
                text_pair,
                &seed,
            );
            let marker_geometry = marker_frame.into_geometry();
            renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
                renderer.draw_geometry(marker_geometry);
            });
        }

        // Draw data labels from state — suppressed mid-sweep so they
        // don't pop in over points the path hasn't grown to yet.
        if let Some(label_config) = &self.data.label
            && !animating
        {
            let label_size = label_config
                .text
                .resolved_size(theme.data_label_text().resolved_size(12.0));

            // Resolve label color (defaults to line color)
            let label_color = if let Some(label_color_spec) = label_config.color {
                label_color_spec.resolve(background, text_pair, &seed, None)
            } else {
                color
            };

            // Resolve the label font once per series — family, weight, and
            // style all flow from the label's `text::Style`, falling back to
            // the theme's data-label default for anything unset.
            let label_font = label_config
                .text
                .resolved_font(theme.data_label_text().resolved_font(theme.font()));

            let label_fill_color = label_config
                .fill
                .map(|spec| spec.resolve(background, text_pair, &seed, None));

            let mut label_frame = Frame::new(renderer, layout_bounds.size());

            for ((label_rect, label_text), resolved_pos) in state
                .label_rects
                .iter()
                .zip(state.label_texts.iter())
                .zip(state.label_positions.iter())
            {
                // Background fill, if configured. The placement search already
                // produced a tight rect for the text, so we just use it.
                if let Some(fill_color) = label_fill_color {
                    let fill_path = Path::new(|b| {
                        b.rectangle(
                            Point::new(label_rect.x, label_rect.y),
                            crate::core::Size::new(label_rect.width, label_rect.height),
                        );
                    });
                    label_frame.fill(&fill_path, fill_color);
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

                label_frame.fill_text(CanvasText {
                    content: label_text.clone(),
                    position: Point::new(anchor_x, anchor_y),
                    color: label_color,
                    size: label_size.into(),
                    font: label_font,
                    align_x: align_x.into(),
                    align_y,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }

            let label_geometry = label_frame.into_geometry();
            renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
                renderer.draw_geometry(label_geometry);
            });
        }
    }
}

/// Returns the dash pattern segments for a [`LineStyle`].
///
/// Borrows `Custom`'s segment vec; the returned slice is only valid for as
/// long as the borrowed `style`.
pub(super) fn dash_segments(style: &LineStyle) -> &[f32] {
    match style {
        LineStyle::Solid => &[],
        LineStyle::Dashed => &[8.0, 4.0],
        LineStyle::Dotted => &[1.0, 3.0],
        LineStyle::Custom { segments } => segments.as_slice(),
    }
}

/// Draws the markers for a polyline series into `frame`.
///
/// Used by both line and area renderers so marker semantics (Show filter,
/// shape paths, fill+stroke layering) stay identical across mark types.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw_markers<R>(
    frame: &mut Frame<R>,
    pixel_points: &[Point],
    raw_points: &[crate::data::Datum],
    config: &marker::Marker,
    base_color: crate::core::Color,
    background: crate::core::Color,
    text_pair: crate::color::Pair,
    seed: &crate::palette::Seed,
) where
    R: geometry::Renderer,
{
    use marker::{Shape, Show};

    let marker_color = config
        .color
        .map(|spec| spec.resolve(background, text_pair, seed, None))
        .unwrap_or(base_color);

    let n = pixel_points.len();
    if n == 0 {
        return;
    }

    let minmax_indices: Vec<usize> = match config.show {
        Show::MinMaxFirst | Show::MinMaxAll | Show::MinMaxLast => {
            let (min_val, max_val) = raw_points
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                    (lo.min(p.y), hi.max(p.y))
                });
            match config.show {
                Show::MinMaxFirst => {
                    let lo = raw_points.iter().position(|p| p.y == min_val);
                    let hi = raw_points.iter().position(|p| p.y == max_val);
                    [lo, hi].into_iter().flatten().collect()
                }
                Show::MinMaxLast => {
                    let lo = raw_points.iter().rposition(|p| p.y == min_val);
                    let hi = raw_points.iter().rposition(|p| p.y == max_val);
                    [lo, hi].into_iter().flatten().collect()
                }
                Show::MinMaxAll => raw_points
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

    for (idx, point) in pixel_points.iter().enumerate() {
        // Non-finite pixel = gap; skip the marker rather than feeding NaN
        // coordinates into the shape path.
        if !point.x.is_finite() || !point.y.is_finite() {
            continue;
        }
        let visible = match config.show {
            Show::Any => true,
            Show::FirstOnly => idx == 0,
            Show::LastOnly => idx == n - 1,
            Show::FirstAndLast => idx == 0 || idx == n - 1,
            Show::MinMaxFirst | Show::MinMaxAll | Show::MinMaxLast => minmax_indices.contains(&idx),
        };
        if !visible {
            continue;
        }

        let path = build_marker_path(config.shape, *point, config.size);

        if config.shape != Shape::X {
            frame.fill(&path, marker_color);
        }

        if let Some(stroke_spec) = config.stroke {
            let stroke_color = stroke_spec.resolve(background, text_pair, seed, None);
            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(config.stroke_width)
                    .with_color(stroke_color),
            );
        } else if config.shape == Shape::X {
            frame.stroke(
                &path,
                Stroke::default()
                    .with_width(config.stroke_width.max(2.0))
                    .with_color(marker_color),
            );
        }
    }
}

/// Builds the path for a single marker shape centered on `point`.
fn build_marker_path(shape: marker::Shape, point: Point, size: f32) -> Path {
    use marker::Shape;
    let half = size / 2.0;
    Path::new(|builder| match shape {
        Shape::Circle => {
            builder.circle(point, half);
        }
        Shape::Square => {
            builder.rectangle(
                Point::new(point.x - half, point.y - half),
                crate::core::Size::new(size, size),
            );
        }
        Shape::Diamond => {
            builder.move_to(Point::new(point.x, point.y - half));
            builder.line_to(Point::new(point.x + half, point.y));
            builder.line_to(Point::new(point.x, point.y + half));
            builder.line_to(Point::new(point.x - half, point.y));
            builder.close();
        }
        Shape::Triangle => {
            builder.move_to(Point::new(point.x, point.y - half));
            builder.line_to(Point::new(point.x + half, point.y + half));
            builder.line_to(Point::new(point.x - half, point.y + half));
            builder.close();
        }
        Shape::TriangleDown => {
            builder.move_to(Point::new(point.x, point.y + half));
            builder.line_to(Point::new(point.x + half, point.y - half));
            builder.line_to(Point::new(point.x - half, point.y - half));
            builder.close();
        }
        Shape::Cross => {
            let arm = half * 0.3;
            builder.move_to(Point::new(point.x - arm, point.y - half));
            builder.line_to(Point::new(point.x + arm, point.y - half));
            builder.line_to(Point::new(point.x + arm, point.y - arm));
            builder.line_to(Point::new(point.x + half, point.y - arm));
            builder.line_to(Point::new(point.x + half, point.y + arm));
            builder.line_to(Point::new(point.x + arm, point.y + arm));
            builder.line_to(Point::new(point.x + arm, point.y + half));
            builder.line_to(Point::new(point.x - arm, point.y + half));
            builder.line_to(Point::new(point.x - arm, point.y + arm));
            builder.line_to(Point::new(point.x - half, point.y + arm));
            builder.line_to(Point::new(point.x - half, point.y - arm));
            builder.line_to(Point::new(point.x - arm, point.y - arm));
            builder.close();
        }
        Shape::X => {
            let diag = half * 0.707;
            builder.move_to(Point::new(point.x - diag, point.y - diag));
            builder.line_to(Point::new(point.x + diag, point.y + diag));
            builder.move_to(Point::new(point.x + diag, point.y - diag));
            builder.line_to(Point::new(point.x - diag, point.y + diag));
        }
    })
}

/// Compute label rectangle for a given position (pixel coordinates)
pub(super) fn compute_label_rect(point: Point, label_width: f32, label_height: f32, position: Position) -> Rectangle {
    let padding = 6.0;

    match position {
        Position::Auto | Position::Above => Rectangle::new(
            Point::new(point.x - label_width / 2.0, point.y - label_height - padding),
            crate::core::Size::new(label_width, label_height),
        ),
        Position::Below => Rectangle::new(
            Point::new(point.x - label_width / 2.0, point.y + padding),
            crate::core::Size::new(label_width, label_height),
        ),
        Position::Left => Rectangle::new(
            Point::new(point.x - label_width - padding, point.y - label_height / 2.0),
            crate::core::Size::new(label_width, label_height),
        ),
        Position::Right => Rectangle::new(
            Point::new(point.x + padding, point.y - label_height / 2.0),
            crate::core::Size::new(label_width, label_height),
        ),
    }
}

/// Get text alignment for a position
pub(super) fn alignment_for_position(
    position: Position,
) -> (crate::core::alignment::Horizontal, crate::core::alignment::Vertical) {
    match position {
        Position::Auto | Position::Above => (
            crate::core::alignment::Horizontal::Center,
            crate::core::alignment::Vertical::Bottom,
        ),
        Position::Below => (
            crate::core::alignment::Horizontal::Center,
            crate::core::alignment::Vertical::Top,
        ),
        Position::Left => (
            crate::core::alignment::Horizontal::Right,
            crate::core::alignment::Vertical::Center,
        ),
        Position::Right => (
            crate::core::alignment::Horizontal::Left,
            crate::core::alignment::Vertical::Center,
        ),
    }
}

/// Check if a label rect is valid (within bounds, no intersections with obstacles, lines, or labels)
fn is_valid_placement(
    rect: Rectangle,
    line_segments: &[(Point, Point)],
    existing_labels: &[Rectangle],
    obstacles: &[Rectangle],
    plot_bounds: Option<Rectangle>,
) -> bool {
    // Must be within plot area bounds (if specified)
    if let Some(bounds) = plot_bounds
        && (rect.x < bounds.x
            || rect.y < bounds.y
            || rect.x + rect.width > bounds.x + bounds.width
            || rect.y + rect.height > bounds.y + bounds.height)
    {
        return false;
    }

    // Must not intersect obstacle rectangles (axes, etc.)
    let hits_obstacle = obstacles
        .iter()
        .any(|obstacle| crate::geometry::intersect::rects(rect, *obstacle));
    if hits_obstacle {
        return false;
    }

    // Must not intersect line segments
    let hits_line = line_segments
        .iter()
        .any(|(p1, p2)| crate::geometry::intersect::line_rect(*p1, *p2, rect));
    if hits_line {
        return false;
    }

    // Must not intersect existing labels
    let hits_label = existing_labels
        .iter()
        .any(|existing| crate::geometry::intersect::rects(rect, *existing));
    if hits_label {
        return false;
    }

    true
}

/// Search outward from a starting rect to find a valid placement
/// Returns the valid rect and distance traveled, or None if no valid position found
fn search_from_position(
    start_rect: Rectangle,
    line_segments: &[(Point, Point)],
    existing_labels: &[Rectangle],
    obstacles: &[Rectangle],
    plot_bounds: Option<Rectangle>,
    max_steps: i32,
) -> Option<(Rectangle, f32)> {
    let step_size = 4.0;

    // 8 directions: right, down-right, down, down-left, left, up-left, up, up-right
    let directions: [(f32, f32); 8] = [
        (1.0, 0.0),
        (1.0, 1.0),
        (0.0, 1.0),
        (-1.0, 1.0),
        (-1.0, 0.0),
        (-1.0, -1.0),
        (0.0, -1.0),
        (1.0, -1.0),
    ];

    // Check starting position first
    if is_valid_placement(start_rect, line_segments, existing_labels, obstacles, plot_bounds) {
        return Some((start_rect, 0.0));
    }

    // BFS-like spiral search outward
    for distance in 1..=max_steps {
        let offset = distance as f32 * step_size;

        for (dx, dy) in &directions {
            let rect = Rectangle::new(
                Point::new(start_rect.x + dx * offset, start_rect.y + dy * offset),
                crate::core::Size::new(start_rect.width, start_rect.height),
            );

            if is_valid_placement(rect, line_segments, existing_labels, obstacles, plot_bounds) {
                let actual_distance = (dx * offset).abs() + (dy * offset).abs();
                return Some((rect, actual_distance));
            }
        }
    }

    None
}

/// Shift `rect` so it fits inside `bounds` without resizing. If the rect is
/// larger than the bounds on an axis, that axis is left anchored at the
/// bounds origin (prefers showing the left/top portion of the text).
pub(super) fn clamp_to_bounds(rect: Rectangle, bounds: Rectangle) -> Rectangle {
    let mut x = rect.x;
    if x + rect.width > bounds.x + bounds.width {
        x = bounds.x + bounds.width - rect.width;
    }
    if x < bounds.x {
        x = bounds.x;
    }
    let mut y = rect.y;
    if y + rect.height > bounds.y + bounds.height {
        y = bounds.y + bounds.height - rect.height;
    }
    if y < bounds.y {
        y = bounds.y;
    }
    Rectangle::new(Point::new(x, y), crate::core::Size::new(rect.width, rect.height))
}

/// Find the best position for a label, returning both the position and final rect
pub(super) fn place_label(
    point: Point,
    label_width: f32,
    label_height: f32,
    line_segments: &[(Point, Point)],
    existing_labels: &[Rectangle],
    obstacles: &[Rectangle],
    plot_bounds: Option<Rectangle>,
) -> (Position, Rectangle) {
    // Start from each ideal position and search outward
    let candidates = [Position::Above, Position::Below, Position::Right, Position::Left];

    let max_search_steps = 10;
    let mut best_result: Option<(Position, Rectangle, f32)> = None;

    for candidate in candidates {
        let start_rect = compute_label_rect(point, label_width, label_height, candidate);

        if let Some((found_rect, distance)) = search_from_position(
            start_rect,
            line_segments,
            existing_labels,
            obstacles,
            plot_bounds,
            max_search_steps,
        ) {
            // Prefer results with smaller distance traveled
            let dominated = best_result
                .as_ref()
                .is_some_and(|(_, _, best_dist)| *best_dist <= distance);

            if !dominated {
                best_result = Some((candidate, found_rect, distance));
            }

            // If we found a perfect placement (distance 0), use it immediately
            if distance == 0.0 {
                break;
            }
        }
    }

    // Return best found, or fallback to Above clamped into bounds. Clamping
    // guarantees that edge-of-plot labels (first/last point) stay visible
    // even when the spiral search couldn't find a valid slot within its
    // step budget — we'd rather accept a small overlap with a line than
    // render "190" as "19".
    let (pos, rect) = best_result.map(|(pos, rect, _)| (pos, rect)).unwrap_or_else(|| {
        let rect = compute_label_rect(point, label_width, label_height, Position::Above);
        (Position::Above, rect)
    });

    let rect = match plot_bounds {
        Some(bounds) => clamp_to_bounds(rect, bounds),
        None => rect,
    };

    (pos, rect)
}

/// Returns the polyline's vertices at the current sweep progress.
///
/// With a `prev` snapshot (data change), each vertex lerps
/// component-wise from `prev[i]` to `cur[i]`. Vertices appended past
/// `prev.len()` (newly-added points) anchor at `cur[0]` so they emerge
/// from the path's leading edge as `progress` advances.
///
/// Without one (fresh mount), the polyline terminates at
/// `floor(progress * len)` full vertices plus a partial segment toward
/// the next vertex, producing a left-to-right path sweep.
///
/// At `progress == 1.0` (or `prev.len() >= cur.len()` and equal) the
/// result equals `cur` exactly, so disabling animation reproduces
/// today's geometry.
fn animate_pixel_points(cur: &[Point], prev: &[Point], progress: f32) -> Vec<Point> {
    if cur.is_empty() || progress >= 1.0 - f32::EPSILON {
        return cur.to_vec();
    }

    if !prev.is_empty() {
        let anchor = cur[0];
        return cur
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let p = prev.get(i).copied().unwrap_or(anchor);
                Point {
                    x: p.x + (c.x - p.x) * progress,
                    y: p.y + (c.y - p.y) * progress,
                }
            })
            .collect();
    }

    let len = cur.len();
    let scaled = progress * len as f32;
    let len_visible = (scaled.floor() as usize).min(len);
    if len_visible >= len {
        return cur.to_vec();
    }

    let mut out: Vec<Point> = cur[..len_visible].to_vec();
    let partial_t = scaled - len_visible as f32;
    let next = cur[len_visible];
    if let Some(last_full) = len_visible.checked_sub(1).and_then(|i| cur.get(i)).copied() {
        out.push(Point {
            x: last_full.x + (next.x - last_full.x) * partial_t,
            y: last_full.y + (next.y - last_full.y) * partial_t,
        });
    } else {
        out.push(next);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const LABEL_WIDTH: f32 = 20.0;
    const LABEL_HEIGHT: f32 = 14.0;

    fn plot_bounds() -> Rectangle {
        Rectangle::new(Point::new(0.0, 0.0), crate::core::Size::new(400.0, 300.0))
    }

    #[test]
    fn compute_label_rect_above() {
        let point = Point::new(100.0, 100.0);
        let rect = compute_label_rect(point, LABEL_WIDTH, LABEL_HEIGHT, Position::Above);

        // Above: centered horizontally, above the point
        assert_eq!(rect.x, 90.0); // 100 - 20/2
        assert_eq!(rect.y, 100.0 - LABEL_HEIGHT - 6.0); // point.y - height - padding
        assert_eq!(rect.width, LABEL_WIDTH);
        assert_eq!(rect.height, LABEL_HEIGHT);
    }

    #[test]
    fn compute_label_rect_below() {
        let point = Point::new(100.0, 100.0);
        let rect = compute_label_rect(point, LABEL_WIDTH, LABEL_HEIGHT, Position::Below);

        // Below: centered horizontally, below the point
        assert_eq!(rect.x, 90.0); // 100 - 20/2
        assert_eq!(rect.y, 106.0); // point.y + padding
    }

    #[test]
    fn compute_label_rect_right() {
        let point = Point::new(100.0, 100.0);
        let rect = compute_label_rect(point, LABEL_WIDTH, LABEL_HEIGHT, Position::Right);

        // Right: to the right of point, centered vertically
        assert_eq!(rect.x, 106.0); // point.x + padding
        assert_eq!(rect.y, 100.0 - LABEL_HEIGHT / 2.0);
    }

    #[test]
    fn compute_label_rect_left() {
        let point = Point::new(100.0, 100.0);
        let rect = compute_label_rect(point, LABEL_WIDTH, LABEL_HEIGHT, Position::Left);

        // Left: to the left of point, centered vertically
        assert_eq!(rect.x, 100.0 - LABEL_WIDTH - 6.0); // point.x - width - padding
        assert_eq!(rect.y, 100.0 - LABEL_HEIGHT / 2.0);
    }

    #[test]
    fn find_best_placement_no_obstacles() {
        // Point in middle of chart, no line segments nearby
        let point = Point::new(100.0, 100.0);
        let segments: Vec<(Point, Point)> = vec![];
        let existing: Vec<Rectangle> = vec![];
        let obstacles: Vec<Rectangle> = vec![];

        let (pos, rect) = place_label(point, LABEL_WIDTH, LABEL_HEIGHT, &segments, &existing, &obstacles, None);

        // Should pick Above (first priority)
        assert_eq!(pos, Position::Above);
        assert_eq!(rect.x, 90.0);
    }

    #[test]
    fn find_best_placement_line_above_picks_below() {
        // Point with line going up - Above would intersect
        let point = Point::new(100.0, 100.0);
        // Line going from point upward
        let segments = vec![(point, Point::new(100.0, 50.0))];
        let existing: Vec<Rectangle> = vec![];
        let obstacles: Vec<Rectangle> = vec![];

        let (pos, _rect) = place_label(point, LABEL_WIDTH, LABEL_HEIGHT, &segments, &existing, &obstacles, None);

        // Above would intersect the vertical line, should pick Below
        assert_eq!(pos, Position::Below);
    }

    #[test]
    fn find_best_placement_with_axis_obstacles() {
        // Point at bottom-left corner with axis obstacles
        let point = Point::new(10.0, 190.0); // near bottom-left
        let segments: Vec<(Point, Point)> = vec![];
        let existing: Vec<Rectangle> = vec![];
        // Simulate left axis (x < 0) and bottom axis (y > 200)
        let obstacles = vec![
            Rectangle::new(Point::new(-50.0, 0.0), crate::core::Size::new(50.0, 200.0)), // left axis
            Rectangle::new(Point::new(0.0, 200.0), crate::core::Size::new(200.0, 30.0)), // bottom axis
        ];

        let (pos, rect) = place_label(
            point,
            LABEL_WIDTH,
            LABEL_HEIGHT,
            &segments,
            &existing,
            &obstacles,
            Some(plot_bounds()),
        );

        // Should find a valid position that doesn't hit obstacles
        // Verify rect doesn't intersect obstacles
        for obs in &obstacles {
            assert!(
                !crate::geometry::intersect::rects(rect, *obs),
                "rect {:?} should not intersect obstacle {:?}",
                rect,
                obs
            );
        }
        // Position should be Above or Right (not Below which hits bottom axis)
        assert!(pos == Position::Above || pos == Position::Right);
    }

    #[test]
    fn find_best_placement_at_origin_with_line_up() {
        // Point at bottom-left with line going up-right
        // The algorithm should find a valid position via search
        let point = Point::new(0.0, 200.0);
        let segments = vec![(point, Point::new(50.0, 150.0))];
        let existing: Vec<Rectangle> = vec![];
        let obstacles: Vec<Rectangle> = vec![];

        let (_pos, rect) = place_label(
            point,
            LABEL_WIDTH,
            LABEL_HEIGHT,
            &segments,
            &existing,
            &obstacles,
            Some(plot_bounds()),
        );

        // Should find a valid position (within bounds, no line intersection)
        assert!(rect.x >= 0.0, "rect.x should be >= 0");
        assert!(rect.y >= 0.0, "rect.y should be >= 0");
        // Verify it doesn't hit the line
        assert!(
            !crate::geometry::intersect::line_rect(point, Point::new(50.0, 150.0), rect),
            "rect should not intersect line"
        );
    }

    #[test]
    fn find_best_placement_steep_line_clears_above() {
        // Point with a mostly horizontal line - Above should work directly
        let point = Point::new(50.0, 200.0);
        let segments = vec![(point, Point::new(150.0, 190.0))];
        let existing: Vec<Rectangle> = vec![];
        let obstacles: Vec<Rectangle> = vec![];

        let (pos, _rect) = place_label(
            point,
            LABEL_WIDTH,
            LABEL_HEIGHT,
            &segments,
            &existing,
            &obstacles,
            Some(plot_bounds()),
        );

        // Above should clear the nearly-horizontal line with no search needed
        assert_eq!(pos, Position::Above);
    }

    #[test]
    fn find_best_placement_corner_finds_offset() {
        // Point at (0, 0) - truly cornered, needs search to escape
        let point = Point::new(5.0, 5.0); // small offset so we have room
        // Line going diagonally, blocking Above and Left
        let segments = vec![(Point::new(0.0, 0.0), Point::new(20.0, 20.0))];
        let existing: Vec<Rectangle> = vec![];
        let obstacles: Vec<Rectangle> = vec![];

        let (_pos, rect) = place_label(
            point,
            LABEL_WIDTH,
            LABEL_HEIGHT,
            &segments,
            &existing,
            &obstacles,
            Some(plot_bounds()),
        );

        // Should find some valid position
        assert!(rect.x >= 0.0);
        assert!(rect.y >= 0.0);
        assert!(
            !crate::geometry::intersect::line_rect(Point::new(0.0, 0.0), Point::new(20.0, 20.0), rect),
            "rect should not intersect line"
        );
    }

    #[test]
    fn find_best_placement_falls_back_clamped_when_search_fails() {
        // Force the spiral to fail for every candidate by making the label
        // wider than the plot bounds. Before clamping, the fallback returned
        // an Above rect centered on the data point, bleeding off the left
        // edge. Now the returned rect is clamped into bounds so the text
        // stays visible (even though it's no longer centered on the point).
        let bounds = plot_bounds();
        let point = Point::new(bounds.x + 5.0, 100.0);
        let wide_label = bounds.width + 20.0;
        let segments: Vec<(Point, Point)> = vec![];
        let existing: Vec<Rectangle> = vec![];
        let obstacles: Vec<Rectangle> = vec![];

        let (_pos, rect) = place_label(
            point,
            wide_label,
            LABEL_HEIGHT,
            &segments,
            &existing,
            &obstacles,
            Some(bounds),
        );

        // Label wider than bounds: clamp leaves it anchored at bounds.x.
        assert_eq!(rect.x, bounds.x);
        assert!(rect.y >= bounds.y);
        assert!(rect.y + rect.height <= bounds.y + bounds.height);
    }

    #[test]
    fn find_best_placement_at_left_edge_stays_in_bounds() {
        // Repro of the area_chart example: first data point of a stacked
        // series sits at plot_bounds.x = 0, and the label "150" needs to fit
        // inside bounds without bleeding off the left edge. The returned rect
        // must satisfy `rect.x >= bounds.x` so the text isn't clipped.
        let point = Point::new(0.0, 100.0);
        let label_width = 18.0; // "150" / "190" at ~6px per char
        let label_height = 14.0;
        // Two series' worth of segments, as area.rs builds via `all_segments`
        let segments = vec![
            (Point::new(0.0, 100.0), Point::new(60.0, 90.0)),
            (Point::new(0.0, 50.0), Point::new(60.0, 45.0)),
        ];
        let existing: Vec<Rectangle> = vec![];
        let obstacles: Vec<Rectangle> = vec![];
        let bounds = plot_bounds();

        let (_pos, rect) = place_label(
            point,
            label_width,
            label_height,
            &segments,
            &existing,
            &obstacles,
            Some(bounds),
        );

        assert!(
            rect.x >= bounds.x,
            "left edge label bled outside plot bounds: rect.x={}, bounds.x={}",
            rect.x,
            bounds.x
        );
        assert!(
            rect.x + rect.width <= bounds.x + bounds.width,
            "right edge of label bled outside plot bounds"
        );
    }

    #[test]
    fn find_best_placement_at_right_edge_stays_in_bounds() {
        // Mirror of the left-edge test for the last data point.
        let bounds = plot_bounds();
        let point = Point::new(bounds.x + bounds.width, 100.0);
        let label_width = 18.0;
        let label_height = 14.0;
        let segments = vec![
            (Point::new(bounds.x + bounds.width - 60.0, 90.0), point),
            (
                Point::new(bounds.x + bounds.width - 60.0, 45.0),
                Point::new(bounds.x + bounds.width, 50.0),
            ),
        ];
        let existing: Vec<Rectangle> = vec![];
        let obstacles: Vec<Rectangle> = vec![];

        let (_pos, rect) = place_label(
            point,
            label_width,
            label_height,
            &segments,
            &existing,
            &obstacles,
            Some(bounds),
        );

        assert!(
            rect.x + rect.width <= bounds.x + bounds.width,
            "right edge label bled outside plot bounds: rect.x={}, rect.width={}, bounds right={}",
            rect.x,
            rect.width,
            bounds.x + bounds.width
        );
        assert!(rect.x >= bounds.x, "left edge of label bled outside plot bounds");
    }

    #[test]
    fn find_best_placement_avoids_existing_labels() {
        let point = Point::new(100.0, 100.0);
        let segments: Vec<(Point, Point)> = vec![];
        // Existing label right where Above would go
        let existing = vec![Rectangle::new(
            Point::new(85.0, 75.0),
            crate::core::Size::new(30.0, 20.0),
        )];
        let obstacles: Vec<Rectangle> = vec![];

        let (pos, _rect) = place_label(point, LABEL_WIDTH, LABEL_HEIGHT, &segments, &existing, &obstacles, None);

        // Above overlaps existing, should pick Below
        assert_eq!(pos, Position::Below);
    }
}

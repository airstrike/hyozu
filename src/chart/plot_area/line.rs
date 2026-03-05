use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text;
use crate::widget::renderer::geometry;

use crate::core::{Point, Rectangle};
use crate::line::label::{Position, Show};
use crate::line::marker::{self, Shape};

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
}

/// A Line series that renders connected line segments.
///
/// Like Guide and Title, this is widget-like but doesn't implement Widget.
pub struct Line<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::line::Line,
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
            _marker: std::marker::PhantomData,
        }
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
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();

        // Transform all data points to pixel coordinates
        state.pixel_points = self.data.points.iter().map(|p| plane.to_pixel(*p)).collect();

        // Build label info if configured
        state.label_texts.clear();
        state.label_positions.clear();
        state.label_rects.clear();

        if let Some(label_config) = &self.data.label {
            let label_size = label_config.size.map(|p| p.0).unwrap_or(12.0);
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
                    Position::Auto => find_best_label_placement(
                        *pixel_point,
                        label_width,
                        label_height,
                        &segments,
                        &state.label_rects,
                        &plane.obstacles,
                        Some(plane.bounds),
                    ),
                    other => {
                        // For fixed positions, use pathfinding too to avoid obstacles
                        let start_rect = compute_label_rect(*pixel_point, label_width, label_height, other);
                        // Search from the fixed position to find valid placement
                        search_from_position(
                            start_rect,
                            &segments,
                            &state.label_rects,
                            &plane.obstacles,
                            Some(plane.bounds),
                            10,
                        )
                        .map(|(rect, _)| (other, rect))
                        .unwrap_or((other, start_rect))
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

        // Get the layout bounds to offset the line to its actual screen position
        let layout_bounds = layout.bounds();

        // Determine color for this line
        let color = if let Some(data_color) = self.data.color {
            data_color.resolve(background, text_pair, None)
        } else {
            palette.get(color_offset).resolve(background, text_pair, None)
        };

        let thickness = 1.5;

        // Create a frame for drawing
        let mut frame = Frame::new(renderer, layout_bounds.size());

        // Build a path from all points
        let path = Path::new(|builder| {
            if let Some(first) = state.pixel_points.first() {
                builder.move_to(crate::core::Point::new(first.x, first.y));
                for point in state.pixel_points.iter().skip(1) {
                    builder.line_to(crate::core::Point::new(point.x, point.y));
                }
            }
        });

        // Stroke the path
        frame.stroke(&path, Stroke::default().with_width(thickness).with_color(color));

        // Draw the geometry at the layout position
        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });

        // Draw markers if configured
        if let Some(marker_config) = &self.data.marker {
            let marker_color = if let Some(marker_color_spec) = marker_config.color {
                marker_color_spec.resolve(background, text_pair, None)
            } else {
                color
            };

            let num_points = state.pixel_points.len();

            // Find min/max Y indices for MinMax modes
            let minmax_indices: Vec<usize> = match marker_config.show {
                marker::Show::MinMaxFirst | marker::Show::MinMaxAll | marker::Show::MinMaxLast => {
                    let (min_val, max_val) = self
                        .data
                        .points
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), p| {
                            (min.min(p.y), max.max(p.y))
                        });

                    match marker_config.show {
                        marker::Show::MinMaxFirst => {
                            let min_idx = self.data.points.iter().position(|p| p.y == min_val);
                            let max_idx = self.data.points.iter().position(|p| p.y == max_val);
                            [min_idx, max_idx].into_iter().flatten().collect()
                        }
                        marker::Show::MinMaxLast => {
                            let min_idx = self.data.points.iter().rposition(|p| p.y == min_val);
                            let max_idx = self.data.points.iter().rposition(|p| p.y == max_val);
                            [min_idx, max_idx].into_iter().flatten().collect()
                        }
                        marker::Show::MinMaxAll => self
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

            // Create a frame for markers
            let mut marker_frame = Frame::new(renderer, layout_bounds.size());

            for (idx, pixel_point) in state.pixel_points.iter().enumerate() {
                // Check if this point should show a marker
                let should_show = match marker_config.show {
                    marker::Show::Any => true,
                    marker::Show::FirstOnly => idx == 0,
                    marker::Show::LastOnly => idx == num_points - 1,
                    marker::Show::FirstAndLast => idx == 0 || idx == num_points - 1,
                    marker::Show::MinMaxFirst | marker::Show::MinMaxAll | marker::Show::MinMaxLast => {
                        minmax_indices.contains(&idx)
                    }
                };

                if !should_show {
                    continue;
                }

                let size = marker_config.size;
                let half = size / 2.0;

                // Build path for the marker shape
                let path = Path::new(|builder| {
                    match marker_config.shape {
                        Shape::Circle => {
                            builder.circle(*pixel_point, half);
                        }
                        Shape::Square => {
                            builder.rectangle(
                                Point::new(pixel_point.x - half, pixel_point.y - half),
                                crate::core::Size::new(size, size),
                            );
                        }
                        Shape::Diamond => {
                            builder.move_to(Point::new(pixel_point.x, pixel_point.y - half));
                            builder.line_to(Point::new(pixel_point.x + half, pixel_point.y));
                            builder.line_to(Point::new(pixel_point.x, pixel_point.y + half));
                            builder.line_to(Point::new(pixel_point.x - half, pixel_point.y));
                            builder.close();
                        }
                        Shape::Triangle => {
                            // Pointing up
                            builder.move_to(Point::new(pixel_point.x, pixel_point.y - half));
                            builder.line_to(Point::new(pixel_point.x + half, pixel_point.y + half));
                            builder.line_to(Point::new(pixel_point.x - half, pixel_point.y + half));
                            builder.close();
                        }
                        Shape::TriangleDown => {
                            // Pointing down
                            builder.move_to(Point::new(pixel_point.x, pixel_point.y + half));
                            builder.line_to(Point::new(pixel_point.x + half, pixel_point.y - half));
                            builder.line_to(Point::new(pixel_point.x - half, pixel_point.y - half));
                            builder.close();
                        }
                        Shape::Cross => {
                            // Plus sign (+)
                            let arm = half * 0.3;
                            builder.move_to(Point::new(pixel_point.x - arm, pixel_point.y - half));
                            builder.line_to(Point::new(pixel_point.x + arm, pixel_point.y - half));
                            builder.line_to(Point::new(pixel_point.x + arm, pixel_point.y - arm));
                            builder.line_to(Point::new(pixel_point.x + half, pixel_point.y - arm));
                            builder.line_to(Point::new(pixel_point.x + half, pixel_point.y + arm));
                            builder.line_to(Point::new(pixel_point.x + arm, pixel_point.y + arm));
                            builder.line_to(Point::new(pixel_point.x + arm, pixel_point.y + half));
                            builder.line_to(Point::new(pixel_point.x - arm, pixel_point.y + half));
                            builder.line_to(Point::new(pixel_point.x - arm, pixel_point.y + arm));
                            builder.line_to(Point::new(pixel_point.x - half, pixel_point.y + arm));
                            builder.line_to(Point::new(pixel_point.x - half, pixel_point.y - arm));
                            builder.line_to(Point::new(pixel_point.x - arm, pixel_point.y - arm));
                            builder.close();
                        }
                        Shape::X => {
                            let diag = half * 0.707; // cos(45°)
                            // We'll draw a simplified X using lines
                            builder.move_to(Point::new(pixel_point.x - diag, pixel_point.y - diag));
                            builder.line_to(Point::new(pixel_point.x + diag, pixel_point.y + diag));
                            builder.move_to(Point::new(pixel_point.x + diag, pixel_point.y - diag));
                            builder.line_to(Point::new(pixel_point.x - diag, pixel_point.y + diag));
                        }
                    }
                });

                // Fill the marker (except for X which is stroke-only)
                if marker_config.shape != Shape::X {
                    marker_frame.fill(&path, marker_color);
                }

                // Stroke the marker if configured
                if let Some(stroke_color_spec) = marker_config.stroke {
                    let stroke_color = stroke_color_spec.resolve(background, text_pair, None);
                    marker_frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(marker_config.stroke_width)
                            .with_color(stroke_color),
                    );
                } else if marker_config.shape == Shape::X {
                    // X shape needs stroke to be visible
                    marker_frame.stroke(
                        &path,
                        Stroke::default()
                            .with_width(marker_config.stroke_width.max(2.0))
                            .with_color(marker_color),
                    );
                }
            }

            let marker_geometry = marker_frame.into_geometry();
            renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
                renderer.draw_geometry(marker_geometry);
            });
        }

        // Draw data labels from state
        if let Some(label_config) = &self.data.label {
            let label_size = label_config.size.map(|p| p.0).unwrap_or(12.0);

            // Resolve label color (defaults to line color)
            let label_color = if let Some(label_color_spec) = label_config.color {
                label_color_spec.resolve(background, text_pair, None)
            } else {
                color
            };

            for ((label_rect, label_text), resolved_pos) in state
                .label_rects
                .iter()
                .zip(state.label_texts.iter())
                .zip(state.label_positions.iter())
            {
                // Calculate alignment based on resolved position
                let (align_x, align_y) = alignment_for_position(*resolved_pos);

                // Calculate anchor point based on position
                let (anchor_x, anchor_y) = match resolved_pos {
                    Position::Auto | Position::Above => (
                        layout_bounds.x + label_rect.x + label_rect.width / 2.0,
                        layout_bounds.y + label_rect.y + label_rect.height,
                    ),
                    Position::Below => (
                        layout_bounds.x + label_rect.x + label_rect.width / 2.0,
                        layout_bounds.y + label_rect.y,
                    ),
                    Position::Left => (
                        layout_bounds.x + label_rect.x + label_rect.width,
                        layout_bounds.y + label_rect.y + label_rect.height / 2.0,
                    ),
                    Position::Right => (
                        layout_bounds.x + label_rect.x,
                        layout_bounds.y + label_rect.y + label_rect.height / 2.0,
                    ),
                };

                renderer.fill_text(
                    crate::core::text::Text {
                        content: label_text.clone(),
                        bounds: crate::core::Size::new(1000.0, 1000.0),
                        size: label_size.into(),
                        font: renderer.default_font(),
                        align_x: align_x.into(),
                        align_y,
                        line_height: crate::core::text::LineHeight::default(),
                        shaping: crate::core::text::Shaping::Basic,
                        wrapping: crate::core::text::Wrapping::None,
                        ellipsis: crate::core::text::Ellipsis::default(),
                        hint_factor: renderer.scale_factor(),
                    },
                    crate::core::Point::new(anchor_x, anchor_y),
                    label_color,
                    *_viewport,
                );
            }
        }
    }
}

/// Compute label rectangle for a given position (pixel coordinates)
fn compute_label_rect(point: Point, label_width: f32, label_height: f32, position: Position) -> Rectangle {
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
fn alignment_for_position(
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

/// Find the best position for a label, returning both the position and final rect
fn find_best_label_placement(
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

    // Return best found, or fallback to Above at origin
    best_result.map(|(pos, rect, _)| (pos, rect)).unwrap_or_else(|| {
        let rect = compute_label_rect(point, label_width, label_height, Position::Above);
        (Position::Above, rect)
    })
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

        let (pos, rect) =
            find_best_label_placement(point, LABEL_WIDTH, LABEL_HEIGHT, &segments, &existing, &obstacles, None);

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

        let (pos, _rect) =
            find_best_label_placement(point, LABEL_WIDTH, LABEL_HEIGHT, &segments, &existing, &obstacles, None);

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

        let (pos, rect) = find_best_label_placement(
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

        let (_pos, rect) = find_best_label_placement(
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

        let (pos, _rect) = find_best_label_placement(
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

        let (_pos, rect) = find_best_label_placement(
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
    fn find_best_placement_avoids_existing_labels() {
        let point = Point::new(100.0, 100.0);
        let segments: Vec<(Point, Point)> = vec![];
        // Existing label right where Above would go
        let existing = vec![Rectangle::new(
            Point::new(85.0, 75.0),
            crate::core::Size::new(30.0, 20.0),
        )];
        let obstacles: Vec<Rectangle> = vec![];

        let (pos, _rect) =
            find_best_label_placement(point, LABEL_WIDTH, LABEL_HEIGHT, &segments, &existing, &obstacles, None);

        // Above overlaps existing, should pick Below
        assert_eq!(pos, Position::Below);
    }
}

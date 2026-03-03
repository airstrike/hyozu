use super::Plane;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size};
use crate::line::marker::Shape;
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text;
use crate::widget::renderer::geometry;

/// State for Xy — stores pixel positions of scatter points
pub struct State {
    /// Pixel coordinates for each point
    pub pixel_points: Vec<Point>,
}

/// An Xy series that renders scatter charts.
pub struct Xy<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::xy::Xy,
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
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Xy
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                pixel_points: Vec::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Xy state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the scatter — transform data to pixel coordinates
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        plane: &Plane,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();

        state.pixel_points = self
            .data
            .points
            .iter()
            .map(|p| plane.to_pixel(*p))
            .collect();

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

        let layout_bounds = layout.bounds();

        let color = if let Some(data_color) = self.data.color {
            data_color.resolve(background, text_pair, None)
        } else {
            palette
                .get(color_offset)
                .resolve(background, text_pair, None)
        };

        let marker_config = &self.data.marker;
        let mut frame = Frame::new(renderer, layout_bounds.size());

        for pixel_point in &state.pixel_points {
            let size = marker_config.size;
            let half = size / 2.0;

            let path = Path::new(|builder| match marker_config.shape {
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
                    builder.move_to(Point::new(
                        pixel_point.x,
                        pixel_point.y - half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + half,
                        pixel_point.y,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x,
                        pixel_point.y + half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x - half,
                        pixel_point.y,
                    ));
                    builder.close();
                }
                Shape::Triangle => {
                    builder.move_to(Point::new(
                        pixel_point.x,
                        pixel_point.y - half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + half,
                        pixel_point.y + half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x - half,
                        pixel_point.y + half,
                    ));
                    builder.close();
                }
                Shape::TriangleDown => {
                    builder.move_to(Point::new(
                        pixel_point.x,
                        pixel_point.y + half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + half,
                        pixel_point.y - half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x - half,
                        pixel_point.y - half,
                    ));
                    builder.close();
                }
                Shape::Cross => {
                    let arm = half * 0.3;
                    builder.move_to(Point::new(
                        pixel_point.x - arm,
                        pixel_point.y - half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + arm,
                        pixel_point.y - half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + arm,
                        pixel_point.y - arm,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + half,
                        pixel_point.y - arm,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + half,
                        pixel_point.y + arm,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + arm,
                        pixel_point.y + arm,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + arm,
                        pixel_point.y + half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x - arm,
                        pixel_point.y + half,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x - arm,
                        pixel_point.y + arm,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x - half,
                        pixel_point.y + arm,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x - half,
                        pixel_point.y - arm,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x - arm,
                        pixel_point.y - arm,
                    ));
                    builder.close();
                }
                Shape::X => {
                    let diag = half * 0.707;
                    builder.move_to(Point::new(
                        pixel_point.x - diag,
                        pixel_point.y - diag,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x + diag,
                        pixel_point.y + diag,
                    ));
                    builder.move_to(Point::new(
                        pixel_point.x + diag,
                        pixel_point.y - diag,
                    ));
                    builder.line_to(Point::new(
                        pixel_point.x - diag,
                        pixel_point.y + diag,
                    ));
                }
            });

            // Fill (except X shape)
            let marker_color = if let Some(color_spec) = marker_config.color {
                color_spec.resolve(background, text_pair, None)
            } else {
                color
            };

            if marker_config.shape != Shape::X {
                frame.fill(&path, marker_color);
            }

            // Stroke
            if let Some(stroke_color_spec) = marker_config.stroke {
                let stroke_color =
                    stroke_color_spec.resolve(background, text_pair, None);
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
        renderer.with_translation(
            crate::core::Vector::new(layout_bounds.x, layout_bounds.y),
            |renderer| {
                renderer.draw_geometry(geometry);
            },
        );
    }
}

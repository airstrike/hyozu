use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::data::Datum;
use crate::mark::tick::Orientation;
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Computed pixel position for a single tick mark.
#[derive(Debug, Clone, Copy)]
struct TickPosition {
    /// Position along the value axis (pixels).
    value_px: f32,
    /// Center of the category band (pixels).
    category_px: f32,
    /// Half-length of the tick line (pixels).
    half_length: f32,
}

/// State for Tick — stores computed pixel positions.
pub struct State {
    positions: Vec<TickPosition>,
}

/// A Tick series that renders per-datum mark lines.
pub struct Tick<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::tick::Tick,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Tick<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub fn new(data: &'a crate::mark::tick::Tick) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State { positions: Vec::new() }),
            children: Vec::new(),
        }
    }

    pub(super) fn diff(&self, _tree: &mut Tree) {}

    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();
        state.positions.clear();

        let (num_categories, band_px) = match self.data.orientation {
            Orientation::Vertical => {
                // Horizontal bars: categories on y-axis, values on x-axis
                let n = (plane.domain.y.max - plane.domain.y.min + 1.0).max(1.0);
                (n, plane.bounds.height / n as f32)
            }
            Orientation::Horizontal => {
                // Vertical bars: categories on x-axis, values on y-axis
                let n = (plane.domain.x.max - plane.domain.x.min + 1.0).max(1.0);
                (n, plane.bounds.width / n as f32)
            }
        };

        let half_length = band_px * self.data.length / 2.0;
        let _ = num_categories;

        for point in &self.data.points {
            // point.x = category index, point.y = value
            let pixel = match self.data.orientation {
                Orientation::Vertical => {
                    // For horizontal bars: x-axis = value, y-axis = category
                    plane.to_pixel(Datum::new(point.y, point.x))
                }
                Orientation::Horizontal => {
                    // For vertical bars: x-axis = category, y-axis = value
                    plane.to_pixel(Datum::new(point.x, point.y))
                }
            };

            let (value_px, category_px) = match self.data.orientation {
                Orientation::Vertical => (pixel.x, pixel.y),
                Orientation::Horizontal => (pixel.y, pixel.x),
            };

            state.positions.push(TickPosition {
                value_px,
                category_px,
                half_length,
            });
        }

        Node::new(Size::ZERO)
    }

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

        if state.positions.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        let line_color = if let Some(color) = self.data.color {
            color.resolve(background, text_pair, &seed, None)
        } else {
            text_pair.on_light
        };

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let path = Path::new(|builder| {
            for pos in &state.positions {
                match self.data.orientation {
                    Orientation::Vertical => {
                        // Vertical line at value_px, spanning category band
                        builder.move_to(crate::core::Point::new(pos.value_px, pos.category_px - pos.half_length));
                        builder.line_to(crate::core::Point::new(pos.value_px, pos.category_px + pos.half_length));
                    }
                    Orientation::Horizontal => {
                        // Horizontal line at value_px, spanning category band
                        builder.move_to(crate::core::Point::new(pos.category_px - pos.half_length, pos.value_px));
                        builder.line_to(crate::core::Point::new(pos.category_px + pos.half_length, pos.value_px));
                    }
                }
            }
        });

        frame.stroke(
            &path,
            Stroke::default().with_width(self.data.width).with_color(line_color),
        );

        // FIXME: Add optional per-tick labels (value text near each tick mark)

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }
}

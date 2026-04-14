use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::data::Datum;
use crate::mark::band::BandOrientation;
use crate::widget::canvas::{Frame, Path, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// State for Band — stores pixel-space rectangle spanning the plot area.
pub struct State {
    /// Lower pixel coordinate along the band's orientation axis.
    pub lower_pixel: f32,
    /// Upper pixel coordinate along the band's orientation axis.
    pub upper_pixel: f32,
}

/// A Band series that renders a translucent shaded value range.
pub struct Band<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::band::Band,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Band<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Band borrowing data
    pub fn new(data: &'a crate::mark::band::Band) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Band
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                lower_pixel: 0.0,
                upper_pixel: 0.0,
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Band state
    pub(super) fn diff(&self, _tree: &mut Tree) {}

    /// Layout the band — convert data values to pixel coordinates.
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();

        match self.data.orientation {
            BandOrientation::Horizontal => {
                state.lower_pixel = plane.to_pixel(Datum::new(0.0, self.data.lower)).y;
                state.upper_pixel = plane.to_pixel(Datum::new(0.0, self.data.upper)).y;
            }
            BandOrientation::Vertical => {
                state.lower_pixel = plane.to_pixel(Datum::new(self.data.lower, 0.0)).x;
                state.upper_pixel = plane.to_pixel(Datum::new(self.data.upper, 0.0)).x;
            }
        }

        Node::new(Size::ZERO)
    }

    /// Draws the band rectangle.
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

        let background = theme.background_color();
        let text_pair = theme.text_pair();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let base_color = if let Some(color) = self.data.color {
            color.resolve(background, text_pair, None)
        } else {
            text_pair.on_light
        };

        let fill_color = crate::core::Color {
            a: base_color.a * self.data.opacity,
            ..base_color
        };

        let rect = match self.data.orientation {
            BandOrientation::Horizontal => {
                // Y-range band spans full plot width. Plane pixel values grow
                // downward, so `upper` (data) maps to a smaller pixel y.
                let (y0, y1) = if state.upper_pixel <= state.lower_pixel {
                    (state.upper_pixel, state.lower_pixel)
                } else {
                    (state.lower_pixel, state.upper_pixel)
                };
                (0.0_f32, y0, layout_bounds.width, (y1 - y0).max(0.0))
            }
            BandOrientation::Vertical => {
                let (x0, x1) = if state.lower_pixel <= state.upper_pixel {
                    (state.lower_pixel, state.upper_pixel)
                } else {
                    (state.upper_pixel, state.lower_pixel)
                };
                (x0, 0.0_f32, (x1 - x0).max(0.0), layout_bounds.height)
            }
        };

        let (rx, ry, rw, rh) = rect;
        if rw > 0.0 && rh > 0.0 {
            let path = Path::new(|builder| {
                builder.rectangle(crate::core::Point::new(rx, ry), crate::core::Size::new(rw, rh));
            });
            frame.fill(&path, fill_color);
        }

        // Optional label at the near edge (top for horizontal, left for vertical).
        if let Some(label_text) = &self.data.label {
            let label_size = 11.0_f32;
            let (position, align_x, align_y) = match self.data.orientation {
                BandOrientation::Horizontal => (
                    crate::core::Point::new(layout_bounds.width - 4.0, ry + 2.0),
                    crate::core::alignment::Horizontal::Right.into(),
                    crate::core::alignment::Vertical::Top,
                ),
                BandOrientation::Vertical => (
                    crate::core::Point::new(rx + 4.0, 4.0),
                    crate::core::alignment::Horizontal::Left.into(),
                    crate::core::alignment::Vertical::Top,
                ),
            };

            frame.fill_text(CanvasText {
                content: label_text.clone(),
                position,
                color: base_color,
                size: label_size.into(),
                font: theme.font(),
                align_x,
                align_y,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }
}

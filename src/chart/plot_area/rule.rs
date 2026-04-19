use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::data::Datum;
use crate::mark::rule::RuleOrientation;
use crate::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// State for Rule — stores pixel position
pub struct State {
    /// Pixel position of the rule line
    pub position: f32,
}

/// A Rule series that renders reference lines.
pub struct Rule<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::rule::Rule,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Rule<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Rule borrowing data
    pub fn new(data: &'a crate::mark::rule::Rule) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Rule
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State { position: 0.0 }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Rule state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the rule — convert value to pixel position
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();

        match self.data.orientation {
            RuleOrientation::Horizontal => {
                let pixel = plane.to_pixel(Datum::new(0.0, self.data.value));
                state.position = pixel.y;
            }
            RuleOrientation::Vertical => {
                let pixel = plane.to_pixel(Datum::new(self.data.value, 0.0));
                state.position = pixel.x;
            }
        }

        Node::new(Size::ZERO)
    }

    /// Draws the rule line
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
        let seed = theme.seed();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let line_color = if let Some(color) = self.data.color {
            color.resolve(background, text_pair, &seed, None)
        } else {
            text_pair.on_light
        };

        let path = Path::new(|builder| match self.data.orientation {
            RuleOrientation::Horizontal => {
                builder.move_to(crate::core::Point::new(0.0, state.position));
                builder.line_to(crate::core::Point::new(layout_bounds.width, state.position));
            }
            RuleOrientation::Vertical => {
                builder.move_to(crate::core::Point::new(state.position, 0.0));
                builder.line_to(crate::core::Point::new(state.position, layout_bounds.height));
            }
        });

        frame.stroke(
            &path,
            Stroke::default().with_width(self.data.width).with_color(line_color),
        );

        // Draw label if configured
        if let Some(label_text) = &self.data.label {
            let label_color = line_color;
            let label_size = 11.0_f32;

            let (position, align_x, align_y) = match self.data.orientation {
                RuleOrientation::Horizontal => (
                    crate::core::Point::new(layout_bounds.width - 4.0, state.position - 4.0),
                    crate::core::alignment::Horizontal::Right.into(),
                    crate::core::alignment::Vertical::Bottom,
                ),
                RuleOrientation::Vertical => (
                    crate::core::Point::new(state.position + 4.0, 4.0),
                    crate::core::alignment::Horizontal::Left.into(),
                    crate::core::alignment::Vertical::Top,
                ),
            };

            frame.fill_text(CanvasText {
                content: label_text.clone(),
                position,
                color: label_color,
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

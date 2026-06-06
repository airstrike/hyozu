use super::{Domain, to_pixel};
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::data::Datum;
use crate::mark::rule::RuleOrientation;
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text::{self, paragraph};
use crate::widget::renderer::geometry;

/// State for Rule — stores pixel position and the shaped label paragraph.
pub struct State<P>
where
    P: text::Paragraph,
{
    /// Pixel position of the rule line
    pub position: f32,
    /// The reference-line label, shaped once in `layout` and rendered in
    /// `draw` via [`crate::core::text::Renderer::fill_paragraph`]. Empty
    /// (default) when the rule has no label configured.
    pub label: paragraph::Plain<P>,
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
    Renderer: text::Renderer<Font = crate::core::Font> + geometry::Renderer,
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
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                position: 0.0,
                label: paragraph::Plain::default(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Rule state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the rule — convert value to pixel position and shape the label.
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        _limits: &Limits,
        domain: &Domain,
        rect: crate::core::Rectangle,
        design: Option<&dyn crate::design::Design>,
    ) -> Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        match self.data.orientation {
            RuleOrientation::Horizontal => {
                let pixel = to_pixel(domain, rect, Datum::new(0.0, self.data.value));
                state.position = pixel.y;
            }
            RuleOrientation::Vertical => {
                let pixel = to_pixel(domain, rect, Datum::new(self.data.value, 0.0));
                state.position = pixel.x;
            }
        }

        // Shape the rule's label. The 11 px size is intrinsic to the mark;
        // the design supplies the font base (default font when the chart has
        // no design), so it is laid out here and drawn in `draw` via
        // `fill_paragraph` rather than reshaped every frame.
        if let Some(label_text) = &self.data.label {
            let default_font = design
                .map(|d| d.data_label_text().resolved_font(renderer.default_font()))
                .unwrap_or_else(|| renderer.default_font());
            let _ = state.label.update(text::Text {
                content: label_text,
                bounds: Size::INFINITE,
                size: 11.0.into(),
                line_height: text::LineHeight::default(),
                font: default_font,
                align_x: text::Alignment::Left,
                align_y: crate::core::alignment::Vertical::Top,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
                ellipsis: text::Ellipsis::default(),
                hint_factor: renderer.scale_factor(),
                font_features: Vec::new(),
                font_variations: Vec::new(),
                letter_spacing: Default::default(),
                weight: None,
            });
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
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        // Non-finite `data.value` propagates through `to_pixel` to a
        // NaN position; the line path and label both crash the tessellator.
        // Treat as a gap.
        if !state.position.is_finite() {
            return;
        }

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

        // Anchor for the cached label paragraph, computed only when a label
        // is configured. `fill_text` shifted the glyph box from `position` by
        // `min_bounds` per alignment (Right: −width, Bottom: −height); we apply
        // the identical shift here so `fill_paragraph` (top-left origin) lands
        // pixel-identically.
        let label_anchor = self.data.label.as_ref().map(|_| {
            let (position, align_x, align_y): (
                crate::core::Point,
                crate::core::alignment::Horizontal,
                crate::core::alignment::Vertical,
            ) = match self.data.orientation {
                RuleOrientation::Horizontal => (
                    crate::core::Point::new(layout_bounds.width - 4.0, state.position - 4.0),
                    crate::core::alignment::Horizontal::Right,
                    crate::core::alignment::Vertical::Bottom,
                ),
                RuleOrientation::Vertical => (
                    crate::core::Point::new(state.position + 4.0, 4.0),
                    crate::core::alignment::Horizontal::Left,
                    crate::core::alignment::Vertical::Top,
                ),
            };

            let bounds = state.label.min_bounds();
            let anchor_x = match align_x {
                crate::core::alignment::Horizontal::Left => position.x,
                crate::core::alignment::Horizontal::Center => position.x - bounds.width / 2.0,
                crate::core::alignment::Horizontal::Right => position.x - bounds.width,
            };
            let anchor_y = match align_y {
                crate::core::alignment::Vertical::Top => position.y,
                crate::core::alignment::Vertical::Center => position.y - bounds.height / 2.0,
                crate::core::alignment::Vertical::Bottom => position.y - bounds.height,
            };
            crate::core::Point::new(anchor_x, anchor_y)
        });

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);
        let geometry = frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(geometry);
        });

        // Draw the label text on top of the line via the cached paragraph.
        if let Some(anchor) = label_anchor {
            renderer.fill_paragraph(state.label.raw(), anchor + translation, line_color, layout_bounds);
        }
    }
}

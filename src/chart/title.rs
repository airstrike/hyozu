use crate::core::layout::{Limits, Node};
use crate::core::text::paragraph;
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size, text};

/// State for a Title - stores paragraph for text measurement
pub struct State<P>
where
    P: text::Paragraph,
{
    /// Paragraph for the title text
    pub paragraph: paragraph::Plain<P>,
}

/// A Title wraps title text and handles UI layout with proper text measurement.
///
/// Similar to Guide, this borrows the data and is widget-like but doesn't
/// implement Widget. Keeps UI code separate from data viz.
pub struct Title<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer,
{
    text: &'a str,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Title<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer,
{
    /// Create a new Title borrowing text
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Title
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                paragraph: paragraph::Plain::default(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Title state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the title, measuring text and calculating size
    pub fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        // Update paragraph with title text
        let _ = state.paragraph.update(text::Text {
            content: self.text,
            bounds: Size::INFINITE,
            size: 16.0.into(), // TODO: Make configurable
            line_height: text::LineHeight::default(),
            font: renderer.default_font(),
            align_x: text::Alignment::Center,
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

        let text_size = state.paragraph.min_bounds();
        let padding = 10.0; // Padding above and below title

        // Title takes full width, height is text height plus padding
        let size = Size::new(limits.max().width, text_size.height + padding * 2.0);

        Node::new(size)
    }

    /// Draws the title
    #[allow(clippy::too_many_arguments)]
    pub fn draw<D>(
        &self,
        _tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        _style: &crate::core::renderer::Style,
        layout: crate::core::Layout<'_>,
        _cursor: crate::core::mouse::Cursor,
        viewport: &crate::core::Rectangle,
    ) where
        D: crate::design::Design + ?Sized,
    {
        let bounds = layout.bounds();
        let padding = 10.0;

        // Get text color from theme, resolved relative to background
        let background = design.background_color();
        let text_pair = design.text_pair();
        let text_color = design.text_color().resolve(background, text_pair, None);

        // Draw title text centered at top with padding
        let position = Point::new(bounds.x + bounds.width / 2.0, bounds.y + padding);

        renderer.fill_text(
            crate::core::text::Text {
                content: self.text.to_string(),
                bounds: Size::new(bounds.width, bounds.height),
                size: 16.0.into(),
                font: renderer.default_font(),
                align_x: crate::core::alignment::Horizontal::Center.into(),
                align_y: crate::core::alignment::Vertical::Top,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                wrapping: crate::core::text::Wrapping::None,
                ellipsis: crate::core::text::Ellipsis::default(),
                hint_factor: renderer.scale_factor(),
                font_features: Vec::new(),
                font_variations: Vec::new(),
                letter_spacing: Default::default(),
                weight: None,
            },
            position,
            text_color,
            *viewport,
        );
    }
}

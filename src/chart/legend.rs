use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Size, text};

/// State for a Legend - stores any necessary state for rendering.
pub struct State<P>
where
    P: text::Paragraph,
{
    /// Placeholder for future legend state (e.g., cached text measurements)
    _phantom: std::marker::PhantomData<P>,
}

/// A Legend displays a key for the chart's data series.
///
/// Like Guide, this borrows data and is widget-like but doesn't implement Widget.
/// Keeps UI code separate from data viz.
pub struct Legend<'a, Message, Renderer>
where
    Renderer: text::Renderer,
{
    _message: std::marker::PhantomData<Message>,
    _lifetime: std::marker::PhantomData<&'a ()>,
    _renderer: std::marker::PhantomData<Renderer>,
}

impl<'a, Message, Renderer> Default for Legend<'a, Message, Renderer>
where
    Renderer: text::Renderer,
 {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, Message, Renderer> Legend<'a, Message, Renderer>
where
    Renderer: text::Renderer,
{
    /// Create a new Legend.
    pub fn new() -> Self {
        Self {
            _message: std::marker::PhantomData,
            _lifetime: std::marker::PhantomData,
            _renderer: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Legend.
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                _phantom: std::marker::PhantomData,
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Legend state.
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff yet
    }

    /// Layout the legend.
    pub fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &Limits,
    ) -> Node {
        // For now, legend takes no space
        Node::new(Size::ZERO.min(limits.max()))
    }

    /// Draws the legend.
    #[allow(clippy::too_many_arguments)]
    pub fn draw<Theme>(
        &self,
        _tree: &crate::core::widget::Tree,
        _renderer: &mut Renderer,
        _theme: &Theme,
        _style: &crate::core::renderer::Style,
        _layout: crate::core::Layout<'_>,
        _cursor: crate::core::mouse::Cursor,
        _viewport: &crate::core::Rectangle,
    ) where
        Theme: crate::design::Design,
    {
        // TODO: Implement legend rendering
        // Will render series labels with color swatches
    }
}

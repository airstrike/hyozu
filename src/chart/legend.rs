use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Size, text};
use crate::data::mark::LegendEntry;

/// Gap between swatch and text label.
const SWATCH_TEXT_GAP: f32 = 4.0;
/// Gap between entries.
const ENTRY_GAP: f32 = 16.0;
/// Vertical padding above the legend row.
const PADDING_TOP: f32 = 4.0;
/// Vertical padding below the legend row.
const PADDING_BOTTOM: f32 = 4.0;
/// Average character width as a proportion of font size (rough estimate).
const CHAR_WIDTH_RATIO: f32 = 0.55;
/// Swatch size as a proportion of font size.
const SWATCH_SCALE: f32 = 0.85;

/// State for a Legend.
pub struct State<P>
where
    P: text::Paragraph,
{
    _phantom: std::marker::PhantomData<P>,
}

/// A Legend displays a key for the chart's data series.
///
/// Owns its entries so it can be stored alongside other borrowed components
/// in Scene without lifetime issues.
pub struct Legend<'a, Message, Renderer>
where
    Renderer: text::Renderer<Font = crate::core::Font>,
{
    entries: Vec<LegendEntry>,
    _marker: std::marker::PhantomData<(Message, &'a Renderer)>,
}

impl<'a, Message, Renderer> Legend<'a, Message, Renderer>
where
    Renderer: text::Renderer<Font = crate::core::Font>,
{
    /// Create a new Legend from entries.
    pub fn new(entries: Vec<LegendEntry>) -> Self {
        Self {
            entries,
            _marker: std::marker::PhantomData,
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
        // No children to diff
    }

    /// Layout the legend.
    ///
    /// Computes the height needed for a horizontal row of entries.
    pub fn layout(
        &self,
        _tree: &mut Tree,
        _renderer: &Renderer,
        limits: &Limits,
    ) -> Node {
        if self.entries.is_empty() {
            return Node::new(Size::ZERO);
        }

        let font_size = 12.0_f32;
        let row_height = font_size + PADDING_TOP + PADDING_BOTTOM;

        Node::new(Size::new(limits.max().width, row_height))
    }

    /// Draws the legend.
    #[allow(clippy::too_many_arguments)]
    pub fn draw<Theme>(
        &self,
        _tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &Theme,
        _style: &crate::core::renderer::Style,
        layout: crate::core::Layout<'_>,
        _cursor: crate::core::mouse::Cursor,
        viewport: &crate::core::Rectangle,
    ) where
        Theme: crate::design::Design + ?Sized,
        Renderer: crate::core::Renderer,
    {
        if self.entries.is_empty() {
            return;
        }

        let bounds = layout.bounds();
        let background = design.background_color();
        let text_pair = design.text_pair();
        let palette = design.data_colors();
        let font = design.font();
        let font_size = design.font_size();
        let swatch_size = font_size * SWATCH_SCALE;

        // Compute total width of all entries to center them
        let total_width: f32 = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let text_width =
                    entry.name.len() as f32 * font_size * CHAR_WIDTH_RATIO;
                let entry_width = swatch_size + SWATCH_TEXT_GAP + text_width;
                if i < self.entries.len() - 1 {
                    entry_width + ENTRY_GAP
                } else {
                    entry_width
                }
            })
            .sum();

        // Center the legend row horizontally
        let start_x = bounds.x + (bounds.width - total_width).max(0.0) / 2.0;
        let mut x = start_x;

        // Vertically center within the row
        let y_top = bounds.y + PADDING_TOP;

        let text_color =
            design.text_color().resolve(background, text_pair, None);

        for (i, entry) in self.entries.iter().enumerate() {
            // Resolve swatch color
            let swatch_color = if let Some(entry_color) = entry.color {
                entry_color.resolve(background, text_pair, None)
            } else {
                palette
                    .get(i % palette.len())
                    .map(|c| c.resolve(background, text_pair, None))
                    .unwrap_or(background)
            };

            // Draw swatch rectangle
            renderer.fill_quad(
                crate::core::renderer::Quad {
                    bounds: crate::core::Rectangle {
                        x,
                        y: y_top + (font_size - swatch_size) / 2.0,
                        width: swatch_size,
                        height: swatch_size,
                    },
                    border: crate::core::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                swatch_color,
            );

            // Draw text label
            renderer.fill_text(
                crate::core::text::Text {
                    content: entry.name.clone(),
                    bounds: Size::new(bounds.width, font_size * 2.0),
                    size: font_size.into(),
                    font,
                    align_x: crate::core::alignment::Horizontal::Left.into(),
                    align_y: crate::core::alignment::Vertical::Top,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    wrapping: crate::core::text::Wrapping::None,
                    ellipsis: crate::core::text::Ellipsis::default(),
                    hint_factor: renderer.scale_factor(),
                },
                crate::core::Point::new(
                    x + swatch_size + SWATCH_TEXT_GAP,
                    y_top,
                ),
                text_color,
                *viewport,
            );

            // Advance x position
            let text_width =
                entry.name.len() as f32 * font_size * CHAR_WIDTH_RATIO;
            x += swatch_size + SWATCH_TEXT_GAP + text_width + ENTRY_GAP;
        }
    }
}

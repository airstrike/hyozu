use std::collections::HashSet;

use crate::core::layout::{Limits, Node};
use crate::core::text::paragraph;
use crate::core::widget::{Tree, tree};
use crate::core::{Rectangle, Size, text};
use crate::data::legend::Position;
use crate::data::mark::LegendEntry;

/// Gap between swatch and text label.
const SWATCH_TEXT_GAP: f32 = 4.0;
/// Gap between entries.
const ENTRY_GAP: f32 = 16.0;
/// Vertical padding above/below the legend.
const PADDING_V: f32 = 4.0;
/// Swatch size as a proportion of font size.
const SWATCH_SCALE: f32 = 0.85;
/// Default font size for legend text (2px smaller than default 12.0 label size).
const DEFAULT_FONT_SIZE: f32 = 10.0;

/// State for a Legend — stores measured text widths and row assignments.
pub struct State<P>
where
    P: text::Paragraph,
{
    /// Measured text widths for each entry.
    pub widths: Vec<f32>,
    /// Scratch paragraph for measurement.
    paragraph: paragraph::Plain<P>,
    /// Row assignments: each row contains (entry_index, x_offset_within_row) pairs.
    pub rows: Vec<Vec<(usize, f32)>>,
    /// Row widths (total content width per row, for centering).
    pub row_widths: Vec<f32>,
    /// Absolute-pixel bounding rect for each entry (indexed by entry index),
    /// cached on the last `draw()` call. Used for legend click hit-testing.
    pub entry_rects: Vec<Option<Rectangle>>,
}

/// A Legend displays a key for the chart's data series.
pub struct Legend<'a, Message, Renderer>
where
    Renderer: text::Renderer<Font = crate::core::Font>,
{
    entries: Vec<LegendEntry>,
    position: Position,
    font_size: f32,
    wrap: bool,
    interactive: bool,
    _marker: std::marker::PhantomData<(Message, &'a Renderer)>,
}

impl<'a, Message, Renderer> Legend<'a, Message, Renderer>
where
    Renderer: text::Renderer<Font = crate::core::Font>,
{
    /// Create a new Legend from entries and configuration.
    pub fn new(
        entries: Vec<LegendEntry>,
        position: Position,
        font_size: Option<f32>,
        wrap: bool,
        interactive: bool,
    ) -> Self {
        Self {
            entries,
            position,
            font_size: font_size.unwrap_or(DEFAULT_FONT_SIZE),
            wrap,
            interactive,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the position of this legend.
    pub fn position(&self) -> Position {
        self.position
    }

    /// Returns whether this legend is click-interactive.
    pub fn interactive(&self) -> bool {
        self.interactive
    }

    /// Returns the entries on this legend.
    pub fn entries(&self) -> &[LegendEntry] {
        &self.entries
    }

    /// Returns true if this legend is horizontal (Above/Below).
    fn is_horizontal(&self) -> bool {
        matches!(self.position, Position::Above | Position::Below)
    }

    /// Returns the initial tree state for this Legend.
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                widths: Vec::new(),
                paragraph: paragraph::Plain::default(),
                rows: Vec::new(),
                row_widths: Vec::new(),
                entry_rects: Vec::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Legend state.
    pub(super) fn diff(&self, _tree: &mut Tree) {}

    /// Layout the legend.
    pub fn layout(&self, tree: &mut Tree, renderer: &Renderer, limits: &Limits) -> Node {
        if self.entries.is_empty() {
            {
                let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
                state.entry_rects.clear();
            }
            return Node::new(Size::ZERO);
        }

        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let font_size = self.font_size;
        let swatch_size = font_size * SWATCH_SCALE;

        // Measure each entry's text width
        state.widths.clear();
        for entry in &self.entries {
            let _ = state.paragraph.update(text::Text {
                content: entry.name.as_str(),
                bounds: Size::INFINITE,
                size: font_size.into(),
                line_height: text::LineHeight::default(),
                font: renderer.default_font(),
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
            state.widths.push(state.paragraph.min_width());
        }

        // Compute entry widths (swatch + gap + text)
        let entry_widths: Vec<f32> = state
            .widths
            .iter()
            .map(|tw| swatch_size + SWATCH_TEXT_GAP + tw)
            .collect();

        let row_height = font_size + PADDING_V * 2.0;

        let (node, box_width) = if self.is_horizontal() {
            // Horizontal wrapping layout
            let available_width = limits.max().width;
            state.rows.clear();
            state.row_widths.clear();

            let mut current_row: Vec<(usize, f32)> = Vec::new();
            let mut row_x = 0.0_f32;

            for (i, &ew) in entry_widths.iter().enumerate() {
                let needed = if current_row.is_empty() { ew } else { ENTRY_GAP + ew };

                if self.wrap && !current_row.is_empty() && row_x + needed > available_width {
                    // Wrap: flush current row
                    state.row_widths.push(row_x);
                    state.rows.push(std::mem::take(&mut current_row));
                    row_x = 0.0;
                }

                let x_offset = if current_row.is_empty() { 0.0 } else { row_x + ENTRY_GAP };
                current_row.push((i, x_offset));
                row_x = x_offset + ew;
            }
            // Flush last row
            if !current_row.is_empty() {
                state.row_widths.push(row_x);
                state.rows.push(current_row);
            }

            let num_rows = state.rows.len().max(1);
            let total_height = num_rows as f32 * row_height;

            (Node::new(Size::new(available_width, total_height)), available_width)
        } else {
            // Vertical stacked layout (Left/Right)
            state.rows.clear();
            state.row_widths.clear();

            for (i, &ew) in entry_widths.iter().enumerate() {
                state.rows.push(vec![(i, 0.0)]);
                state.row_widths.push(ew);
            }

            let max_width = entry_widths.iter().copied().fold(0.0_f32, f32::max);
            let total_height = self.entries.len() as f32 * row_height;

            (
                Node::new(Size::new(max_width + ENTRY_GAP, total_height)),
                max_width + ENTRY_GAP,
            )
        };

        // Compute entry bounds relative to the legend's own origin (0, 0).
        // The widget adds the legend's final layout position at hit-test time.
        let mut entry_rects: Vec<Option<Rectangle>> = vec![None; self.entries.len()];
        for (row_idx, row) in state.rows.iter().enumerate() {
            let row_width = state.row_widths.get(row_idx).copied().unwrap_or(0.0);
            let row_y = row_idx as f32 * row_height + PADDING_V;
            let row_start_x = if self.is_horizontal() {
                (box_width - row_width).max(0.0) / 2.0
            } else {
                0.0
            };
            for &(entry_idx, x_offset) in row {
                let text_width = state.widths.get(entry_idx).copied().unwrap_or(0.0);
                entry_rects[entry_idx] = Some(Rectangle {
                    x: row_start_x + x_offset,
                    y: row_y,
                    width: swatch_size + SWATCH_TEXT_GAP + text_width,
                    height: font_size,
                });
            }
        }
        state.entry_rects = entry_rects;

        node
    }

    /// Draws the legend.
    #[allow(clippy::too_many_arguments)]
    pub fn draw<Theme>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &Theme,
        _style: &crate::core::renderer::Style,
        layout: crate::core::Layout<'_>,
        _cursor: crate::core::mouse::Cursor,
        viewport: &crate::core::Rectangle,
        palette: &crate::palette::Resolved,
        hidden_series: &HashSet<String>,
    ) where
        Theme: crate::design::Design + ?Sized,
        Renderer: crate::core::Renderer,
    {
        if self.entries.is_empty() {
            return;
        }

        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let bounds = layout.bounds();
        let background = design.background_color();
        let text_pair = design.text_pair();
        let font = design.font();
        let font_size = self.font_size;
        let swatch_size = font_size * SWATCH_SCALE;
        let text_color = design.text_color().resolve(background, text_pair, None);
        let row_height = font_size + PADDING_V * 2.0;

        for (row_idx, row) in state.rows.iter().enumerate() {
            let row_width = state.row_widths.get(row_idx).copied().unwrap_or(0.0);
            let row_y = bounds.y + row_idx as f32 * row_height + PADDING_V;

            // Center this row horizontally within bounds (for horizontal layout)
            let row_start_x = if self.is_horizontal() {
                bounds.x + (bounds.width - row_width).max(0.0) / 2.0
            } else {
                bounds.x
            };

            for &(entry_idx, x_offset) in row {
                let entry = &self.entries[entry_idx];
                let x = row_start_x + x_offset;

                let is_hidden = hidden_series.contains(&entry.name);
                let dim = |c: crate::core::Color| {
                    if is_hidden {
                        crate::core::Color { a: c.a * 0.35, ..c }
                    } else {
                        c
                    }
                };

                // Resolve swatch color
                let raw_swatch = if let Some(entry_color) = entry.color {
                    entry_color.resolve(background, text_pair, None)
                } else {
                    palette.get(entry_idx).resolve(background, text_pair, None)
                };
                let swatch_color = dim(raw_swatch);

                // Draw swatch
                renderer.fill_quad(
                    crate::core::renderer::Quad {
                        bounds: crate::core::Rectangle {
                            x,
                            y: row_y + (font_size - swatch_size) / 2.0,
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
                        font_features: Vec::new(),
                        font_variations: Vec::new(),
                        letter_spacing: Default::default(),
                        weight: None,
                    },
                    crate::core::Point::new(x + swatch_size + SWATCH_TEXT_GAP, row_y),
                    dim(text_color),
                    *viewport,
                );

                let _ = (entry_idx, x);
            }
        }
    }
}

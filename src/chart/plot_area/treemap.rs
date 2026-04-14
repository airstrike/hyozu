use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::widget::canvas::{Frame, Path, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Default dark blue/navy palette for treemap charts (business dashboard aesthetic).
const DEFAULT_PALETTE: &[(u8, u8, u8)] = &[
    (0x1B, 0x3A, 0x5C), // deep navy
    (0x2C, 0x5F, 0x8A), // medium blue
    (0x3A, 0x7C, 0xA5), // teal blue
    (0x1E, 0x4D, 0x6E), // dark teal
    (0x4A, 0x90, 0xB8), // steel blue
    (0x15, 0x2E, 0x4A), // midnight blue
    (0x34, 0x6B, 0x96), // ocean blue
    (0x5B, 0xA0, 0xC5), // light steel
    (0x0F, 0x25, 0x3C), // very dark navy
    (0x48, 0x7D, 0xA8), // dusty blue
];

/// State for Treemap — stores pre-calculated rectangle positions for
/// hit-testing AND pre-truncated label strings, so `draw` doesn't allocate
/// a new String for every item every frame.
pub struct State {
    /// Pixel rectangles for each item.
    pub item_rects: Vec<crate::core::Rectangle>,
    /// Pre-truncated display labels per item, sized to fit each rectangle's
    /// width using a 12 px font baseline (the bar/line label default).
    /// Empty string means "rectangle too small to label". Computed in
    /// `layout`; theme-font-size mismatches at draw time may cause minor
    /// over- or under-truncation that auto-corrects on the next layout
    /// (resize, data change).
    pub item_labels: Vec<String>,
}

/// A Treemap series that renders treemap charts.
pub struct Treemap<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::treemap::Treemap,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Treemap<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Treemap borrowing data.
    pub fn new(data: &'a crate::mark::treemap::Treemap) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Treemap.
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                item_rects: Vec::new(),
                item_labels: Vec::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Treemap state.
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the treemap — compute rectangle positions using squarified algorithm.
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, limits: &Limits, _plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();
        let size = limits.max();

        if self.data.items.is_empty() {
            state.item_rects.clear();
            return Node::new(Size::ZERO);
        }

        let gap = self.data.gap;

        // Sort items by value (descending) for squarified algorithm, keeping track of original indices
        let mut indexed_items: Vec<(usize, f32)> = self
            .data
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| item.value > 0.0)
            .map(|(i, item)| (i, item.value))
            .collect();
        indexed_items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let total: f32 = indexed_items.iter().map(|(_, v)| *v).sum();

        if total <= 0.0 {
            state.item_rects.clear();
            return Node::new(Size::ZERO);
        }

        // Compute rectangles using squarified treemap algorithm
        let rects = squarify(&indexed_items, total, crate::core::Rectangle {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        });

        // Map back to original item order
        state.item_rects = vec![
            crate::core::Rectangle {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            };
            self.data.items.len()
        ];

        for (orig_idx, rect) in rects {
            // Apply gap (inset each rectangle)
            let half_gap = gap / 2.0;
            let inset_rect = crate::core::Rectangle {
                x: rect.x + half_gap,
                y: rect.y + half_gap,
                width: (rect.width - gap).max(0.0),
                height: (rect.height - gap).max(0.0),
            };
            state.item_rects[orig_idx] = inset_rect;
        }

        // Pre-truncate per-item labels so `draw` doesn't allocate Strings
        // every frame. Uses a fixed 12 px baseline for char width because
        // `theme.font_size()` isn't available in `layout` (theme is draw-
        // only by design). Mismatches with non-default themes auto-correct
        // on the next relayout (resize, data change).
        const LABEL_FONT_BASELINE: f32 = 12.0;
        const LABEL_PADDING: f32 = 4.0;
        const MIN_LABEL_WIDTH: f32 = 20.0;
        const MIN_LABEL_HEIGHT: f32 = 14.0;
        let char_width = LABEL_FONT_BASELINE * 0.6;

        state.item_labels = self
            .data
            .items
            .iter()
            .zip(state.item_rects.iter())
            .map(|(item, rect)| {
                if rect.width < MIN_LABEL_WIDTH || rect.height < MIN_LABEL_HEIGHT {
                    return String::new();
                }
                let available = rect.width - LABEL_PADDING * 2.0;
                let max_chars = (available / char_width).floor() as usize;
                if item.label.len() <= max_chars {
                    item.label.clone()
                } else if max_chars > 3 {
                    // Truncate on a char boundary so multi-byte UTF-8 codepoints
                    // don't get sliced mid-character.
                    let byte_end = item
                        .label
                        .char_indices()
                        .nth(max_chars - 3)
                        .map(|(i, _)| i)
                        .unwrap_or(item.label.len());
                    format!("{}...", &item.label[..byte_end])
                } else {
                    let byte_end = item
                        .label
                        .char_indices()
                        .nth(max_chars)
                        .map(|(i, _)| i)
                        .unwrap_or(item.label.len());
                    item.label[..byte_end].to_string()
                }
            })
            .collect();

        Node::new(Size::ZERO)
    }

    /// Draws the treemap chart.
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

        if state.item_rects.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        // Per-item label padding/font baseline match the constants used in
        // `layout` so the truncation lines up with where `draw` puts the
        // text. Theme font size is still honored for the actual rendering;
        // only the truncation budget is fixed.
        let padding = 4.0;
        let font_size = theme.font_size();

        // Draw each rectangle
        for (i, (item, rect)) in self.data.items.iter().zip(state.item_rects.iter()).enumerate() {
            if rect.width <= 0.0 || rect.height <= 0.0 {
                continue;
            }

            // Resolve color: item color > palette > default navy palette
            let color = if let Some(item_color) = item.color {
                item_color.resolve(background, text_pair, None)
            } else {
                // Try the palette first; if color_offset == 0 and no explicit palette,
                // use the default navy palette for business dashboard look
                let palette_color = palette.get(color_offset + i);
                let resolved = palette_color.resolve(background, text_pair, None);

                // Check if this is likely a default theme palette (not user-specified)
                // If the item has no explicit color, use default navy palette as fallback
                if item.color.is_none() && self.uses_default_palette(i) {
                    let (r, g, b) = DEFAULT_PALETTE[i % DEFAULT_PALETTE.len()];
                    crate::core::Color::from_rgb8(r, g, b)
                } else {
                    resolved
                }
            };

            // Draw filled rectangle
            let path = Path::new(|builder| {
                builder.rectangle(
                    crate::core::Point::new(rect.x, rect.y),
                    crate::core::Size::new(rect.width, rect.height),
                );
            });
            frame.fill(&path, color);

            // Draw label — pre-truncated in layout. Empty string means
            // "rectangle too small to label".
            let label_text = state.item_labels.get(i).map(String::as_str).unwrap_or("");
            if !label_text.is_empty() {
                let label_color = text_pair.resolve(color, Some(background));
                frame.fill_text(CanvasText {
                    content: label_text.to_string(),
                    position: crate::core::Point::new(rect.x + padding, rect.y + padding),
                    color: label_color,
                    size: crate::core::Pixels(font_size),
                    font: crate::core::Font::default(),
                    align_x: crate::core::alignment::Horizontal::Left.into(),
                    align_y: crate::core::alignment::Vertical::Top,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);

        let geometry = frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(geometry);
        });
    }

    /// Check whether we should use the default navy palette for this item.
    /// Returns true when no items have explicit colors set.
    fn uses_default_palette(&self, _index: usize) -> bool {
        self.data.items.iter().all(|item| item.color.is_none())
    }
}

/// Squarified treemap algorithm.
///
/// Produces rectangles with good aspect ratios by greedily adding items
/// to rows, choosing the orientation that minimizes the worst aspect ratio.
///
/// Returns a list of (original_index, Rectangle) pairs.
fn squarify(
    items: &[(usize, f32)],
    total: f32,
    bounds: crate::core::Rectangle,
) -> Vec<(usize, crate::core::Rectangle)> {
    if items.is_empty() || total <= 0.0 || bounds.width <= 0.0 || bounds.height <= 0.0 {
        return Vec::new();
    }

    let total_area = bounds.width * bounds.height;
    let mut result = Vec::with_capacity(items.len());
    let mut remaining = bounds;
    let mut idx = 0;

    while idx < items.len() {
        let remaining_total: f32 = items[idx..].iter().map(|(_, v)| *v).sum();
        if remaining_total <= 0.0 {
            break;
        }

        // Determine orientation: lay out along the shorter side
        let is_wide = remaining.width >= remaining.height;
        let side = if is_wide { remaining.height } else { remaining.width };

        if side <= 0.0 {
            break;
        }

        // Greedily add items to the current row while aspect ratio improves
        let mut row: Vec<(usize, f32)> = Vec::new();
        let mut row_sum: f32 = 0.0;

        while idx < items.len() {
            let (orig_idx, value) = items[idx];
            let new_sum = row_sum + value;

            if row.is_empty() {
                // Always add the first item
                row.push((orig_idx, value));
                row_sum = new_sum;
                idx += 1;
            } else {
                // Check if adding this item improves the worst aspect ratio
                let area_scale = total_area / total;
                let worst_before = worst_ratio(&row, row_sum, side, area_scale);
                let mut test_row = row.clone();
                test_row.push((orig_idx, value));
                let worst_after = worst_ratio(&test_row, new_sum, side, area_scale);

                if worst_after <= worst_before {
                    row.push((orig_idx, value));
                    row_sum = new_sum;
                    idx += 1;
                } else {
                    break;
                }
            }
        }

        // Layout the row
        let row_area = (row_sum / total) * total_area;
        let row_thickness = row_area / side;

        let mut offset = 0.0;
        for (orig_idx, value) in &row {
            let item_area = (*value / total) * total_area;
            let item_length = if row_thickness > 0.0 {
                item_area / row_thickness
            } else {
                0.0
            };

            let rect = if is_wide {
                crate::core::Rectangle {
                    x: remaining.x,
                    y: remaining.y + offset,
                    width: row_thickness,
                    height: item_length,
                }
            } else {
                crate::core::Rectangle {
                    x: remaining.x + offset,
                    y: remaining.y,
                    width: item_length,
                    height: row_thickness,
                }
            };

            result.push((*orig_idx, rect));
            offset += item_length;
        }

        // Reduce remaining area
        if is_wide {
            remaining.x += row_thickness;
            remaining.width -= row_thickness;
        } else {
            remaining.y += row_thickness;
            remaining.height -= row_thickness;
        }
    }

    result
}

/// Compute the worst (highest) aspect ratio among items in a row.
///
/// For each item, compute the aspect ratio of the rectangle it would occupy
/// if laid out along a side of length `side`.
fn worst_ratio(row: &[(usize, f32)], row_sum: f32, side: f32, area_scale: f32) -> f32 {
    if row.is_empty() || side <= 0.0 || row_sum <= 0.0 {
        return f32::INFINITY;
    }

    let row_area = row_sum * area_scale;
    let thickness = row_area / side;

    if thickness <= 0.0 {
        return f32::INFINITY;
    }

    let mut worst = 0.0_f32;
    for (_, value) in row {
        let item_area = *value * area_scale;
        let item_length = item_area / thickness;

        let ratio = if thickness > item_length {
            thickness / item_length
        } else {
            item_length / thickness
        };

        worst = worst.max(ratio);
    }

    worst
}

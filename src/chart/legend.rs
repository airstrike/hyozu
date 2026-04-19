use std::collections::HashSet;

use crate::core::layout::{Limits, Node};
use crate::core::text::paragraph;
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size, text};
use crate::data::legend::{Anchor, Orientation};
use crate::data::mark::{LegendEntry, LegendSwatch};
use crate::line::LineStyle;
use crate::line::marker::Shape;
use crate::widget::canvas::{Frame, LineCap, LineDash, Path, Stroke};
use crate::widget::renderer::geometry;

/// Gap between swatch and text label.
const SWATCH_TEXT_GAP: f32 = 4.0;
/// Gap between entries.
const ENTRY_GAP: f32 = 16.0;
/// Vertical padding above/below the legend.
const PADDING_V: f32 = 4.0;
/// Square swatch size as a proportion of font size.
const SWATCH_SCALE: f32 = 0.85;
/// Line swatch width as a proportion of font size — wider than a square so the
/// stroke actually reads as a line rather than a dash.
const LINE_SWATCH_SCALE: f32 = 2.0;
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
    /// Row widths (total content width per row, used by draw to align rows).
    pub row_widths: Vec<f32>,
    /// Absolute-pixel bounding rect for each entry (indexed by entry index),
    /// cached on the last `draw()` call. Used for legend click hit-testing.
    pub entry_rects: Vec<Option<Rectangle>>,
}

/// A Legend displays a key for the chart's data series.
pub struct Legend<'a, Message, Renderer>
where
    Renderer: text::Renderer<Font = crate::core::Font> + geometry::Renderer,
{
    entries: Vec<LegendEntry>,
    anchor: Anchor,
    orientation: Orientation,
    text: crate::text::Style,
    wrap: bool,
    interactive: bool,
    _marker: std::marker::PhantomData<(Message, &'a Renderer)>,
}

impl<'a, Message, Renderer> Legend<'a, Message, Renderer>
where
    Renderer: text::Renderer<Font = crate::core::Font> + geometry::Renderer,
{
    /// Create a new Legend from entries and configuration.
    pub fn new(
        entries: Vec<LegendEntry>,
        anchor: Anchor,
        orientation: Orientation,
        text: crate::text::Style,
        wrap: bool,
        interactive: bool,
    ) -> Self {
        Self {
            entries,
            anchor,
            orientation,
            text,
            wrap,
            interactive,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the anchor of this legend.
    pub fn anchor(&self) -> Anchor {
        self.anchor
    }

    /// Returns the orientation of this legend.
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Returns whether this legend is click-interactive.
    pub fn interactive(&self) -> bool {
        self.interactive
    }

    /// Returns the entries on this legend.
    pub fn entries(&self) -> &[LegendEntry] {
        &self.entries
    }

    /// Returns true if this legend flows horizontally.
    fn is_horizontal(&self) -> bool {
        matches!(self.orientation, Orientation::Horizontal)
    }

    /// Returns the alignment of rows inside the legend box, along the
    /// flow direction. `0.0` = start, `0.5` = center, `1.0` = end.
    ///
    /// Only horizontal legends use this — vertical legends stack one
    /// entry per row, left-aligned inside the column.
    fn alignment(&self) -> f32 {
        if !self.is_horizontal() {
            return 0.0;
        }
        match self.anchor {
            Anchor::TopLeft | Anchor::BottomLeft | Anchor::Left => 0.0,
            Anchor::Top | Anchor::Bottom => 0.5,
            Anchor::TopRight | Anchor::BottomRight | Anchor::Right => 1.0,
        }
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
        let font_size = self.text.resolved_size(DEFAULT_FONT_SIZE);

        // Measure each entry's text width
        state.widths.clear();
        for entry in &self.entries {
            let _ = state.paragraph.update(text::Text {
                content: entry.name.as_str(),
                bounds: Size::INFINITE,
                size: font_size.into(),
                line_height: text::LineHeight::default(),
                font: self.text.resolved_font(renderer.default_font()),
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

        // Compute entry widths (swatch + gap + text) — swatch width varies by kind.
        let entry_widths: Vec<f32> = self
            .entries
            .iter()
            .zip(state.widths.iter())
            .map(|(entry, tw)| swatch_width(&entry.swatch, font_size) + SWATCH_TEXT_GAP + tw)
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
            // Vertical stacked layout
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
        let alignment = self.alignment();
        let mut entry_rects: Vec<Option<Rectangle>> = vec![None; self.entries.len()];
        for (row_idx, row) in state.rows.iter().enumerate() {
            let row_width = state.row_widths.get(row_idx).copied().unwrap_or(0.0);
            let row_y = row_idx as f32 * row_height + PADDING_V;
            let row_start_x = if self.is_horizontal() {
                (box_width - row_width).max(0.0) * alignment
            } else {
                0.0
            };
            for &(entry_idx, x_offset) in row {
                let text_width = state.widths.get(entry_idx).copied().unwrap_or(0.0);
                let sw = swatch_width(&self.entries[entry_idx].swatch, font_size);
                entry_rects[entry_idx] = Some(Rectangle {
                    x: row_start_x + x_offset,
                    y: row_y,
                    width: sw + SWATCH_TEXT_GAP + text_width,
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
        let seed = design.seed();
        let legend_default = design.legend_text();
        let font = self.text.resolved_font(legend_default.resolved_font(design.font()));
        let font_size = self.text.resolved_size(legend_default.resolved_size(DEFAULT_FONT_SIZE));
        let square_size = font_size * SWATCH_SCALE;
        let text_color = design.text_color().resolve(background, text_pair, &seed, None);
        let row_height = font_size + PADDING_V * 2.0;
        let alignment = self.alignment();

        for (row_idx, row) in state.rows.iter().enumerate() {
            let row_width = state.row_widths.get(row_idx).copied().unwrap_or(0.0);
            let row_y = bounds.y + row_idx as f32 * row_height + PADDING_V;

            let row_start_x = if self.is_horizontal() {
                bounds.x + (bounds.width - row_width).max(0.0) * alignment
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
                    entry_color.resolve(background, text_pair, &seed, None)
                } else {
                    palette.get(entry_idx).resolve(background, text_pair, &seed, None)
                };
                let swatch_color = dim(raw_swatch);

                let sw = swatch_width(&entry.swatch, font_size);

                match &entry.swatch {
                    LegendSwatch::Square => {
                        renderer.fill_quad(
                            crate::core::renderer::Quad {
                                bounds: crate::core::Rectangle {
                                    x,
                                    y: row_y + (font_size - square_size) / 2.0,
                                    width: square_size,
                                    height: square_size,
                                },
                                border: crate::core::Border {
                                    radius: 2.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            swatch_color,
                        );
                    }
                    LegendSwatch::Line { style, marker } => {
                        draw_line_swatch(
                            renderer,
                            x,
                            row_y,
                            sw,
                            font_size,
                            style,
                            marker.as_ref(),
                            swatch_color,
                            background,
                            text_pair,
                            &seed,
                            &dim,
                        );
                    }
                }

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
                    crate::core::Point::new(x + sw + SWATCH_TEXT_GAP, row_y),
                    dim(text_color),
                    *viewport,
                );

                let _ = (entry_idx, x);
            }
        }
    }
}

/// Returns the width of the swatch drawn next to a legend entry, in pixels.
fn swatch_width(swatch: &LegendSwatch, font_size: f32) -> f32 {
    match swatch {
        LegendSwatch::Square => font_size * SWATCH_SCALE,
        LegendSwatch::Line { .. } => font_size * LINE_SWATCH_SCALE,
    }
}

/// Draws a horizontal line swatch with an optional marker overlay.
#[allow(clippy::too_many_arguments)]
fn draw_line_swatch<Renderer>(
    renderer: &mut Renderer,
    x: f32,
    row_y: f32,
    width: f32,
    font_size: f32,
    style: &LineStyle,
    marker: Option<&crate::line::marker::Marker>,
    line_color: crate::core::Color,
    background: crate::core::Color,
    text_pair: crate::color::Pair,
    seed: &crate::palette::Seed,
    dim: &dyn Fn(crate::core::Color) -> crate::core::Color,
) where
    Renderer: geometry::Renderer,
{
    let cy = font_size / 2.0;
    let mut frame = Frame::new(renderer, Size::new(width, font_size));

    let line_path = Path::new(|builder| {
        builder.move_to(Point::new(0.0, cy));
        builder.line_to(Point::new(width, cy));
    });

    let dash_stack: &[f32] = match style {
        LineStyle::Solid => &[],
        LineStyle::Dashed => &[8.0, 4.0],
        LineStyle::Dotted => &[1.0, 3.0],
        LineStyle::Custom { segments } => segments.as_slice(),
    };

    let mut stroke = Stroke::default().with_width(1.5).with_color(line_color);
    stroke.line_dash = LineDash {
        segments: dash_stack,
        offset: 0,
    };
    if matches!(style, LineStyle::Dotted) {
        stroke = stroke.with_line_cap(LineCap::Round);
    }
    frame.stroke(&line_path, stroke);

    if let Some(marker) = marker {
        let marker_fill = marker
            .color
            .map(|c| c.resolve(background, text_pair, seed, None))
            .unwrap_or(line_color);
        let marker_fill = dim(marker_fill);
        // Cap marker so it fits in the swatch height while respecting config.
        let size = marker.size.min(font_size).max(2.0);
        let half = size / 2.0;
        let cx = width / 2.0;
        let center = Point::new(cx, cy);

        let marker_path = Path::new(|builder| match marker.shape {
            Shape::Circle => {
                builder.circle(center, half);
            }
            Shape::Square => {
                builder.rectangle(Point::new(cx - half, cy - half), Size::new(size, size));
            }
            Shape::Diamond => {
                builder.move_to(Point::new(cx, cy - half));
                builder.line_to(Point::new(cx + half, cy));
                builder.line_to(Point::new(cx, cy + half));
                builder.line_to(Point::new(cx - half, cy));
                builder.close();
            }
            Shape::Triangle => {
                builder.move_to(Point::new(cx, cy - half));
                builder.line_to(Point::new(cx + half, cy + half));
                builder.line_to(Point::new(cx - half, cy + half));
                builder.close();
            }
            Shape::TriangleDown => {
                builder.move_to(Point::new(cx, cy + half));
                builder.line_to(Point::new(cx + half, cy - half));
                builder.line_to(Point::new(cx - half, cy - half));
                builder.close();
            }
            Shape::Cross => {
                let arm = half * 0.3;
                builder.move_to(Point::new(cx - arm, cy - half));
                builder.line_to(Point::new(cx + arm, cy - half));
                builder.line_to(Point::new(cx + arm, cy - arm));
                builder.line_to(Point::new(cx + half, cy - arm));
                builder.line_to(Point::new(cx + half, cy + arm));
                builder.line_to(Point::new(cx + arm, cy + arm));
                builder.line_to(Point::new(cx + arm, cy + half));
                builder.line_to(Point::new(cx - arm, cy + half));
                builder.line_to(Point::new(cx - arm, cy + arm));
                builder.line_to(Point::new(cx - half, cy + arm));
                builder.line_to(Point::new(cx - half, cy - arm));
                builder.line_to(Point::new(cx - arm, cy - arm));
                builder.close();
            }
            Shape::X => {
                let diag = half * 0.707;
                builder.move_to(Point::new(cx - diag, cy - diag));
                builder.line_to(Point::new(cx + diag, cy + diag));
                builder.move_to(Point::new(cx + diag, cy - diag));
                builder.line_to(Point::new(cx - diag, cy + diag));
            }
        });

        if marker.shape != Shape::X {
            frame.fill(&marker_path, marker_fill);
        }

        if let Some(stroke_spec) = marker.stroke {
            let stroke_color = dim(stroke_spec.resolve(background, text_pair, seed, None));
            frame.stroke(
                &marker_path,
                Stroke::default()
                    .with_width(marker.stroke_width)
                    .with_color(stroke_color),
            );
        } else if marker.shape == Shape::X {
            frame.stroke(
                &marker_path,
                Stroke::default()
                    .with_width(marker.stroke_width.max(2.0))
                    .with_color(marker_fill),
            );
        }
    }

    let geometry = frame.into_geometry();
    renderer.with_translation(crate::core::Vector::new(x, row_y), |renderer| {
        renderer.draw_geometry(geometry);
    });
}

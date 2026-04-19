//! Color-scale legend drawn on top of (or alongside) the plot area.
//!
//! Shared between marks that carry a continuous-scale key — currently the
//! choropleth — and driven by a [`data::legend::Legend`] config plus a
//! theme-free [`Plan`] that the mark's `layout` step caches on state. The
//! draw step reads the plan, resolves theme-dependent bits (colors, font),
//! and emits geometry onto an existing [`Frame`].

use crate::core::{Color, Point, Rectangle, alignment};
use crate::data::legend::{Anchor, Legend, Orientation, Placement};
use crate::palette;
use crate::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};
use crate::widget::renderer::geometry;

/// Theme-free plan for the scale legend, materialized in `layout` so
/// `draw` doesn't reformat labels or re-walk the value range.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    /// Formatted min label (end of gradient nearest the "low" side).
    pub min_label: String,
    /// Formatted max label.
    pub max_label: String,
}

/// Pixel margin from the plot-area edge for overlaid scale legends.
const EDGE_PADDING: f32 = 16.0;
/// Inner padding inside the legend background panel.
const PANEL_PADDING: f32 = 8.0;
/// Default font size for legend text.
const FONT_SIZE: f32 = 10.0;
/// Gradient bar thickness on the non-flow axis.
const BAR_THICKNESS: f32 = 10.0;
/// Tick line length.
const TICK_LENGTH: f32 = 4.0;
/// Number of gradient segments rendered.
const GRADIENT_SEGMENTS: usize = 64;
/// Target bar length along the flow axis for horizontal legends.
const HORIZONTAL_BAR_LENGTH: f32 = 200.0;
/// Target bar length along the flow axis for vertical legends.
const VERTICAL_BAR_LENGTH: f32 = 140.0;
/// Minimum chart width (horizontal) or height (vertical) below which the
/// scale legend is suppressed to avoid cramping the plot area.
const MIN_CHART_EXTENT: f32 = 300.0;

/// Render the scale legend for a mark onto `frame`.
///
/// `plot_bounds` is the plot-area rectangle in the same coordinate space as
/// the `Frame`. `color_stops` and `label_color` are theme-dependent inputs
/// materialized in the caller's `draw`.
#[allow(clippy::too_many_arguments)]
pub fn draw<Theme, Renderer>(
    frame: &mut Frame<Renderer>,
    plan: &Plan,
    legend: &Legend,
    title: Option<&str>,
    plot_bounds: Rectangle,
    color_stops: &[Color],
    background: Color,
    border_color: Color,
    label_color: Color,
    theme: &Theme,
) where
    Theme: crate::design::Design + ?Sized,
    Renderer: geometry::Renderer,
{
    // Inset placement degrades to Overlaid here: scene-level space
    // reservation for mark-owned scale legends is not yet wired, so the
    // legend always draws on top of the plot area. The placement value
    // is retained on `Legend` so callers can still express intent.
    let _ = legend.placement_value();

    let orientation = legend.orientation_value();
    let anchor = legend.anchor_value();

    // ── Extents check ─────────────────────────────────────────────────
    let major_extent = match orientation {
        Orientation::Horizontal => plot_bounds.width,
        Orientation::Vertical => plot_bounds.height,
    };
    if major_extent < MIN_CHART_EXTENT {
        return;
    }

    // ── Geometry ──────────────────────────────────────────────────────
    let has_title = title.is_some();
    let title_line_height = if has_title { FONT_SIZE + 4.0 } else { 0.0 };
    let label_line_height = FONT_SIZE + 2.0;

    let (panel_size, bar_offset, bar_size) = match orientation {
        Orientation::Horizontal => {
            let bar_length = if plot_bounds.width < 400.0 {
                (plot_bounds.width * 0.45).min(HORIZONTAL_BAR_LENGTH)
            } else {
                HORIZONTAL_BAR_LENGTH
            };
            let content_w = bar_length;
            let content_h = title_line_height + BAR_THICKNESS + TICK_LENGTH + label_line_height;
            let panel_w = content_w + PANEL_PADDING * 2.0;
            let panel_h = content_h + PANEL_PADDING * 2.0;
            let bar_offset_y = PANEL_PADDING + title_line_height;
            (
                crate::core::Size::new(panel_w, panel_h),
                crate::core::Vector::new(PANEL_PADDING, bar_offset_y),
                crate::core::Size::new(bar_length, BAR_THICKNESS),
            )
        }
        Orientation::Vertical => {
            let bar_length = if plot_bounds.height < 400.0 {
                (plot_bounds.height * 0.45).min(VERTICAL_BAR_LENGTH)
            } else {
                VERTICAL_BAR_LENGTH
            };
            // Label column: widest of min / max / title, rough char-width
            // estimate (5px per glyph at 10pt) since we don't have a
            // paragraph cache here — the legend stays on the plan-cached
            // labels so this is still O(1) per draw.
            let max_label_chars = plan
                .min_label
                .chars()
                .count()
                .max(plan.max_label.chars().count())
                .max(title.map(|t| t.chars().count()).unwrap_or(0));
            let label_col_w = (max_label_chars as f32) * 6.0 + 4.0;
            let content_w = BAR_THICKNESS + TICK_LENGTH + label_col_w;
            let content_h = title_line_height + bar_length;
            let panel_w = content_w + PANEL_PADDING * 2.0;
            let panel_h = content_h + PANEL_PADDING * 2.0;
            let bar_offset_y = PANEL_PADDING + title_line_height;
            (
                crate::core::Size::new(panel_w, panel_h),
                crate::core::Vector::new(PANEL_PADDING, bar_offset_y),
                crate::core::Size::new(BAR_THICKNESS, bar_length),
            )
        }
    };

    let panel_origin = anchor_origin(anchor, plot_bounds, panel_size);

    // 1. Background panel.
    let panel_bg = Color { a: 0.85, ..background };
    let panel_rect = Path::new(|builder| {
        builder.rectangle(panel_origin, panel_size);
    });
    frame.fill(&panel_rect, panel_bg);
    frame.stroke(&panel_rect, Stroke::default().with_color(border_color).with_width(0.5));

    let bar_origin = Point::new(panel_origin.x + bar_offset.x, panel_origin.y + bar_offset.y);

    // 2. Optional title.
    if let Some(title_text) = title {
        let title_pos = match orientation {
            Orientation::Horizontal => Point::new(bar_origin.x + bar_size.width / 2.0, panel_origin.y + PANEL_PADDING),
            Orientation::Vertical => {
                Point::new(panel_origin.x + panel_size.width / 2.0, panel_origin.y + PANEL_PADDING)
            }
        };
        frame.fill_text(CanvasText {
            content: title_text.to_string(),
            position: title_pos,
            color: label_color,
            size: crate::core::Pixels(FONT_SIZE),
            font: theme.font(),
            align_x: alignment::Horizontal::Center.into(),
            align_y: alignment::Vertical::Top,
            line_height: crate::core::text::LineHeight::default(),
            shaping: crate::core::text::Shaping::Basic,
            ..CanvasText::default()
        });
    }

    // 3. Gradient segments.
    match orientation {
        Orientation::Horizontal => draw_horizontal_gradient(frame, bar_origin, bar_size, color_stops),
        Orientation::Vertical => draw_vertical_gradient(frame, bar_origin, bar_size, color_stops),
    }

    // 4. Bar outline.
    let outline = Path::new(|builder| {
        builder.rectangle(bar_origin, bar_size);
    });
    frame.stroke(&outline, Stroke::default().with_color(border_color).with_width(0.5));

    // 5. Ticks + labels.
    match orientation {
        Orientation::Horizontal => {
            let tick_y_top = bar_origin.y + bar_size.height;
            let tick_y_bot = tick_y_top + TICK_LENGTH;
            let left_tick = Path::new(|builder| {
                builder.move_to(Point::new(bar_origin.x, tick_y_top));
                builder.line_to(Point::new(bar_origin.x, tick_y_bot));
            });
            frame.stroke(&left_tick, Stroke::default().with_color(border_color).with_width(0.5));
            let right_tick = Path::new(|builder| {
                builder.move_to(Point::new(bar_origin.x + bar_size.width, tick_y_top));
                builder.line_to(Point::new(bar_origin.x + bar_size.width, tick_y_bot));
            });
            frame.stroke(&right_tick, Stroke::default().with_color(border_color).with_width(0.5));

            let labels_y = tick_y_bot + 1.0;
            frame.fill_text(CanvasText {
                content: plan.min_label.clone(),
                position: Point::new(bar_origin.x, labels_y),
                color: label_color,
                size: crate::core::Pixels(FONT_SIZE),
                font: theme.font(),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Top,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
            frame.fill_text(CanvasText {
                content: plan.max_label.clone(),
                position: Point::new(bar_origin.x + bar_size.width, labels_y),
                color: label_color,
                size: crate::core::Pixels(FONT_SIZE),
                font: theme.font(),
                align_x: alignment::Horizontal::Right.into(),
                align_y: alignment::Vertical::Top,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
        }
        Orientation::Vertical => {
            // Labels go to the right of the bar; tick marks on the bar edge.
            let tick_x_left = bar_origin.x + bar_size.width;
            let tick_x_right = tick_x_left + TICK_LENGTH;
            let top_tick = Path::new(|builder| {
                builder.move_to(Point::new(tick_x_left, bar_origin.y));
                builder.line_to(Point::new(tick_x_right, bar_origin.y));
            });
            frame.stroke(&top_tick, Stroke::default().with_color(border_color).with_width(0.5));
            let bot_tick = Path::new(|builder| {
                builder.move_to(Point::new(tick_x_left, bar_origin.y + bar_size.height));
                builder.line_to(Point::new(tick_x_right, bar_origin.y + bar_size.height));
            });
            frame.stroke(&bot_tick, Stroke::default().with_color(border_color).with_width(0.5));

            let labels_x = tick_x_right + 2.0;
            // Max at the top (gradient's high end), min at the bottom.
            frame.fill_text(CanvasText {
                content: plan.max_label.clone(),
                position: Point::new(labels_x, bar_origin.y),
                color: label_color,
                size: crate::core::Pixels(FONT_SIZE),
                font: theme.font(),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Top,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
            frame.fill_text(CanvasText {
                content: plan.min_label.clone(),
                position: Point::new(labels_x, bar_origin.y + bar_size.height),
                color: label_color,
                size: crate::core::Pixels(FONT_SIZE),
                font: theme.font(),
                align_x: alignment::Horizontal::Left.into(),
                align_y: alignment::Vertical::Bottom,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
        }
    }
}

fn anchor_origin(anchor: Anchor, plot: Rectangle, panel: crate::core::Size) -> Point {
    let (x, y) = match anchor {
        Anchor::TopLeft => (plot.x + EDGE_PADDING, plot.y + EDGE_PADDING),
        Anchor::Top => (plot.x + (plot.width - panel.width) / 2.0, plot.y + EDGE_PADDING),
        Anchor::TopRight => (plot.x + plot.width - EDGE_PADDING - panel.width, plot.y + EDGE_PADDING),
        Anchor::Right => (
            plot.x + plot.width - EDGE_PADDING - panel.width,
            plot.y + (plot.height - panel.height) / 2.0,
        ),
        Anchor::BottomRight => (
            plot.x + plot.width - EDGE_PADDING - panel.width,
            plot.y + plot.height - EDGE_PADDING - panel.height,
        ),
        Anchor::Bottom => (
            plot.x + (plot.width - panel.width) / 2.0,
            plot.y + plot.height - EDGE_PADDING - panel.height,
        ),
        Anchor::BottomLeft => (
            plot.x + EDGE_PADDING,
            plot.y + plot.height - EDGE_PADDING - panel.height,
        ),
        Anchor::Left => (plot.x + EDGE_PADDING, plot.y + (plot.height - panel.height) / 2.0),
    };
    Point::new(x, y)
}

fn draw_horizontal_gradient<Renderer: geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    origin: Point,
    size: crate::core::Size,
    color_stops: &[Color],
) {
    let segment_width = size.width / GRADIENT_SEGMENTS as f32;
    for i in 0..GRADIENT_SEGMENTS {
        let t = i as f32 / (GRADIENT_SEGMENTS - 1) as f32;
        let seg_color = palette::sample_gradient(color_stops, t);
        let seg_x = origin.x + i as f32 * segment_width;
        // 0.5px overlap to hide hairlines between segments.
        let seg_w = segment_width + 0.5;
        let seg_path = Path::new(|builder| {
            builder.rectangle(Point::new(seg_x, origin.y), crate::core::Size::new(seg_w, size.height));
        });
        frame.fill(&seg_path, seg_color);
    }
}

fn draw_vertical_gradient<Renderer: geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    origin: Point,
    size: crate::core::Size,
    color_stops: &[Color],
) {
    let segment_height = size.height / GRADIENT_SEGMENTS as f32;
    for i in 0..GRADIENT_SEGMENTS {
        // Vertical scale convention: min at bottom (t=0 at bottom), max at top.
        let t = 1.0 - (i as f32 / (GRADIENT_SEGMENTS - 1) as f32);
        let seg_color = palette::sample_gradient(color_stops, t);
        let seg_y = origin.y + i as f32 * segment_height;
        let seg_h = segment_height + 0.5;
        let seg_path = Path::new(|builder| {
            builder.rectangle(Point::new(origin.x, seg_y), crate::core::Size::new(size.width, seg_h));
        });
        frame.fill(&seg_path, seg_color);
    }
}

/// Returns `true` when the legend's placement is `Overlaid`. Exposed so
/// the caller can decide whether to short-circuit or defer to scene-level
/// space reservation (not yet implemented at the scene layer).
pub fn is_overlaid(legend: &Legend) -> bool {
    matches!(legend.placement_value(), Placement::Overlaid)
}

//! Color-scale legend drawn on top of (or alongside) the plot area.
//!
//! Shared between marks that carry a continuous-scale key — currently the
//! choropleth — and driven by a [`data::legend::Config`] plus a
//! theme-free [`Plan`] that the mark's `layout` step caches on state. The
//! draw step reads the plan, resolves theme-dependent bits (colors, font),
//! and emits geometry onto an existing [`Frame`].
//!
//! # Placement
//!
//! Two placement modes are supported:
//!
//! - [`Placement::Overlaid`]: the panel floats inside the plot area at the
//!   anchor's corner/edge. The plot area keeps its full size.
//! - [`Placement::Inset`]: the scene reserves a strip along the anchored
//!   edge and the legend draws inside that strip. The plot area shrinks by
//!   the reservation budget. [`reservation`] returns the edge + pixel
//!   budget a scene layout step should carve off.

use crate::core::{Color, Point, Rectangle, Size, Vector, alignment};
use crate::data::legend::{Anchor, Config, Edge, Orientation, Placement};
use crate::palette;
use crate::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};
use crate::widget::renderer::geometry;

/// One labeled tick along the gradient legend.
#[derive(Debug, Clone)]
pub struct Tick {
    /// Position along the gradient as a unit interval — `0.0` is the
    /// "low" end (left for horizontal, bottom for vertical), `1.0` is
    /// the "high" end. Pre-computed so `draw` doesn't re-divide on
    /// every frame.
    pub t: f32,
    /// Pre-formatted label rendered next to the tick line.
    pub label: String,
}

/// Theme-free plan for the scale legend, materialized in `layout` so
/// `draw` doesn't reformat labels or re-walk the value range.
///
/// `ticks` is sorted from low to high. The first and last entries are
/// always the domain endpoints (`t = 0.0` and `t = 1.0`); interior
/// entries are placed at "nice" round values picked by the same
/// Heckbert step the axis ticks use, so the gradient gets the same
/// 3-7 reading anchors a D3 / Vega-Lite / ggplot continuous legend
/// shows.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub ticks: Vec<Tick>,
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
/// Estimated width per glyph at `FONT_SIZE` when we don't have a paragraph
/// cache on hand. Keeps label-column math O(1).
const GLYPH_WIDTH_ESTIMATE: f32 = 6.0;

/// Pixel reservation a scene layout should carve off the plot area for a
/// scale legend with [`Placement::Inset`].
///
/// Returns `None` when the legend is [`Placement::Overlaid`] (no space
/// reservation) — the caller should fall through to the standard overlay
/// path instead. Horizontal legends reserve vertical space on the top/
/// bottom edge; vertical legends reserve horizontal space on the left/
/// right edge. The numeric budget includes both panel padding and label
/// extents and is independent of the current plot size — the scene needs
/// a value it can subtract before the plot dimensions are known.
pub fn reservation(legend: &Config, title: Option<&str>) -> Option<(Edge, f32)> {
    if !matches!(legend.placement_value(), Placement::Inset) {
        return None;
    }
    let has_title = title.is_some();
    let title_line_height = if has_title { FONT_SIZE + 4.0 } else { 0.0 };
    let label_line_height = FONT_SIZE + 2.0;
    let budget = match legend.orientation_value() {
        Orientation::Horizontal => {
            title_line_height + BAR_THICKNESS + TICK_LENGTH + label_line_height + PANEL_PADDING * 2.0
        }
        Orientation::Vertical => {
            // Vertical: budget is bar thickness + tick + label column +
            // padding. Labels are usually short (min/max formatted
            // numbers) but the title can widen the strip when it beats
            // the numeric labels.
            BAR_THICKNESS + TICK_LENGTH + label_column_width(title) + PANEL_PADDING * 2.0
        }
    };
    Some((legend.edge(), budget))
}

/// Resolved panel geometry for a draw call.
struct PanelLayout {
    origin: Point,
    size: Size,
    /// Offset from panel origin to bar origin.
    bar_offset: Vector,
    bar_size: Size,
}

/// Size a draw panel based on the plot-area span on the flow axis.
///
/// Used for [`Placement::Overlaid`] where the panel floats inside the plot
/// area: the bar length scales down gracefully on narrow charts.
fn panel_for_overlay(legend: &Config, title: Option<&str>, plot_bounds: Rectangle) -> PanelLayout {
    let orientation = legend.orientation_value();
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
                Size::new(panel_w, panel_h),
                Vector::new(PANEL_PADDING, bar_offset_y),
                Size::new(bar_length, BAR_THICKNESS),
            )
        }
        Orientation::Vertical => {
            let bar_length = if plot_bounds.height < 400.0 {
                (plot_bounds.height * 0.45).min(VERTICAL_BAR_LENGTH)
            } else {
                VERTICAL_BAR_LENGTH
            };
            // Panel width fits the wider of (a) the title or (b) the
            // bar + numeric-label cluster. The title typically wins
            // (e.g. "July 2025 revenue ($K)" is much wider than
            // "$500K"), so the cluster gets centered horizontally
            // beneath the centered title — matching D3 / Vega-Lite
            // / ggplot vertical legend layout.
            const NUMERIC_LABEL_CHARS: f32 = 6.0;
            let numeric_label_w = NUMERIC_LABEL_CHARS * GLYPH_WIDTH_ESTIMATE;
            let cluster_w = BAR_THICKNESS + TICK_LENGTH + 2.0 + numeric_label_w;
            let title_w = label_column_width(title);
            let content_w = cluster_w.max(title_w);
            let content_h = title_line_height + bar_length;
            let panel_w = content_w + PANEL_PADDING * 2.0;
            let panel_h = content_h + PANEL_PADDING * 2.0;
            let bar_offset_y = PANEL_PADDING + title_line_height;
            let bar_offset_x = PANEL_PADDING + (content_w - cluster_w) / 2.0;
            (
                Size::new(panel_w, panel_h),
                Vector::new(bar_offset_x, bar_offset_y),
                Size::new(BAR_THICKNESS, bar_length),
            )
        }
    };

    let origin = anchor_origin(legend.anchor_value(), plot_bounds, panel_size);
    PanelLayout {
        origin,
        size: panel_size,
        bar_offset,
        bar_size,
    }
}

/// Size a panel that fills a reserved strip rectangle.
///
/// Used for [`Placement::Inset`]: the strip has a fixed thickness decided
/// by [`reservation`], and the bar stretches along the flow axis up to the
/// configured maximum.
fn panel_for_strip(legend: &Config, title: Option<&str>, strip: Rectangle) -> PanelLayout {
    let orientation = legend.orientation_value();
    let has_title = title.is_some();
    let title_line_height = if has_title { FONT_SIZE + 4.0 } else { 0.0 };

    let (panel_size, bar_offset, bar_size) = match orientation {
        Orientation::Horizontal => {
            // Strip fills the edge; bar length capped at the configured
            // maximum so a very wide map doesn't produce an unreadably
            // long gradient.
            let strip_bar_budget = (strip.width - PANEL_PADDING * 2.0).max(0.0);
            let bar_length = strip_bar_budget.min(HORIZONTAL_BAR_LENGTH);
            let panel_w = bar_length + PANEL_PADDING * 2.0;
            let panel_h = strip.height;
            let bar_offset_y = PANEL_PADDING + title_line_height;
            (
                Size::new(panel_w, panel_h),
                Vector::new(PANEL_PADDING, bar_offset_y),
                Size::new(bar_length, BAR_THICKNESS),
            )
        }
        Orientation::Vertical => {
            let strip_bar_budget = (strip.height - PANEL_PADDING * 2.0 - title_line_height).max(0.0);
            let bar_length = strip_bar_budget.min(VERTICAL_BAR_LENGTH);
            let panel_w = strip.width;
            let panel_h = title_line_height + bar_length + PANEL_PADDING * 2.0;
            let bar_offset_y = PANEL_PADDING + title_line_height;
            // Center the bar+labels cluster (bar + tick gap + numeric
            // label) horizontally inside the panel. The numeric label
            // column is short — formatted values like "$500K" or
            // "1.2M" cap around 6 glyphs — so use that width instead
            // of `label_column_width`, which sizes for the title and
            // would shrink the centering offset to almost zero.
            const NUMERIC_LABEL_CHARS: f32 = 6.0;
            let numeric_label_w = NUMERIC_LABEL_CHARS * GLYPH_WIDTH_ESTIMATE;
            let cluster_w = BAR_THICKNESS + TICK_LENGTH + 2.0 + numeric_label_w;
            let bar_offset_x = ((panel_w - cluster_w) * 0.5).max(PANEL_PADDING);
            (
                Size::new(panel_w, panel_h),
                Vector::new(bar_offset_x, bar_offset_y),
                Size::new(BAR_THICKNESS, bar_length),
            )
        }
    };

    // Center the panel within the strip on the flow axis so the bar sits
    // roughly under the middle of the plot area.
    let origin = match orientation {
        Orientation::Horizontal => Point::new(strip.x + (strip.width - panel_size.width) / 2.0, strip.y),
        Orientation::Vertical => Point::new(strip.x, strip.y + (strip.height - panel_size.height) / 2.0),
    };

    PanelLayout {
        origin,
        size: panel_size,
        bar_offset,
        bar_size,
    }
}

/// Estimates the label column width for a vertical legend.
///
/// The numeric min/max labels are short (6 glyphs is a safe upper bound
/// for formatted values like `"999.9M"` or `"1.2e3"`); we widen the
/// column when the legend title beats that.
fn label_column_width(title: Option<&str>) -> f32 {
    let title_chars = title.map(|t| t.chars().count()).unwrap_or(0);
    let chars = title_chars.max(6);
    (chars as f32) * GLYPH_WIDTH_ESTIMATE + 4.0
}

/// Render the scale legend for a mark onto `frame`.
///
/// `plot_bounds` is the plot-area rectangle in the same coordinate space as
/// the `Frame`. When `strip_rect` is `Some`, the legend draws inside the
/// reserved strip (honoring [`Placement::Inset`]); otherwise it overlays
/// the plot area at the anchor's corner/edge.
#[allow(clippy::too_many_arguments)]
pub fn draw<Theme, Renderer>(
    frame: &mut Frame<Renderer>,
    plan: &Plan,
    legend: &Config,
    title: Option<&str>,
    plot_bounds: Rectangle,
    strip_rect: Option<Rectangle>,
    color_stops: &[Color],
    background: Color,
    border_color: Color,
    label_color: Color,
    theme: &Theme,
) where
    Theme: crate::design::Design + ?Sized,
    Renderer: geometry::Renderer,
{
    let orientation = legend.orientation_value();

    // ── Extents check ─────────────────────────────────────────────────
    // For Inset we use the strip's own extent; for Overlaid we fall back
    // to plot_bounds. This prevents the legend from drawing inside a
    // cramped plot area without suppressing it when the strip itself is
    // comfortably sized.
    let gating_extent = match (strip_rect, orientation) {
        (Some(strip), Orientation::Horizontal) => strip.width,
        (Some(strip), Orientation::Vertical) => strip.height,
        (None, Orientation::Horizontal) => plot_bounds.width,
        (None, Orientation::Vertical) => plot_bounds.height,
    };
    if gating_extent < MIN_CHART_EXTENT {
        return;
    }

    // ── Geometry ──────────────────────────────────────────────────────
    let layout = match strip_rect {
        Some(strip) => panel_for_strip(legend, title, strip),
        None => panel_for_overlay(legend, title, plot_bounds),
    };
    let panel_origin = layout.origin;
    let panel_size = layout.size;
    let bar_offset = layout.bar_offset;
    let bar_size = layout.bar_size;

    // 1. Background panel. Inset strips blend into the chart background so
    // they don't look like a floating overlay.
    let panel_bg = match strip_rect {
        Some(_) => Color { a: 0.0, ..background },
        None => Color { a: 0.85, ..background },
    };
    let panel_rect = Path::new(|builder| {
        builder.rectangle(panel_origin, panel_size);
    });
    if panel_bg.a > 0.0 {
        frame.fill(&panel_rect, panel_bg);
        frame.stroke(&panel_rect, Stroke::default().with_color(border_color).with_width(0.5));
    }

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
            let labels_y = tick_y_bot + 1.0;
            for (i, tick) in plan.ticks.iter().enumerate() {
                let x = bar_origin.x + bar_size.width * tick.t;
                let line = Path::new(|builder| {
                    builder.move_to(Point::new(x, tick_y_top));
                    builder.line_to(Point::new(x, tick_y_bot));
                });
                frame.stroke(&line, Stroke::default().with_color(border_color).with_width(0.5));
                // Endpoints align to their corner (the bar's outer
                // edge); interior ticks center their label on the
                // tick line.
                let align = if i == 0 {
                    alignment::Horizontal::Left
                } else if i == plan.ticks.len() - 1 {
                    alignment::Horizontal::Right
                } else {
                    alignment::Horizontal::Center
                };
                frame.fill_text(CanvasText {
                    content: tick.label.clone(),
                    position: Point::new(x, labels_y),
                    color: label_color,
                    size: crate::core::Pixels(FONT_SIZE),
                    font: theme.font(),
                    align_x: align.into(),
                    align_y: alignment::Vertical::Top,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }
        Orientation::Vertical => {
            // Labels go to the right of the bar; tick marks on the bar edge.
            let tick_x_left = bar_origin.x + bar_size.width;
            let tick_x_right = tick_x_left + TICK_LENGTH;
            let labels_x = tick_x_right + 2.0;
            for (i, tick) in plan.ticks.iter().enumerate() {
                // Vertical legends paint `t = 1.0` (high end) at the
                // top and `t = 0.0` at the bottom — invert here so the
                // y position grows downward as the value drops.
                let y = bar_origin.y + bar_size.height * (1.0 - tick.t);
                let line = Path::new(|builder| {
                    builder.move_to(Point::new(tick_x_left, y));
                    builder.line_to(Point::new(tick_x_right, y));
                });
                frame.stroke(&line, Stroke::default().with_color(border_color).with_width(0.5));
                // Top endpoint anchors its baseline at the tick (Top
                // align); bottom endpoint hangs from the tick (Bottom
                // align); interior ticks center their label vertically
                // on the tick line.
                let align_y = if i == plan.ticks.len() - 1 {
                    alignment::Vertical::Top
                } else if i == 0 {
                    alignment::Vertical::Bottom
                } else {
                    alignment::Vertical::Center
                };
                frame.fill_text(CanvasText {
                    content: tick.label.clone(),
                    position: Point::new(labels_x, y),
                    color: label_color,
                    size: crate::core::Pixels(FONT_SIZE),
                    font: theme.font(),
                    align_x: alignment::Horizontal::Left.into(),
                    align_y,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }
    }
}

fn anchor_origin(anchor: Anchor, plot: Rectangle, panel: Size) -> Point {
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
    size: Size,
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
            builder.rectangle(Point::new(seg_x, origin.y), Size::new(seg_w, size.height));
        });
        frame.fill(&seg_path, seg_color);
    }
}

fn draw_vertical_gradient<Renderer: geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    origin: Point,
    size: Size,
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
            builder.rectangle(Point::new(origin.x, seg_y), Size::new(size.width, seg_h));
        });
        frame.fill(&seg_path, seg_color);
    }
}

/// Returns `true` when the legend's placement is `Overlaid`. Exposed so
/// the caller can decide whether to short-circuit or defer to scene-level
/// space reservation.
pub fn is_overlaid(legend: &Config) -> bool {
    matches!(legend.placement_value(), Placement::Overlaid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::legend::{Anchor, Config, Orientation, Placement};

    #[test]
    fn overlaid_legend_reports_no_reservation() {
        let l = Config::overlay(Anchor::BottomRight);
        assert_eq!(l.placement_value(), Placement::Overlaid);
        assert!(reservation(&l, None).is_none());
    }

    #[test]
    fn inset_horizontal_bottom_reserves_vertical_strip_with_title() {
        let l = Config::below();
        let (edge, size) = reservation(&l, Some("GDP")).expect("inset legend reserves");
        assert_eq!(edge, Edge::Bottom);
        // title(14) + bar(10) + tick(4) + label(12) + padding(16) = 56
        assert!((size - 56.0).abs() < 0.1, "unexpected horizontal+title budget: {size}");
    }

    #[test]
    fn inset_horizontal_bottom_reserves_vertical_strip_without_title() {
        let l = Config::below();
        let (_, size) = reservation(&l, None).unwrap();
        // bar(10) + tick(4) + label(12) + padding(16) = 42
        assert!(
            (size - 42.0).abs() < 0.1,
            "unexpected horizontal no-title budget: {size}"
        );
    }

    #[test]
    fn inset_vertical_right_reserves_horizontal_strip() {
        let l = Config::right();
        let (edge, size) = reservation(&l, None).expect("inset legend reserves");
        assert_eq!(edge, Edge::Right);
        // bar(10) + tick(4) + label_col(6*6+4=40) + padding(16) = 70; with
        // 10% slack for cjk-ish glyphs in future this is acceptable — the
        // test just guards against wild regressions.
        assert!((60.0..=100.0).contains(&size), "unexpected vertical budget: {size}");
    }

    #[test]
    fn inset_vertical_right_widens_for_long_title() {
        let l = Config::right();
        let (_, narrow) = reservation(&l, None).unwrap();
        let (_, wide) = reservation(&l, Some("Population density per km2")).unwrap();
        assert!(wide > narrow, "long title should widen vertical strip");
    }

    #[test]
    fn inset_anchor_tieskew_resolves_via_edge() {
        // Orientation::Vertical + Anchor::TopRight must reserve on the
        // right edge; this exercises `Config::edge()` tie-breaking.
        let l = Config::overlay(Anchor::TopRight)
            .placement(Placement::Inset)
            .orientation(Orientation::Vertical);
        let (edge, _) = reservation(&l, None).unwrap();
        assert_eq!(edge, Edge::Right);
    }
}

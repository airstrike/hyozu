//! Scale legends: the keys that explain a mark's color or size scale.
//!
//! A mark that carries a scale legend reports it through
//! [`crate::Mark::scale_legend_config`] as a [`legend::Scale`] request:
//! its [`Config`], title and [`Key`]. [`reservation`] turns the request
//! into the edge strip an inset legend needs. During layout the scene
//! turns each such mark into a [`Legend`]: a theme-free plan holding the
//! tick labels, color steps or circle sizes plus where its colors come
//! from. Drawing resolves those colors against the theme and paints the
//! key onto a frame.
//!
//! # Keys
//!
//! - [`Key::Gradient`]: a continuous color bar for choropleths, heatmaps
//!   and scalar diagrams (contour lines, hexbins, tile grids). Ticks sit
//!   at the color scale's transform positions, so a log scale reads in
//!   decades.
//! - [`Key::Steps`]: one uniform swatch per band for banded diagrams such
//!   as filled contours, labeled at the band edges.
//! - [`Key::Sizes`]: circles at round values, drawn at the diameters an
//!   [`crate::Xy::size_by`] encoding gives them.
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

mod ticks;

use crate::chart::plot_area::{choropleth, diagram};
use crate::core::{Color, Point, Rectangle, Size, Vector, alignment};
use crate::data::legend::{self, Anchor, Config, Edge, Key, Orientation, Placement};
use crate::data::mark::Mark;
use crate::scale::Transform;
use crate::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};
use crate::widget::renderer::geometry;
use crate::{encoding, palette};

use ticks::Label;

/// One labeled tick along a key.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Tick {
    /// Position along the key as a unit interval: `0.0` is the low end
    /// (left for horizontal, bottom for vertical), `1.0` the high end.
    pub(crate) t: f32,
    /// Pre-formatted label rendered next to the tick line.
    pub(crate) label: String,
}

/// A mark's scale legend, planned in layout: what the key shows and
/// where its colors come from, with no theme resolved yet.
#[derive(Debug, Clone)]
pub(crate) struct Legend {
    /// The mark's index among the plot area's series, which keys the
    /// strip the scene reserved for it.
    series: usize,
    config: Config,
    title: Option<String>,
    plan: Plan,
}

#[derive(Debug, Clone)]
enum Plan {
    /// A continuous color bar.
    Gradient { ramp: Ramp, ticks: Vec<Tick> },
    /// Uniform color steps, one per bin, with ticks at the bin edges.
    Steps {
        ramp: Ramp,
        bins: Vec<Bin>,
        ticks: Vec<Tick>,
    },
    /// Circles at round values, ascending.
    Sizes { fill: Fill, circles: Vec<Circle> },
}

/// Where a key's colors come from.
#[derive(Debug, Clone)]
enum Ramp {
    /// A continuous palette sampled in OKLch, as heatmaps and choropleths
    /// paint.
    Palette { palette: palette::Palette, reverse: bool },
    /// `count` chart palette slots from `offset`, spread over the unit
    /// interval and blended pairwise, as scalar diagrams paint.
    Slots { offset: usize, count: usize },
}

/// One color step: the unit position its color samples, unless the
/// entry sets its own color.
#[derive(Debug, Clone, Copy)]
struct Bin {
    unit: f64,
    color: Option<crate::color::Color>,
}

#[derive(Debug, Clone)]
struct Circle {
    diameter: f32,
    label: String,
}

/// A size key's circle fill: the marker's color, else the series slot.
#[derive(Debug, Clone, Copy)]
struct Fill {
    color: Option<crate::color::Color>,
    slot: usize,
    opacity: f32,
}

/// A [`Ramp`] resolved against the theme.
enum Colors {
    Stops(Vec<Color>),
    Slots(Vec<Color>),
}

/// Theme colors and font for the panel, title and labels.
struct Chrome {
    border: Color,
    label: Color,
    font: crate::core::Font,
}

/// Pixel margin from the plot-area edge for overlaid scale legends.
const EDGE_PADDING: f32 = 16.0;
/// Inner padding inside the legend background panel.
const PANEL_PADDING: f32 = 8.0;
/// Default font size for legend text.
const FONT_SIZE: f32 = 10.0;
/// Height of the title line, gap included.
const TITLE_LINE_HEIGHT: f32 = FONT_SIZE + 4.0;
/// Height of a tick-label line.
const LABEL_LINE_HEIGHT: f32 = FONT_SIZE + 2.0;
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
/// Extra room above and below vertical gradient bars so endpoint labels
/// do not crowd the title or panel edge when labels are vertically
/// centered on their ticks.
const VERTICAL_BAR_END_PADDING: f32 = 8.0;
/// Minimum plot width (horizontal) or height (vertical) below which an
/// overlaid color bar is suppressed to avoid covering a cramped plot.
const MIN_CHART_EXTENT: f32 = 300.0;
/// Shortest color bar an inset strip draws.
const MIN_BAR_LENGTH: f32 = 60.0;
/// Estimated width per glyph at `FONT_SIZE` when we don't have a paragraph
/// cache on hand. Keeps label-column math O(1).
const GLYPH_WIDTH_ESTIMATE: f32 = 6.0;
/// Glyphs a numeric label is budgeted for: formatted values such as
/// `"999.9M"` or `"1.2e3"` stay within six.
const NUMERIC_LABEL_CHARS: f32 = 6.0;
/// Gap between neighboring circles of a horizontal size key.
const CIRCLE_GAP: f32 = 10.0;
/// Gap between neighboring rows of a vertical size key.
const CIRCLE_ROW_GAP: f32 = 4.0;
/// Gap between a circle and its label.
const CIRCLE_LABEL_GAP: f32 = 4.0;
/// Most circles a size key shows.
const MAX_CIRCLES: usize = 4;
/// Circles narrower than this are left out of a size key.
const SMALLEST_CIRCLE: f32 = 3.0;

/// Pixel reservation a scene layout should carve off the plot area for a
/// scale legend with [`Placement::Inset`].
///
/// Returns `None` when the legend is [`Placement::Overlaid`] (no space
/// reservation) — the caller should fall through to the standard overlay
/// path instead. Horizontal legends reserve vertical space on the top/
/// bottom edge; vertical legends reserve horizontal space on the left/
/// right edge. The numeric budget includes both panel padding and label
/// extents and is independent of the current plot size — the scene needs
/// a value it can subtract before the plot dimensions are known. A size
/// key budgets for its largest possible circle.
pub fn reservation(scale: &legend::Scale<'_>) -> Option<(Edge, f32)> {
    let legend = scale.config();
    if !matches!(legend.placement_value(), Placement::Inset) {
        return None;
    }
    let title = scale.title();
    let title_line_height = title_height(title);
    let budget = match (scale.key(), legend.orientation_value()) {
        (Key::Gradient | Key::Steps, Orientation::Horizontal) => {
            title_line_height + BAR_THICKNESS + TICK_LENGTH + LABEL_LINE_HEIGHT + PANEL_PADDING * 2.0
        }
        // Vertical: bar thickness + tick + label column + padding. Labels
        // are usually short (formatted numbers) but the title can widen
        // the strip when it beats the numeric labels.
        (Key::Gradient | Key::Steps, Orientation::Vertical) => {
            BAR_THICKNESS + TICK_LENGTH + label_column_width(title) + PANEL_PADDING * 2.0
        }
        (Key::Sizes { largest }, Orientation::Horizontal) => {
            title_line_height + largest + CIRCLE_LABEL_GAP + LABEL_LINE_HEIGHT + PANEL_PADDING * 2.0
        }
        (Key::Sizes { largest }, Orientation::Vertical) => {
            let cluster = largest + CIRCLE_LABEL_GAP * 2.0 + NUMERIC_LABEL_CHARS * GLYPH_WIDTH_ESTIMATE;
            cluster.max(text_width(title)) + PANEL_PADDING * 2.0
        }
    };
    Some((legend.edge(), budget))
}

/// Returns `true` when the legend's placement is `Overlaid`. Exposed so
/// the caller can decide whether to short-circuit or defer to scene-level
/// space reservation.
pub fn is_overlaid(legend: &Config) -> bool {
    matches!(legend.placement_value(), Placement::Overlaid)
}

impl Legend {
    /// Plans the scale legend `mark` requests, or `None` when it has none.
    ///
    /// `series` is the mark's index among the plot area's series and
    /// `offset` the first palette slot it paints with, both in the plot
    /// area's order, so diagram ramps and bubble fills match the marks.
    pub(crate) fn of(series: usize, mark: &Mark, offset: usize) -> Option<Self> {
        let scale = mark.scale_legend_config()?;
        let config = scale.config();
        let orientation = config.orientation_value();
        let label =
            |format: Option<&crate::scale::Format<f64>>| Label::new(config.value_format_ref().or(format).cloned());
        let plan = match mark {
            Mark::Choropleth(c) => {
                let (lo, hi) = choropleth::color_domain(c);
                Plan::Gradient {
                    ramp: Ramp::Palette {
                        palette: choropleth::resolved_palette(&c.color),
                        reverse: c.color.reverse,
                    },
                    ticks: ticks::along(lo, hi, c.color.transform, &label(c.color.format.as_ref()), orientation),
                }
            }
            Mark::Heatmap(h) => {
                let (lo, hi) = h.value_range();
                Plan::Gradient {
                    ramp: Ramp::Palette {
                        palette: h.resolved_palette(),
                        reverse: false,
                    },
                    ticks: ticks::along(lo, hi, h.color.transform, &label(h.color.format.as_ref()), orientation),
                }
            }
            Mark::Diagram(d) => {
                let domain = d.color_domain?;
                let ramp = Ramp::Slots {
                    offset,
                    count: d.entries.len(),
                };
                match &d.bins {
                    Some(edges) if edges.len() == d.entries.len() + 1 => Plan::Steps {
                        bins: d
                            .entries
                            .iter()
                            .map(|e| Bin {
                                unit: diagram::scalar_unit(domain, e.value),
                                color: e.color,
                            })
                            .collect(),
                        ticks: ticks::edges(edges, &label(None), orientation),
                        ramp,
                    },
                    _ => Plan::Gradient {
                        ticks: ticks::along(domain.0, domain.1, Transform::Linear, &label(None), orientation),
                        ramp,
                    },
                }
            }
            Mark::Xy(xy) => Plan::Sizes {
                fill: Fill {
                    color: xy.marker().color.or(xy.color),
                    slot: offset,
                    opacity: xy.opacity,
                },
                circles: circles(xy.diameters()?, &label(None)),
            },
            _ => return None,
        };
        Some(Self {
            series,
            config: config.clone(),
            title: scale.title().map(str::to_owned),
            plan,
        })
    }

    /// Fits the key to the room the scene laid out: `plot` is the plot
    /// area's size and `strip` the size of the strip reserved for an inset
    /// legend. A color bar shorter than the plan assumed thins its tick
    /// labels again; a size key that would not fit drops interior circles,
    /// keeping the smallest and largest longest.
    pub(crate) fn fit(&mut self, plot: Size, strip: Option<Size>) {
        let orientation = self.config.orientation_value();
        let title = self.title.as_deref();
        let plot = Rectangle::with_size(plot);
        let strip = strip.map(Rectangle::with_size);
        match &mut self.plan {
            Plan::Gradient { ticks, .. } | Plan::Steps { ticks, .. } => {
                let layout = match strip {
                    Some(strip) => panel_for_strip(&self.config, title, strip),
                    None => panel_for_overlay(&self.config, title, plot),
                };
                let nominal = match orientation {
                    Orientation::Horizontal => ticks::HORIZONTAL_FIT_LENGTH,
                    Orientation::Vertical => ticks::VERTICAL_FIT_LENGTH,
                };
                let length = bar_length(&layout, orientation);
                if length < nominal {
                    *ticks = ticks::fit(std::mem::take(ticks), orientation, length);
                }
            }
            Plan::Sizes { circles, .. } => {
                let room = strip.unwrap_or(plot);
                while circles.len() > 1 {
                    let panel = panel_for_sizes(&self.config, title, circles, plot, strip);
                    if panel.size.width <= room.width && panel.size.height <= room.height {
                        break;
                    }
                    let drop = circles.len().saturating_sub(2);
                    circles.remove(drop);
                }
            }
        }
    }

    /// The mark's index among the plot area's series.
    pub(crate) fn series(&self) -> usize {
        self.series
    }

    /// Draws the legend.
    ///
    /// `plot` is the plot area's absolute bounds. When `strip` is `Some`,
    /// the legend renders inside that absolute rectangle, the strip the
    /// scene reserved for an inset legend; otherwise it overlays the plot
    /// area at its anchor. `palette` is the chart's resolved palette.
    pub(crate) fn draw<Theme, Renderer>(
        &self,
        renderer: &mut Renderer,
        theme: &Theme,
        palette: &palette::Resolved,
        plot: Rectangle,
        strip: Option<Rectangle>,
    ) where
        Theme: crate::design::Design + ?Sized,
        Renderer: geometry::Renderer,
    {
        // The frame covers the strip, or the plot area for an overlay;
        // geometry inside it is local to that box.
        let (frame_size, translation, strip_local) = match strip {
            Some(strip) => (
                strip.size(),
                Vector::new(strip.x, strip.y),
                Some(Rectangle::with_size(strip.size())),
            ),
            None => (plot.size(), Vector::new(plot.x, plot.y), None),
        };
        let mut frame = Frame::new(renderer, frame_size);
        self.paint(
            &mut frame,
            theme,
            palette,
            Rectangle::with_size(plot.size()),
            strip_local,
        );
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(frame.into_geometry());
        });
    }

    fn paint<Theme, Renderer>(
        &self,
        frame: &mut Frame<Renderer>,
        theme: &Theme,
        palette: &palette::Resolved,
        plot_bounds: Rectangle,
        strip_rect: Option<Rectangle>,
    ) where
        Theme: crate::design::Design + ?Sized,
        Renderer: geometry::Renderer,
    {
        let legend = &self.config;
        let title = self.title.as_deref();
        let orientation = legend.orientation_value();

        let layout = match &self.plan {
            Plan::Gradient { .. } | Plan::Steps { .. } => {
                match strip_rect {
                    // The scene already gave an inset legend its strip, so
                    // it draws whenever a readable bar fits there.
                    Some(strip) => {
                        let layout = panel_for_strip(legend, title, strip);
                        if bar_length(&layout, orientation) < MIN_BAR_LENGTH {
                            return;
                        }
                        layout
                    }
                    // An overlay floats over the marks, so it stays out of
                    // a cramped plot area.
                    None => {
                        let extent = match orientation {
                            Orientation::Horizontal => plot_bounds.width,
                            Orientation::Vertical => plot_bounds.height,
                        };
                        if extent < MIN_CHART_EXTENT {
                            return;
                        }
                        panel_for_overlay(legend, title, plot_bounds)
                    }
                }
            }
            Plan::Sizes { circles, .. } => {
                let layout = panel_for_sizes(legend, title, circles, plot_bounds, strip_rect);
                let room = strip_rect.unwrap_or(plot_bounds);
                if circles.is_empty() || layout.size.width > room.width || layout.size.height > room.height {
                    return;
                }
                layout
            }
        };

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();
        let chrome = Chrome {
            border: theme.divider_color().resolve(background, text_pair, &seed, None),
            label: Color {
                a: 0.7,
                ..text_pair.resolve(background, None)
            },
            font: theme.font(),
        };
        let resolve = |c: &crate::color::Color| c.resolve(background, text_pair, &seed, None);

        // 1. Background panel. Inset strips blend into the chart
        // background so they don't look like a floating overlay.
        if strip_rect.is_none() {
            let panel = Path::rectangle(layout.origin, layout.size);
            frame.fill(&panel, Color { a: 0.85, ..background });
            frame.stroke(&panel, Stroke::default().with_color(chrome.border).with_width(0.5));
        }

        let body = Rectangle::new(layout.origin + layout.body_offset, layout.body_size);

        // 2. Optional title, centered over the bar of a horizontal color
        // key and over the panel otherwise.
        if let Some(title) = title {
            let x = match (&self.plan, orientation) {
                (Plan::Gradient { .. } | Plan::Steps { .. }, Orientation::Horizontal) => body.center_x(),
                _ => layout.origin.x + layout.size.width / 2.0,
            };
            text(
                frame,
                &chrome,
                title,
                Point::new(x, layout.origin.y + PANEL_PADDING),
                (alignment::Horizontal::Center, alignment::Vertical::Top),
            );
        }

        // 3. The key itself.
        match &self.plan {
            Plan::Gradient { ramp, ticks } => {
                let colors = ramp.resolve(palette, background, text_pair, &seed);
                draw_gradient(frame, body, orientation, &colors);
                draw_bar_outline_and_ticks(frame, body, orientation, ticks, &chrome);
            }
            Plan::Steps { ramp, bins, ticks } => {
                let colors = ramp.resolve(palette, background, text_pair, &seed);
                let n = bins.len().max(1) as f32;
                for (i, bin) in bins.iter().enumerate() {
                    let color = bin.color.as_ref().map_or_else(|| colors.at(bin.unit as f32), resolve);
                    let (lo, hi) = (i as f32 / n, (i + 1) as f32 / n);
                    let rect = match orientation {
                        Orientation::Horizontal => Rectangle {
                            x: body.x + body.width * lo,
                            y: body.y,
                            width: body.width * (hi - lo),
                            height: body.height,
                        },
                        // Low values at the bottom, as on a vertical bar.
                        Orientation::Vertical => Rectangle {
                            x: body.x,
                            y: body.y + body.height * (1.0 - hi),
                            width: body.width,
                            height: body.height * (hi - lo),
                        },
                    };
                    frame.fill(&Path::rectangle(rect.position(), rect.size()), color);
                }
                draw_bar_outline_and_ticks(frame, body, orientation, ticks, &chrome);
            }
            Plan::Sizes { fill, circles } => {
                let color = fill
                    .color
                    .as_ref()
                    .map_or_else(|| resolve(&palette.get(fill.slot)), resolve);
                let fill_color = Color {
                    a: color.a * fill.opacity.clamp(0.0, 1.0),
                    ..color
                };
                let largest = largest(circles);
                let mut cursor = 0.0;
                // Vertical keys list the largest circle first, so the high
                // end sits on top as on a vertical color bar.
                let ordered: Box<dyn Iterator<Item = &Circle>> = match orientation {
                    Orientation::Horizontal => Box::new(circles.iter()),
                    Orientation::Vertical => Box::new(circles.iter().rev()),
                };
                for circle in ordered {
                    let (center, label_at, align) = match orientation {
                        Orientation::Horizontal => {
                            let column = circle_column_width(circle);
                            let cx = body.x + cursor + column / 2.0;
                            cursor += column + CIRCLE_GAP;
                            (
                                Point::new(cx, body.y + largest - circle.diameter / 2.0),
                                Point::new(cx, body.y + largest + CIRCLE_LABEL_GAP),
                                (alignment::Horizontal::Center, alignment::Vertical::Top),
                            )
                        }
                        Orientation::Vertical => {
                            let row = circle.diameter.max(LABEL_LINE_HEIGHT);
                            let cy = body.y + cursor + row / 2.0;
                            cursor += row + CIRCLE_ROW_GAP;
                            (
                                Point::new(body.x + largest / 2.0, cy),
                                Point::new(body.x + largest + CIRCLE_LABEL_GAP * 2.0, cy),
                                (alignment::Horizontal::Left, alignment::Vertical::Center),
                            )
                        }
                    };
                    let disc = Path::circle(center, circle.diameter / 2.0);
                    frame.fill(&disc, fill_color);
                    frame.stroke(&disc, Stroke::default().with_color(color).with_width(1.0));
                    text(frame, &chrome, &circle.label, label_at, align);
                }
            }
        }
    }
}

impl Ramp {
    fn resolve(
        &self,
        resolved: &palette::Resolved,
        background: Color,
        text: crate::color::Pair,
        seed: &palette::Seed,
    ) -> Colors {
        match self {
            Self::Palette { palette, reverse } => {
                let mut stops = choropleth::palette_to_continuous_stops(palette, seed);
                if *reverse {
                    stops.reverse();
                }
                Colors::Stops(stops)
            }
            Self::Slots { offset, count } => Colors::Slots(
                (0..*count)
                    .map(|j| resolved.get(offset + j).resolve(background, text, seed, None))
                    .collect(),
            ),
        }
    }
}

impl Colors {
    /// The color at unit position `t`, sampled as the marks sample it.
    fn at(&self, t: f32) -> Color {
        match self {
            Self::Stops(stops) => palette::sample_gradient(stops, t),
            Self::Slots(slots) => diagram::scalar_color(slots, f64::from(t)),
        }
    }
}

/// Up to [`MAX_CIRCLES`] circles at round values of the size domain, each
/// at the diameter the encoding maps it to, ascending. Values whose circle
/// would be too small to see are left out; a collapsed domain shows its
/// single value.
fn circles(diameters: encoding::Diameters, label: &Label) -> Vec<Circle> {
    let (lo, hi) = diameters.domain();
    let transform = match diameters.scale() {
        encoding::channel::SizeScale::Log => Transform::Log,
        encoding::channel::SizeScale::Linear | encoding::channel::SizeScale::Sqrt => Transform::Linear,
    };
    let mut values: Vec<f64> = ticks::round_values(lo, hi, transform, MAX_CIRCLES)
        .into_iter()
        .filter(|v| diameters.at(*v).is_some_and(|d| d >= SMALLEST_CIRCLE))
        .collect();
    if values.len() > MAX_CIRCLES {
        // Thin from the top so the largest round value always shows.
        let stride = values.len().div_ceil(MAX_CIRCLES);
        values = values.into_iter().rev().step_by(stride).collect();
        values.reverse();
    }
    if values.is_empty() && hi.is_finite() {
        values.push(hi);
    }
    let labels = label.all(&values);
    values
        .into_iter()
        .zip(labels)
        .filter_map(|(v, label)| {
            Some(Circle {
                diameter: diameters.at(v)?,
                label,
            })
        })
        .collect()
}

/// Length of a color key's bar along its flow axis.
fn bar_length(layout: &PanelLayout, orientation: Orientation) -> f32 {
    match orientation {
        Orientation::Horizontal => layout.body_size.width,
        Orientation::Vertical => layout.body_size.height,
    }
}

fn largest(circles: &[Circle]) -> f32 {
    circles.iter().fold(0.0, |m, c| m.max(c.diameter))
}

/// Width a horizontal size key gives one circle: the wider of the circle
/// and its label.
fn circle_column_width(circle: &Circle) -> f32 {
    circle.diameter.max(text_width(Some(&circle.label)))
}

/// Resolved panel geometry for a draw call.
struct PanelLayout {
    origin: Point,
    size: Size,
    /// Offset from panel origin to the key: the bar of a color key, or the
    /// circles and their labels for a size key.
    body_offset: Vector,
    body_size: Size,
}

/// Size a draw panel based on the plot-area span on the flow axis.
///
/// Used for [`Placement::Overlaid`] where the panel floats inside the plot
/// area: the bar length scales down gracefully on narrow charts.
fn panel_for_overlay(legend: &Config, title: Option<&str>, plot_bounds: Rectangle) -> PanelLayout {
    let orientation = legend.orientation_value();
    let title_line_height = title_height(title);

    let (panel_size, body_offset, body_size) = match orientation {
        Orientation::Horizontal => {
            let bar_length = if plot_bounds.width < 400.0 {
                (plot_bounds.width * 0.45).min(HORIZONTAL_BAR_LENGTH)
            } else {
                HORIZONTAL_BAR_LENGTH
            };
            let content_w = bar_length;
            let content_h = title_line_height + BAR_THICKNESS + TICK_LENGTH + LABEL_LINE_HEIGHT;
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
            let cluster_w = vertical_cluster_width();
            let title_w = label_column_width(title);
            let content_w = cluster_w.max(title_w);
            let content_h = title_line_height + VERTICAL_BAR_END_PADDING * 2.0 + bar_length;
            let panel_w = content_w + PANEL_PADDING * 2.0;
            let panel_h = content_h + PANEL_PADDING * 2.0;
            let bar_offset_y = PANEL_PADDING + title_line_height + VERTICAL_BAR_END_PADDING;
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
        body_offset,
        body_size,
    }
}

/// Size a panel that fills a reserved strip rectangle.
///
/// Used for [`Placement::Inset`]: the strip has a fixed thickness decided
/// by [`reservation`], and the bar stretches along the flow axis up to the
/// configured maximum.
fn panel_for_strip(legend: &Config, title: Option<&str>, strip: Rectangle) -> PanelLayout {
    let orientation = legend.orientation_value();
    let title_line_height = title_height(title);

    let (panel_size, body_offset, body_size) = match orientation {
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
            let strip_bar_budget =
                (strip.height - PANEL_PADDING * 2.0 - title_line_height - VERTICAL_BAR_END_PADDING * 2.0).max(0.0);
            let bar_length = strip_bar_budget.min(VERTICAL_BAR_LENGTH);
            let panel_w = strip.width;
            let panel_h = title_line_height + VERTICAL_BAR_END_PADDING * 2.0 + bar_length + PANEL_PADDING * 2.0;
            let bar_offset_y = PANEL_PADDING + title_line_height + VERTICAL_BAR_END_PADDING;
            // Center the bar + numeric-label cluster horizontally inside
            // the panel. The cluster is sized for short numeric labels
            // rather than `label_column_width`, which sizes for the title
            // and would shrink the centering offset to almost zero.
            let bar_offset_x = ((panel_w - vertical_cluster_width()) * 0.5).max(PANEL_PADDING);
            (
                Size::new(panel_w, panel_h),
                Vector::new(bar_offset_x, bar_offset_y),
                Size::new(BAR_THICKNESS, bar_length),
            )
        }
    };

    PanelLayout {
        origin: center_in_strip(orientation, strip, panel_size),
        size: panel_size,
        body_offset,
        body_size,
    }
}

/// Size a size-key panel around its circles and labels, then place it in
/// the strip or at the overlay anchor.
fn panel_for_sizes(
    legend: &Config,
    title: Option<&str>,
    circles: &[Circle],
    plot_bounds: Rectangle,
    strip: Option<Rectangle>,
) -> PanelLayout {
    let orientation = legend.orientation_value();
    let largest = largest(circles);
    let gaps = circles.len().saturating_sub(1) as f32;
    let body_size = match orientation {
        Orientation::Horizontal => Size::new(
            circles.iter().map(circle_column_width).sum::<f32>() + CIRCLE_GAP * gaps,
            largest + CIRCLE_LABEL_GAP + LABEL_LINE_HEIGHT,
        ),
        Orientation::Vertical => Size::new(
            largest + CIRCLE_LABEL_GAP * 2.0 + circles.iter().map(|c| text_width(Some(&c.label))).fold(0.0, f32::max),
            circles.iter().map(|c| c.diameter.max(LABEL_LINE_HEIGHT)).sum::<f32>() + CIRCLE_ROW_GAP * gaps,
        ),
    };
    let content_w = body_size.width.max(text_width(title));
    let size = Size::new(
        content_w + PANEL_PADDING * 2.0,
        title_height(title) + body_size.height + PANEL_PADDING * 2.0,
    );
    let body_offset = Vector::new(
        PANEL_PADDING + (content_w - body_size.width) / 2.0,
        PANEL_PADDING + title_height(title),
    );
    let origin = match strip {
        Some(strip) => {
            // Center across the strip's thickness too: the strip budgets
            // for the largest possible circle, which round values rarely
            // reach.
            let along = center_in_strip(orientation, strip, size);
            match orientation {
                Orientation::Horizontal => Point::new(along.x, strip.y + ((strip.height - size.height) / 2.0).max(0.0)),
                Orientation::Vertical => Point::new(strip.x + ((strip.width - size.width) / 2.0).max(0.0), along.y),
            }
        }
        None => anchor_origin(legend.anchor_value(), plot_bounds, size),
    };
    PanelLayout {
        origin,
        size,
        body_offset,
        body_size,
    }
}

/// Center the panel within the strip on the flow axis so the key sits
/// roughly beside the middle of the plot area.
fn center_in_strip(orientation: Orientation, strip: Rectangle, panel: Size) -> Point {
    match orientation {
        Orientation::Horizontal => Point::new(strip.x + (strip.width - panel.width) / 2.0, strip.y),
        Orientation::Vertical => Point::new(strip.x, strip.y + (strip.height - panel.height) / 2.0),
    }
}

/// Height of the title line, or zero without a title.
fn title_height(title: Option<&str>) -> f32 {
    if title.is_some() { TITLE_LINE_HEIGHT } else { 0.0 }
}

/// Estimated rendered width of `text` at the legend font size.
fn text_width(text: Option<&str>) -> f32 {
    text.map_or(0.0, |t| t.chars().count() as f32 * GLYPH_WIDTH_ESTIMATE)
}

/// Width of a vertical bar plus its tick marks and numeric labels.
fn vertical_cluster_width() -> f32 {
    BAR_THICKNESS + TICK_LENGTH + 2.0 + NUMERIC_LABEL_CHARS * GLYPH_WIDTH_ESTIMATE
}

/// Estimates the label column width for a vertical legend.
///
/// The numeric labels are short (6 glyphs is a safe upper bound for
/// formatted values like `"999.9M"` or `"1.2e3"`); we widen the column
/// when the legend title beats that.
fn label_column_width(title: Option<&str>) -> f32 {
    let title_chars = title.map(|t| t.chars().count()).unwrap_or(0);
    let chars = (title_chars as f32).max(NUMERIC_LABEL_CHARS);
    chars * GLYPH_WIDTH_ESTIMATE + 4.0
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

fn text<Renderer: geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    chrome: &Chrome,
    content: &str,
    position: Point,
    (align_x, align_y): (alignment::Horizontal, alignment::Vertical),
) {
    frame.fill_text(CanvasText {
        content: content.to_owned(),
        position,
        color: chrome.label,
        size: crate::core::Pixels(FONT_SIZE),
        font: chrome.font,
        align_x: align_x.into(),
        align_y,
        line_height: crate::core::text::LineHeight::default(),
        shaping: crate::core::text::Shaping::Basic,
        ..CanvasText::default()
    });
}

fn draw_gradient<Renderer: geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    bar: Rectangle,
    orientation: Orientation,
    colors: &Colors,
) {
    for i in 0..GRADIENT_SEGMENTS {
        let unit = i as f32 / (GRADIENT_SEGMENTS - 1) as f32;
        // 0.5px overlap hides hairlines between segments.
        let rect = match orientation {
            Orientation::Horizontal => {
                let width = bar.width / GRADIENT_SEGMENTS as f32;
                Rectangle {
                    x: bar.x + i as f32 * width,
                    y: bar.y,
                    width: width + 0.5,
                    height: bar.height,
                }
            }
            Orientation::Vertical => {
                let height = bar.height / GRADIENT_SEGMENTS as f32;
                Rectangle {
                    x: bar.x,
                    y: bar.y + i as f32 * height,
                    width: bar.width,
                    height: height + 0.5,
                }
            }
        };
        // Vertical scale convention: min at the bottom, max at the top.
        let t = match orientation {
            Orientation::Horizontal => unit,
            Orientation::Vertical => 1.0 - unit,
        };
        frame.fill(&Path::rectangle(rect.position(), rect.size()), colors.at(t));
    }
}

fn draw_bar_outline_and_ticks<Renderer: geometry::Renderer>(
    frame: &mut Frame<Renderer>,
    bar: Rectangle,
    orientation: Orientation,
    ticks: &[Tick],
    chrome: &Chrome,
) {
    let hairline = Stroke::default().with_color(chrome.border).with_width(0.5);
    frame.stroke(&Path::rectangle(bar.position(), bar.size()), hairline);

    match orientation {
        Orientation::Horizontal => {
            let tick_top = bar.y + bar.height;
            let tick_bottom = tick_top + TICK_LENGTH;
            for (i, tick) in ticks.iter().enumerate() {
                let x = bar.x + bar.width * tick.t;
                frame.stroke(
                    &Path::line(Point::new(x, tick_top), Point::new(x, tick_bottom)),
                    hairline,
                );
                // Endpoints align to their corner (the bar's outer edge);
                // interior ticks center their label on the tick line.
                let align = if i == 0 {
                    alignment::Horizontal::Left
                } else if i == ticks.len() - 1 {
                    alignment::Horizontal::Right
                } else {
                    alignment::Horizontal::Center
                };
                text(
                    frame,
                    chrome,
                    &tick.label,
                    Point::new(x, tick_bottom + 1.0),
                    (align, alignment::Vertical::Top),
                );
            }
        }
        Orientation::Vertical => {
            // Labels go to the right of the bar; tick marks on the bar edge.
            let tick_left = bar.x + bar.width;
            let tick_right = tick_left + TICK_LENGTH;
            for tick in ticks {
                // Vertical legends paint `t = 1.0` (high end) at the top
                // and `t = 0.0` at the bottom.
                let y = bar.y + bar.height * (1.0 - tick.t);
                frame.stroke(
                    &Path::line(Point::new(tick_left, y), Point::new(tick_right, y)),
                    hairline,
                );
                // Center every label on its tick. Endpoint-specific
                // top/bottom anchoring makes vertical legends look uneven
                // because glyph ascenders and descenders change the
                // perceived spacing between adjacent values.
                text(
                    frame,
                    chrome,
                    &tick.label,
                    Point::new(tick_right + 2.0, y),
                    (alignment::Horizontal::Left, alignment::Vertical::Center),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::legend::{Anchor, Config, Orientation, Placement};

    fn gradient(config: &Config, title: Option<&str>) -> Option<(Edge, f32)> {
        reservation(&legend::Scale::new(config, title, Key::Gradient))
    }

    fn plan(mark: impl Into<Mark>) -> Plan {
        Legend::of(0, &mark.into(), 0)
            .expect("mark carries a scale legend")
            .plan
    }

    #[test]
    fn overlaid_legend_reports_no_reservation() {
        let l = Config::overlay(Anchor::BottomRight);
        assert_eq!(l.placement_value(), Placement::Overlaid);
        assert!(gradient(&l, None).is_none());
    }

    #[test]
    fn inset_horizontal_bottom_reserves_vertical_strip_with_title() {
        let l = Config::below();
        let (edge, size) = gradient(&l, Some("GDP")).expect("inset legend reserves");
        assert_eq!(edge, Edge::Bottom);
        // title(14) + bar(10) + tick(4) + label(12) + padding(16) = 56
        assert!((size - 56.0).abs() < 0.1, "unexpected horizontal+title budget: {size}");
    }

    #[test]
    fn inset_horizontal_bottom_reserves_vertical_strip_without_title() {
        let l = Config::below();
        let (_, size) = gradient(&l, None).unwrap();
        // bar(10) + tick(4) + label(12) + padding(16) = 42
        assert!(
            (size - 42.0).abs() < 0.1,
            "unexpected horizontal no-title budget: {size}"
        );
    }

    #[test]
    fn inset_vertical_right_reserves_horizontal_strip() {
        let l = Config::right();
        let (edge, size) = gradient(&l, None).expect("inset legend reserves");
        assert_eq!(edge, Edge::Right);
        // bar(10) + tick(4) + label_col(6*6+4=40) + padding(16) = 70; with
        // 10% slack for cjk-ish glyphs in future this is acceptable — the
        // test just guards against wild regressions.
        assert!((60.0..=100.0).contains(&size), "unexpected vertical budget: {size}");
    }

    #[test]
    fn inset_vertical_right_widens_for_long_title() {
        let l = Config::right();
        let (_, narrow) = gradient(&l, None).unwrap();
        let (_, wide) = gradient(&l, Some("Population density per km2")).unwrap();
        assert!(wide > narrow, "long title should widen vertical strip");
    }

    #[test]
    fn size_key_reserves_room_for_its_largest_circle() {
        let below = Config::below();
        let right = Config::right();
        let budget = |config: &Config, largest| {
            reservation(&legend::Scale::new(config, None, Key::Sizes { largest }))
                .expect("inset size key reserves")
                .1
        };
        // circle(40) + gap(4) + label(12) + padding(16)
        assert!((budget(&below, 40.0) - 72.0).abs() < 0.1);
        assert!(budget(&right, 40.0) > 40.0 + PANEL_PADDING * 2.0);
        assert!(budget(&below, 60.0) > budget(&below, 40.0));
    }

    #[test]
    fn vertical_overlay_panel_pads_bar_end_labels() {
        let legend = Config::overlay(Anchor::BottomRight).orientation(Orientation::Vertical);
        let panel = panel_for_overlay(&legend, Some("Revenue"), Rectangle {
            x: 0.0,
            y: 0.0,
            width: 600.0,
            height: 400.0,
        });

        assert!(
            panel.body_offset.y >= PANEL_PADDING + TITLE_LINE_HEIGHT + VERTICAL_BAR_END_PADDING,
            "vertical legend should keep the top tick label away from the title"
        );
        assert!(
            panel.size.height - (panel.body_offset.y + panel.body_size.height)
                >= PANEL_PADDING + VERTICAL_BAR_END_PADDING,
            "vertical legend should keep the bottom tick label away from the panel edge"
        );
    }

    #[test]
    fn inset_anchor_tieskew_resolves_via_edge() {
        // Orientation::Vertical + Anchor::TopRight must reserve on the
        // right edge; this exercises `Config::edge()` tie-breaking.
        let l = Config::overlay(Anchor::TopRight)
            .placement(Placement::Inset)
            .orientation(Orientation::Vertical);
        let (edge, _) = gradient(&l, None).unwrap();
        assert_eq!(edge, Edge::Right);
    }

    #[test]
    fn heatmap_color_bar_spans_its_color_domain() {
        let heatmap = crate::heatmap(vec![3.0, 47.0, 12.0, 30.0], 2, 2)
            .color_domain(0.0, 50.0)
            .scale_legend(Config::right());
        let Plan::Gradient { ticks, .. } = plan(heatmap) else {
            panic!("heatmaps draw a color bar");
        };
        let labels: Vec<_> = ticks.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels, ["0", "10", "20", "30", "40", "50"]);
        assert!(ticks.windows(2).all(|w| w[0].t < w[1].t));
    }

    #[test]
    fn log_heatmap_color_bar_ticks_follow_the_transform() {
        let heatmap = crate::heatmap(vec![1.0, 10.0, 100.0, 1000.0], 2, 2)
            .log()
            .scale_legend(Config::right());
        let Plan::Gradient { ticks, .. } = plan(heatmap) else {
            panic!("heatmaps draw a color bar");
        };
        let at: Vec<_> = ticks.iter().map(|t| (t.label.as_str(), t.t)).collect();
        assert_eq!(at.len(), 4, "{at:?}");
        for ((_, t), expected) in at.iter().zip([0.0, 1.0 / 3.0, 2.0 / 3.0, 1.0]) {
            assert!((t - expected).abs() < 1e-5, "{at:?}");
        }
    }

    #[test]
    fn heatmap_without_a_legend_plans_nothing() {
        let mark: Mark = crate::heatmap(vec![1.0, 2.0], 1, 2).into();
        assert!(Legend::of(0, &mark, 0).is_none());
    }

    #[test]
    fn contour_band_steps_match_the_levels() {
        let axis: Vec<f64> = (0..=4).map(f64::from).collect();
        let heights: Vec<f64> = axis.iter().flat_map(|&y| axis.iter().map(move |&x| x + y)).collect();
        let levels = [0.0, 1.5, 3.0, 4.5, 6.0, 8.0];
        let bands = crate::field::Mesh::grid(&axis, &axis, &heights)
            .expect("grid")
            .contour_bands(&levels)
            .expect("increasing levels")
            .scale_legend(Config::right());
        let mark: Mark = bands.into();
        assert_eq!(
            mark.scale_legend_config().map(|s| s.key()),
            Some(Key::Steps),
            "banded fields request color steps"
        );
        let Plan::Steps { bins, ticks, .. } = plan(mark.clone()) else {
            panic!("banded fields draw color steps");
        };
        assert_eq!(bins.len(), levels.len() - 1, "one step per band");
        let labels: Vec<_> = ticks.iter().map(|t| t.label.as_str()).collect();
        assert_eq!(labels, ["0.0", "1.5", "3.0", "4.5", "6.0", "8.0"]);
        for (i, tick) in ticks.iter().enumerate() {
            assert!((tick.t - i as f32 / 5.0).abs() < 1e-6, "edges space evenly");
        }
        // Each step's color samples where the diagram paints that band.
        let Mark::Diagram(d) = &mark else { unreachable!() };
        let domain = d.color_domain.expect("scalar");
        for (bin, entry) in bins.iter().zip(&d.entries) {
            assert_eq!(bin.unit, diagram::scalar_unit(domain, entry.value));
        }
        assert!(bins.windows(2).all(|w| w[0].unit < w[1].unit));
    }

    #[test]
    fn contour_lines_draw_a_color_bar_and_leave_the_series_legend() {
        let axis: Vec<f64> = (0..=4).map(f64::from).collect();
        let heights: Vec<f64> = axis.iter().flat_map(|&y| axis.iter().map(move |&x| x * y)).collect();
        let lines = crate::field::Mesh::grid(&axis, &axis, &heights)
            .expect("grid")
            .contours(&[2.0, 4.0, 8.0])
            .scale_legend(Config::below());
        let mark: Mark = lines.into();
        assert!(matches!(plan(mark.clone()), Plan::Gradient { .. }));
        assert!(mark.legend_entries().is_empty());
    }

    #[test]
    fn categorical_diagrams_ignore_a_scale_legend() {
        let funnel: Mark = crate::funnel(&[("Visits", 100.0), ("Signups", 40.0)])
            .scale_legend(Config::right())
            .into();
        assert!(funnel.scale_legend_config().is_none());
        assert!(!funnel.legend_entries().is_empty());
    }

    #[test]
    fn size_key_circles_match_the_bubble_diameters() {
        let values = [8.0, 22.0, 14.0, 34.0, 18.0, 42.0];
        let encoding = crate::encoding::size_by(move |i, _| values[i])
            .domain(0.0..=42.0)
            .range(0.0..=43.0);
        let bubbles = crate::xy([(1., 3.), (2., 5.), (3., 2.), (4., 6.), (5., 4.), (6., 7.)])
            .size_by(encoding.clone())
            .size_legend(Config::right());
        let mapping = encoding.diameters(&[]).expect("explicit domain");
        let Plan::Sizes { circles, .. } = plan(bubbles) else {
            panic!("size encodings draw a size key");
        };
        let labels: Vec<_> = circles.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["10", "20", "30", "40"]);
        for (circle, v) in circles.iter().zip([10.0, 20.0, 30.0, 40.0]) {
            assert_eq!(Some(circle.diameter), mapping.at(v));
        }
        // Sqrt scale: a circle with twice the value has twice the area.
        let area = |d: f32| d * d;
        assert!((area(circles[1].diameter) / area(circles[0].diameter) - 2.0).abs() < 1e-3);
    }

    #[test]
    fn short_strips_keep_the_extreme_circles_and_fewer_labels() {
        let values = [8.0, 22.0, 14.0, 34.0, 18.0, 42.0];
        let bubbles: Mark = crate::xy([(1., 3.), (2., 5.), (3., 2.), (4., 6.), (5., 4.), (6., 7.)])
            .size_by(
                crate::encoding::size_by(move |i, _| values[i])
                    .domain(0.0..=42.0)
                    .range(0.0..=43.0),
            )
            .size_legend(Config::right())
            .size_legend_title("Orders")
            .into();
        let mut legend = Legend::of(0, &bubbles, 0).expect("size key");
        let (_, budget) = reservation(&bubbles.scale_legend_config().expect("request")).expect("inset");
        legend.fit(Size::new(400.0, 120.0), Some(Size::new(budget, 120.0)));
        let Plan::Sizes { circles, .. } = &legend.plan else {
            panic!("size key");
        };
        let labels: Vec<_> = circles.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["10", "40"], "the smallest and largest circles stay");

        let heatmap: Mark = crate::heatmap(vec![0.0, 100.0], 1, 2)
            .scale_legend(Config::right())
            .into();
        let mut legend = Legend::of(0, &heatmap, 0).expect("color bar");
        let Plan::Gradient { ticks: roomy, .. } = legend.plan.clone() else {
            panic!("color bar");
        };
        legend.fit(Size::new(400.0, 90.0), Some(Size::new(70.0, 90.0)));
        let Plan::Gradient { ticks, .. } = &legend.plan else {
            panic!("color bar");
        };
        assert!(ticks.len() < roomy.len(), "a short bar thins its labels");
        assert_eq!(ticks.first().map(|t| t.t), Some(0.0));
        assert_eq!(ticks.last().map(|t| t.t), Some(1.0));
    }

    #[test]
    fn size_key_needs_a_size_encoding() {
        let plain: Mark = crate::xy([(1.0, 2.0)]).size_legend(Config::right()).into();
        assert!(plain.scale_legend_config().is_none());
    }

    #[test]
    fn size_key_thins_to_the_largest_round_values() {
        let bubbles = crate::xy([(0.0, 0.0), (1.0, 100.0)])
            .size_by(crate::encoding::size_by(|_, d| d.y).range(2.0..=30.0).linear())
            .size_legend(Config::below());
        let Plan::Sizes { circles, .. } = plan(bubbles) else {
            panic!("size key");
        };
        assert!(circles.len() <= MAX_CIRCLES);
        assert_eq!(circles.last().map(|c| c.label.as_str()), Some("100"));
        assert!(circles.windows(2).all(|w| w[0].diameter < w[1].diameter));
    }

    #[test]
    fn size_panel_fits_its_strip_without_spilling() {
        let circles: Vec<Circle> = [6.0, 18.0, 30.0]
            .into_iter()
            .map(|diameter| Circle {
                diameter,
                label: format!("{diameter}"),
            })
            .collect();
        for config in [Config::right(), Config::below()] {
            let (_, budget) = reservation(&legend::Scale::new(&config, Some("Orders"), Key::Sizes {
                largest: 30.0,
            }))
            .expect("inset");
            let strip = match config.orientation_value() {
                Orientation::Vertical => Rectangle::new(Point::new(500.0, 0.0), Size::new(budget, 300.0)),
                Orientation::Horizontal => Rectangle::new(Point::new(0.0, 300.0), Size::new(500.0, budget)),
            };
            let panel = panel_for_sizes(
                &config,
                Some("Orders"),
                &circles,
                Rectangle::with_size(Size::new(500.0, 300.0)),
                Some(strip),
            );
            let rect = Rectangle::new(panel.origin, panel.size);
            assert!(
                rect.x >= strip.x - 0.01
                    && rect.y >= strip.y - 0.01
                    && rect.x + rect.width <= strip.x + strip.width + 0.01
                    && rect.y + rect.height <= strip.y + strip.height + 0.01,
                "{rect:?} spills out of {strip:?}"
            );
        }
    }
}

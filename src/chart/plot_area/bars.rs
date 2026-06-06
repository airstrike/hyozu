use super::{Domain, Insets, to_pixel};
use crate::animation;
use crate::core::layout::{Limits, Node};
use crate::core::text::{self, paragraph};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size};
use crate::data::Datum;
use crate::mark::bar::label::Position;
use crate::widget::canvas::{Frame, Path};
use crate::widget::renderer::geometry;

/// Builds a rectangle path with selectively rounded ends.
///
/// A bar has two ends along its growth axis: the *value end* (top for
/// columns, right for bars) and the *baseline end* (bottom / left). The
/// value end uses the per-corner radii from `corners` directly, honoring a
/// Recharts-style `radius=[…]`; the baseline end mirrors those radii so a
/// bar reads as a symmetric pill. Each end is rounded only when its flag is
/// set: by default just the value end rounds (so flush bars sit on the axis
/// and stacked segments meet without gaps), while a bar resting on a track
/// rail — and the rail itself — round both. Radii are clamped so neither
/// can exceed `min(w, h) / 2`.
fn push_bar_path(
    builder: &mut crate::widget::canvas::path::Builder,
    rect: &Rectangle,
    corners: crate::core::border::Radius,
    is_horizontal: bool,
    round_value_end: bool,
    round_baseline: bool,
) {
    let w = rect.width;
    let h = rect.height;
    // NaN is the gap convention: a non-finite coordinate or dimension means
    // "missing value, draw nothing." Without this guard, NaN slips past the
    // `<=` comparison (NaN compares false) and into the lyon tessellator.
    if !rect.x.is_finite() || !rect.y.is_finite() || !w.is_finite() || !h.is_finite() {
        return;
    }
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let cap = (w.min(h)) * 0.5;

    // Resolve the four corner radii from the value-end pair, gated per end.
    // Vertical: value end = top (TL, TR), baseline = bottom (BL, BR).
    // Horizontal: value end = right (TR, BR), baseline = left (TL, BL).
    let (mut r_tl, mut r_tr, mut r_br, mut r_bl) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
    if is_horizontal {
        if round_value_end {
            r_tr = corners.top_right;
            r_br = corners.bottom_right;
        }
        if round_baseline {
            r_tl = corners.top_right;
            r_bl = corners.bottom_right;
        }
    } else {
        if round_value_end {
            r_tl = corners.top_left;
            r_tr = corners.top_right;
        }
        if round_baseline {
            r_bl = corners.top_left;
            r_br = corners.top_right;
        }
    }
    let r_tl = r_tl.clamp(0.0, cap);
    let r_tr = r_tr.clamp(0.0, cap);
    let r_br = r_br.clamp(0.0, cap);
    let r_bl = r_bl.clamp(0.0, cap);

    if r_tl <= 0.0 && r_tr <= 0.0 && r_br <= 0.0 && r_bl <= 0.0 {
        builder.rectangle(Point::new(rect.x, rect.y), Size::new(w, h));
        return;
    }

    // General rounded rectangle, clockwise from just past TL on the top
    // edge. `arc_to(corner, edge_point, radius)` matches the tangent form
    // used elsewhere; a zero-radius corner skips its arc, so the preceding
    // `line_to` lands square on the corner.
    let x = rect.x;
    let y = rect.y;
    let tl = Point::new(x, y);
    let tr = Point::new(x + w, y);
    let br = Point::new(x + w, y + h);
    let bl = Point::new(x, y + h);

    builder.move_to(Point::new(x + r_tl, y));
    builder.line_to(Point::new(tr.x - r_tr, tr.y));
    if r_tr > 0.0 {
        builder.arc_to(tr, Point::new(tr.x, tr.y + r_tr), r_tr);
    }
    builder.line_to(Point::new(br.x, br.y - r_br));
    if r_br > 0.0 {
        builder.arc_to(br, Point::new(br.x - r_br, br.y), r_br);
    }
    builder.line_to(Point::new(bl.x + r_bl, bl.y));
    if r_bl > 0.0 {
        builder.arc_to(bl, Point::new(bl.x, bl.y - r_bl), r_bl);
    }
    builder.line_to(Point::new(tl.x, tl.y + r_tl));
    if r_tl > 0.0 {
        builder.arc_to(tl, Point::new(tl.x + r_tl, tl.y), r_tl);
    }
    builder.close();
}
/// State for Bars - stores positioned bar rectangles
pub struct State<P>
where
    P: text::Paragraph,
{
    /// Pixel rectangles for each series, outer vec is series, inner vec is bars
    pub series_rects: Vec<Vec<Rectangle>>,
    /// Data-label paragraphs, shaped once in `layout` and rendered in `draw`
    /// via [`crate::core::text::Renderer::fill_paragraph`]. Keyed the same as
    /// `series_rects` (outer: series, inner: bar index) so the draw pass can
    /// index a bar's label directly. Shaping the text in `layout` keeps `draw`
    /// O(1) per label and gives the inset measure a real `min_bounds()` to
    /// query instead of a character-width estimate.
    pub labels: Vec<Vec<paragraph::Plain<P>>>,
    /// Pixel rectangles for each label, outer vec is series, inner vec is bars
    pub label_rects: Vec<Vec<Option<Rectangle>>>,
    /// Theme-independent fill encoding plans, one per series. `None` for
    /// series with no `color_by` encoding. Computed in `layout` so the
    /// encoding's expensive walk (extractor calls, distinct-key indexing)
    /// only runs when the data changes, not on every repaint.
    pub series_fill_plans: Vec<Option<crate::encoding::FillPlan>>,
    /// Bar rectangles from the most recent layout before the current one,
    /// captured by [`crate::chart::Chart::diff`] when data changes so the
    /// next sweep can interpolate from previous bounds to current bounds.
    /// Empty on a fresh mount, in which case the animation collapses to a
    /// baseline-anchored grow-in.
    pub previous_series_rects: Vec<Vec<Rectangle>>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
    /// The plot area's pixel rectangle (layout-relative, matching
    /// `series_rects`) captured at layout time. The track rail spans this
    /// extent along the value axis behind each bar.
    pub plot_bounds: Rectangle,
}

/// A Bars series that renders vertical bar charts.
///
/// Like Guide and Title, this is widget-like but doesn't implement Widget.
pub struct Bars<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub(crate) data: &'a crate::bar::Bars,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Bars<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer<Font = crate::core::Font> + geometry::Renderer,
{
    /// Create a new Bars borrowing data
    pub fn new(data: &'a crate::bar::Bars) -> Self {
        Self {
            data,
            animate: true,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets whether the mount/data-change sweep should run. Wired by
    /// [`super::PlotArea::with_animate`] from [`crate::Data::animate`].
    pub(crate) fn set_animate(&mut self, animate: bool) {
        self.animate = animate;
    }

    /// Returns the initial tree state for this Bars
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                series_rects: Vec::new(),
                labels: Vec::new(),
                label_rects: Vec::new(),
                series_fill_plans: Vec::new(),
                previous_series_rects: Vec::new(),
                tick: animation::Tick::new(),
                plot_bounds: Rectangle {
                    x: 0.0,
                    y: 0.0,
                    width: 0.0,
                    height: 0.0,
                },
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Bars state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Returns the pixel inset required on each edge of the data-mapping region
    /// so that `Position::Above` data labels don't clip against the plot area
    /// boundary. Only labels that extend past the bar end (outside the bar)
    /// contribute; inside-placement labels return zero insets.
    ///
    /// For each bar, solves `R >= extent - (extent - pad - label_extent) / v`
    /// where `v ∈ [0, 1]` is the bar's normalized position along the value
    /// axis. Returns the max across all bars.
    ///
    /// `label_extent` is the value-axis size of the *already-shaped* label
    /// paragraph (`state.labels[series][bar].min_bounds()`): width for
    /// horizontal bars (label at the bar's right end), height for vertical
    /// (label above the bar top). The caller shapes the labels first via
    /// [`super::PlotArea::shape_labels`]; this never reshapes.
    pub(super) fn min_insets(
        &self,
        state: &State<Renderer::Paragraph>,
        plot_size: Size,
        x_bounds: (f64, f64),
        y_bounds: (f64, f64),
    ) -> Insets {
        use crate::mark::bar::Direction;

        let mut insets = Insets::default();
        let is_horizontal = self.data.direction == Direction::Horizontal;

        // Which axis bounds drive the bar end? For horizontal bars, the bar
        // length is plotted along x (from x_min); for vertical, along y.
        let (min_v, max_v) = if is_horizontal { x_bounds } else { y_bounds };
        let range = max_v - min_v;
        if range <= 0.0 {
            return insets;
        }

        let extent = if is_horizontal {
            plot_size.width
        } else {
            plot_size.height
        };
        let pad = 4.0_f32;

        for (series_idx, series) in self.data.series.iter().enumerate() {
            let Some(label_config) = &series.label else {
                continue;
            };
            if label_config.position != Position::Above {
                continue;
            }

            let Some(labels) = state.labels.get(series_idx) else {
                continue;
            };

            for (bar_idx, point) in series.points.iter().enumerate() {
                // point.y carries the bar length in both orientations
                let value = point.y;
                let v = ((value - min_v) / range) as f32;
                // Only bars past midpoint can really need a large inset; the
                // formula naturally yields <=0 otherwise, but skip v<=0 to
                // avoid negative-range division artefacts.
                if v <= 0.0 {
                    continue;
                }

                let Some(bounds) = labels.get(bar_idx).map(|p| p.min_bounds()) else {
                    continue;
                };
                if bounds.width <= 0.0 {
                    continue;
                }
                // The relevant label extent is along the value axis: the
                // shaped paragraph's width for horizontal bars (label at the
                // bar's right end), its height for vertical bars (label above
                // the bar top). The +6/+4 pads mirror the ~3px-each-side /
                // ~2px-top-bottom clearance drawn around a label. Using the
                // wrong axis produces grossly wrong insets — the label clips
                // against the plot area edge.
                let label_extent_along_value_axis = if is_horizontal {
                    bounds.width + 6.0
                } else {
                    bounds.height + 4.0
                };

                // Assumes the other-end inset on the same axis is 0 (safe
                // under-estimate of extent; any actual other-end inset makes
                // the true required value slightly larger, typically <2px).
                let required = extent - (extent - pad - label_extent_along_value_axis) / v;
                let required = required.max(0.0);

                if is_horizontal {
                    insets.right = insets.right.max(required);
                } else {
                    insets.top = insets.top.max(required);
                }
            }
        }

        insets
    }

    /// Layout the bars - calculates bar positions and sizes
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        domain: &Domain,
        rect: Rectangle,
        design: Option<&dyn crate::design::Design>,
    ) -> Node {
        use crate::mark::bar::Direction;

        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        if self.data.series.is_empty() {
            state.series_rects.clear();
            return Node::new(Size::ZERO);
        }

        // Determine number of bins (positions) - use max points across all series
        let num_bins = self.data.series.iter().map(|s| s.points.len()).max().unwrap_or(0);

        if num_bins == 0 {
            state.series_rects.clear();
            return Node::new(Size::ZERO);
        }

        let bar_length = self.data.size.get();
        let spacing_prop = self.data.spacing.get();
        let num_series = self.data.series.len();

        match self.data.direction {
            Direction::Horizontal => {
                self.layout_horizontal(state, domain, rect, num_bins, num_series, bar_length, spacing_prop);
            }
            Direction::Vertical => {
                self.layout_vertical(state, domain, rect, num_bins, num_series, bar_length, spacing_prop);
            }
        }

        // Capture the plot extent so the draw pass can span the track rail
        // across the full value axis behind each bar.
        state.plot_bounds = rect;

        // Compute label rects for hit-testing
        state.label_rects = self.compute_label_rects(state, design);

        // The data-label paragraphs are shaped once in the measurement pass
        // (`PlotArea::shape_labels`, which `min_insets` then measures); this
        // final pass reuses them, so it must not reshape here.

        // Pre-plan fill encodings, one per series. The plan is theme-free
        // and stable across repaints — only re-runs when layout invalidates
        // (data change, resize). Materialization to actual colors happens
        // in `draw` once the theme seed is available.
        state.series_fill_plans = self
            .data
            .series
            .iter()
            .map(|s| s.color_by.as_ref().map(|enc| enc.plan_fill(&s.points)))
            .collect();

        // Bars take no space - they're rendered within the plane
        Node::new(Size::ZERO)
    }

    /// Layout bars vertically (default) - bars grow upward from baseline.
    #[allow(clippy::too_many_arguments)]
    fn layout_vertical(
        &self,
        state: &mut State<Renderer::Paragraph>,
        domain: &Domain,
        rect: Rectangle,
        num_bins: usize,
        num_series: usize,
        bar_length: f32,
        spacing_prop: f32,
    ) {
        use crate::mark::bar::Layout;

        // Calculate zero line position
        let zero_y = to_pixel(domain, rect, Datum::ORIGIN).y;

        match self.data.layout {
            Layout::Grouped => {
                // Total width for all bins (categories)
                let total_width = rect.width;
                let bin_width = total_width / num_bins as f32;

                // Within each bin, calculate bar width based on:
                // - bar_length: proportion of bin width to use for bars
                // - spacing: spacing between bars as proportion of bar width
                // Total bar units: N bars + spacing * (N - 1) gaps
                let total_bar_units = num_series as f32 + spacing_prop * (num_series as f32 - 1.0);
                let bar_width = if total_bar_units > 0.0 {
                    (bin_width * bar_length) / total_bar_units
                } else {
                    bin_width * bar_length
                };
                let spacing_px = bar_width * spacing_prop;

                // Calculate rectangles for each series
                state.series_rects = self
                    .data
                    .series
                    .iter()
                    .enumerate()
                    .map(|(series_idx, series)| {
                        let visual_series_idx = self.data.visual_series_index(series_idx);
                        series
                            .points
                            .iter()
                            .enumerate()
                            .map(|(bin_idx, point)| {
                                let pixel_point = to_pixel(domain, rect, *point);
                                let bar_height = (zero_y - pixel_point.y).abs();

                                // Use plane to transform data coordinate to pixel
                                // For grouped layout, position bars within the group with spacing
                                let bar_center_x = to_pixel(domain, rect, Datum::x(bin_idx as f64)).x;
                                let total_group_width = bin_width * bar_length;
                                let group_start = bar_center_x - total_group_width / 2.0;
                                let x = group_start + visual_series_idx as f32 * (bar_width + spacing_px);

                                Rectangle {
                                    x,
                                    y: pixel_point.y.min(zero_y),
                                    width: bar_width,
                                    height: bar_height,
                                }
                            })
                            .collect()
                    })
                    .collect();
            }
            Layout::Stacked => {
                // For stacked: full width for each bin (spacing has no effect)
                let total_width = rect.width;
                let bin_width = total_width / num_bins as f32;
                let bar_width = bin_width * bar_length;

                // Use fold to accumulate both rectangles and cumulative tops
                let (series_rects, _) = self.data.series.iter().fold(
                    (Vec::new(), vec![zero_y; num_bins]),
                    |(mut all_rects, mut cumulative_tops), series| {
                        let rects = series
                            .points
                            .iter()
                            .enumerate()
                            .map(|(bin_idx, point)| {
                                let pixel_point = to_pixel(domain, rect, *point);
                                let bar_height = (zero_y - pixel_point.y).abs();

                                let y = cumulative_tops[bin_idx] - bar_height;
                                cumulative_tops[bin_idx] = y;

                                // Use plane to transform data coordinate to pixel
                                let bar_center_x = to_pixel(domain, rect, Datum::x(bin_idx as f64)).x;
                                let x = bar_center_x - bar_width / 2.0;

                                Rectangle {
                                    x,
                                    y,
                                    width: bar_width,
                                    height: bar_height,
                                }
                            })
                            .collect();

                        all_rects.push(rects);
                        (all_rects, cumulative_tops)
                    },
                );

                state.series_rects = series_rects;
            }
            Layout::Overlaid => {
                // For overlaid: same position for all series (spacing has no effect)
                let total_width = rect.width;
                let bin_width = total_width / num_bins as f32;
                let bar_width = bin_width * bar_length;

                state.series_rects = self
                    .data
                    .series
                    .iter()
                    .map(|series| {
                        series
                            .points
                            .iter()
                            .enumerate()
                            .map(|(bin_idx, point)| {
                                let pixel_point = to_pixel(domain, rect, *point);
                                let bar_height = (zero_y - pixel_point.y).abs();

                                // Use plane to transform data coordinate to pixel
                                let bar_center_x = to_pixel(domain, rect, Datum::x(bin_idx as f64)).x;
                                let x = bar_center_x - bar_width / 2.0;

                                Rectangle {
                                    x,
                                    y: pixel_point.y.min(zero_y),
                                    width: bar_width,
                                    height: bar_height,
                                }
                            })
                            .collect()
                    })
                    .collect();
            }
        }
    }

    /// Layout bars horizontally - bars grow rightward from baseline.
    #[allow(clippy::too_many_arguments)]
    fn layout_horizontal(
        &self,
        state: &mut State<Renderer::Paragraph>,
        domain: &Domain,
        rect: Rectangle,
        num_bins: usize,
        num_series: usize,
        bar_length: f32,
        spacing_prop: f32,
    ) {
        use crate::mark::bar::Layout;

        // Calculate zero line position (x where value = 0)
        let zero_x = to_pixel(domain, rect, Datum::ORIGIN).x;

        match self.data.layout {
            Layout::Grouped => {
                let total_height = rect.height;
                let bin_height = total_height / num_bins as f32;

                let total_bar_units = num_series as f32 + spacing_prop * (num_series as f32 - 1.0);
                let bar_height = if total_bar_units > 0.0 {
                    (bin_height * bar_length) / total_bar_units
                } else {
                    bin_height * bar_length
                };
                let spacing_px = bar_height * spacing_prop;

                state.series_rects = self
                    .data
                    .series
                    .iter()
                    .enumerate()
                    .map(|(series_idx, series)| {
                        let visual_series_idx = self.data.visual_series_index(series_idx);
                        series
                            .points
                            .iter()
                            .enumerate()
                            .map(|(bin_idx, point)| {
                                // point.x = category index, point.y = value
                                let value_x = to_pixel(domain, rect, Datum::new(point.y, 0.0)).x;
                                let bar_width = (value_x - zero_x).abs();
                                let x = value_x.min(zero_x);

                                // Position category along y-axis
                                let bar_center_y = to_pixel(domain, rect, Datum::new(0.0, bin_idx as f64)).y;
                                let total_group_height = bin_height * bar_length;
                                let group_start = bar_center_y - total_group_height / 2.0;
                                let y = group_start + visual_series_idx as f32 * (bar_height + spacing_px);

                                Rectangle {
                                    x,
                                    y,
                                    width: bar_width,
                                    height: bar_height,
                                }
                            })
                            .collect()
                    })
                    .collect();
            }
            Layout::Stacked => {
                let total_height = rect.height;
                let bin_height = total_height / num_bins as f32;
                let bar_height = bin_height * bar_length;

                // Accumulate along x instead of y
                let (series_rects, _) = self.data.series.iter().fold(
                    (Vec::new(), vec![zero_x; num_bins]),
                    |(mut all_rects, mut cumulative_rights), series| {
                        let rects = series
                            .points
                            .iter()
                            .enumerate()
                            .map(|(bin_idx, point)| {
                                let value_x = to_pixel(domain, rect, Datum::new(point.y, 0.0)).x;
                                let bar_width = (value_x - zero_x).abs();

                                let x = cumulative_rights[bin_idx];
                                cumulative_rights[bin_idx] = x + bar_width;

                                let bar_center_y = to_pixel(domain, rect, Datum::new(0.0, bin_idx as f64)).y;
                                let y = bar_center_y - bar_height / 2.0;

                                Rectangle {
                                    x,
                                    y,
                                    width: bar_width,
                                    height: bar_height,
                                }
                            })
                            .collect();

                        all_rects.push(rects);
                        (all_rects, cumulative_rights)
                    },
                );

                state.series_rects = series_rects;
            }
            Layout::Overlaid => {
                let total_height = rect.height;
                let bin_height = total_height / num_bins as f32;
                let bar_height = bin_height * bar_length;

                state.series_rects = self
                    .data
                    .series
                    .iter()
                    .map(|series| {
                        series
                            .points
                            .iter()
                            .enumerate()
                            .map(|(bin_idx, point)| {
                                let value_x = to_pixel(domain, rect, Datum::new(point.y, 0.0)).x;
                                let bar_width = (value_x - zero_x).abs();
                                let x = value_x.min(zero_x);

                                let bar_center_y = to_pixel(domain, rect, Datum::new(0.0, bin_idx as f64)).y;
                                let y = bar_center_y - bar_height / 2.0;

                                Rectangle {
                                    x,
                                    y,
                                    width: bar_width,
                                    height: bar_height,
                                }
                            })
                            .collect()
                    })
                    .collect();
            }
        }
    }

    /// Computes label bounding rectangles for hit-testing.
    fn compute_label_rects(
        &self,
        state: &State<Renderer::Paragraph>,
        design: Option<&dyn crate::design::Design>,
    ) -> Vec<Vec<Option<Rectangle>>> {
        let is_horizontal = self.data.direction == crate::mark::bar::Direction::Horizontal;

        let default_size = design
            .and_then(|d| d.data_label_text().size)
            .map(|p| p.0)
            .unwrap_or(12.0);

        self.data
            .series
            .iter()
            .zip(state.series_rects.iter())
            .map(|(series, rects)| {
                let Some(label_config) = &series.label else {
                    return rects.iter().map(|_| None).collect();
                };

                let label_size = label_config.text.size.map(|p| p.0).unwrap_or(default_size);

                rects
                    .iter()
                    .zip(series.points.iter())
                    .enumerate()
                    .map(|(bar_idx, (rect, point))| {
                        if !point.y.is_finite() {
                            return None;
                        }
                        let text = (label_config.format)(point.y);
                        if text.is_empty() {
                            return None;
                        }

                        // Resolve per-point size override
                        let size = series
                            .point_label(bar_idx)
                            .and_then(|l| l.size())
                            .map(|p| p.0)
                            .unwrap_or(label_size);

                        let (lx, ly, align_x, align_y) = label_position(rect, label_config.position, is_horizontal);

                        // Estimate text bounds
                        let char_width = size * 0.6;
                        let text_width = text.len() as f32 * char_width + 6.0; // 3px padding each side
                        let text_height = size * 1.2 + 4.0; // 2px padding top/bottom

                        // Convert alignment + position to top-left origin rect
                        let x = match align_x {
                            crate::core::alignment::Horizontal::Left => lx,
                            crate::core::alignment::Horizontal::Center => lx - text_width / 2.0,
                            crate::core::alignment::Horizontal::Right => lx - text_width,
                        };
                        let y = match align_y {
                            crate::core::alignment::Vertical::Top => ly,
                            crate::core::alignment::Vertical::Center => ly - text_height / 2.0,
                            crate::core::alignment::Vertical::Bottom => ly - text_height,
                        };

                        Some(Rectangle {
                            x,
                            y,
                            width: text_width,
                            height: text_height,
                        })
                    })
                    .collect()
            })
            .collect()
    }

    /// Shapes the data-label paragraphs, one per drawn bar, keyed `[series][bar]`
    /// to mirror `series_rects` so `draw` can index a bar's label directly.
    ///
    /// The font is resolved theme-free — the label's own family/weight/style
    /// layered on [`crate::core::text::Renderer::default_font`] — so that
    /// `layout` stays independent of the active theme (a theme that overrides
    /// the data-label *font family* will not affect these metrics; color stays
    /// a draw-time concern). Bars with a non-finite value or empty formatted
    /// text keep a default (empty) paragraph; the draw pass skips them with the
    /// same guards, so the index stays aligned with `bar_idx`.
    pub(super) fn shape_labels(
        &self,
        state: &mut State<Renderer::Paragraph>,
        renderer: &Renderer,
        design: Option<&dyn crate::design::Design>,
    ) {
        use crate::core::alignment;

        let default_font = design
            .map(|d| d.data_label_text().resolved_font(renderer.default_font()))
            .unwrap_or_else(|| renderer.default_font());
        let default_size = design
            .and_then(|d| d.data_label_text().size)
            .map(|p| p.0)
            .unwrap_or(12.0);
        let hint_factor = renderer.scale_factor();

        while state.labels.len() < self.data.series.len() {
            state.labels.push(Vec::new());
        }
        state.labels.truncate(self.data.series.len());

        for (series_idx, series) in self.data.series.iter().enumerate() {
            let labels = &mut state.labels[series_idx];

            let Some(label_config) = &series.label else {
                labels.clear();
                continue;
            };

            // One paragraph per data point. Shaping depends only on the
            // point's value + the label font/size, not on bar geometry, so
            // this runs in the measurement pass before the plane is known.
            let bar_count = series.points.len();

            while labels.len() < bar_count {
                labels.push(paragraph::Plain::default());
            }
            labels.truncate(bar_count);

            let config_size = label_config.text.size.map(|p| p.0).unwrap_or(default_size);

            for (bar_idx, point) in series.points.iter().take(bar_count).enumerate() {
                let paragraph = &mut labels[bar_idx];

                if !point.y.is_finite() {
                    continue;
                }

                let content = (label_config.format)(point.y);
                if content.is_empty() {
                    continue;
                }

                let point_label = series.point_label(bar_idx);
                let size: crate::core::Pixels = point_label
                    .and_then(|l| l.size())
                    .map(|p| p.0)
                    .unwrap_or(config_size)
                    .into();
                let mut font = label_config.text.resolved_font(default_font);
                if let Some(w) = point_label.and_then(|l| l.weight()).or(label_config.text.weight) {
                    font.weight = w;
                }
                if let Some(s) = point_label.and_then(|l| l.style()).or(label_config.text.style) {
                    font.style = s;
                }

                let _ = paragraph.update(text::Text {
                    content: &content,
                    bounds: Size::INFINITE,
                    size,
                    line_height: text::LineHeight::default(),
                    font,
                    align_x: text::Alignment::Left,
                    align_y: alignment::Vertical::Top,
                    shaping: text::Shaping::Basic,
                    wrapping: text::Wrapping::None,
                    ellipsis: text::Ellipsis::default(),
                    hint_factor,
                    font_features: Vec::new(),
                    font_variations: Vec::new(),
                    letter_spacing: Default::default(),
                    weight: None,
                });
            }
        }
    }

    /// Draws the bars using pre-calculated rectangles
    #[allow(clippy::too_many_arguments)]
    pub fn draw<Theme>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        _style: &crate::core::renderer::Style,
        _layout: crate::core::Layout<'_>,
        _cursor: crate::core::mouse::Cursor,
        _viewport: &crate::core::Rectangle,
        color_offset: usize,
        palette: &crate::palette::Resolved,
        chart_user_palette: Option<&crate::palette::Palette>,
        mark_index: usize,
        selection: &Option<crate::target::Target>,
        corners: crate::core::border::Radius,
        track: Option<crate::core::Color>,
        baseline_exposed: bool,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        let background = theme.background_color();
        let text_pair = theme.text_pair();

        // Get the layout bounds to offset the bars to their actual screen position
        let layout_bounds = _layout.bounds();

        // Single frame for all bar series
        let mut bar_frame = Frame::new(renderer, layout_bounds.size());
        // Single frame for label background fills (the text itself is drawn
        // directly via `fill_paragraph`, not into this frame).
        let mut label_frame = Frame::new(renderer, layout_bounds.size());

        // Data-label render list: `(series, bar, anchor, color)`. The anchor is
        // layout-local (matching the bar rects); it is offset by `layout_bounds`
        // at render time. Collected during the series loop and drawn after the
        // background frame so glyphs sit on top of their fills.
        let mut label_draws: Vec<(usize, usize, Point, crate::core::Color)> = Vec::new();

        // Resolve all bar colors via the full priority chain:
        // point_colors > color_by encoding > series.color > palette fallback.
        // See GOG.md § 7.
        let mut all_bar_colors: Vec<Vec<crate::core::Color>> = Vec::new();

        let is_horizontal = self.data.direction == crate::mark::bar::Direction::Horizontal;
        // The value-end corner radii (top for columns, right for bars). A
        // single resolved `Radius` flows in from the chart's `Style.corners`
        // (theme default, overridable per chart via `.style()`); only the
        // value-end pair applies to bars.
        let max_end_radius = if is_horizontal {
            corners.top_right.max(corners.bottom_right)
        } else {
            corners.top_left.max(corners.top_right)
        };

        // Sweep: each bar's rectangle interpolates from its previous-layout
        // rect (or from a column-shared baseline on fresh mount) toward its
        // current rect. The mount anchor is the bottom-most edge of the
        // column for vertical layouts (or leftmost for horizontal) — for
        // stacked layouts every segment in the column collapses to the
        // same edge, keeping the stack flush against itself instead of
        // opening gaps between segments. With `animate = false` progress
        // pins to `1.0` and the animated rect equals
        // `state.series_rects[s][i]`, reproducing today's geometry exactly.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animating = progress < 1.0 - f32::EPSILON;
        let n_columns = state.series_rects.iter().map(|s| s.len()).max().unwrap_or(0);
        let baselines: Vec<f32> = (0..n_columns)
            .map(|col| {
                let init = if is_horizontal {
                    f32::INFINITY
                } else {
                    f32::NEG_INFINITY
                };
                state
                    .series_rects
                    .iter()
                    .filter_map(|s| s.get(col))
                    .fold(init, |acc, r| {
                        if is_horizontal {
                            acc.min(r.x)
                        } else {
                            acc.max(r.y + r.height)
                        }
                    })
            })
            .collect();
        let animated_series_rects: Vec<Vec<Rectangle>> = state
            .series_rects
            .iter()
            .enumerate()
            .map(|(s, rects)| {
                rects
                    .iter()
                    .enumerate()
                    .map(|(i, cur)| {
                        let prev = state.previous_series_rects.get(s).and_then(|v| v.get(i)).copied();
                        let baseline =
                            baselines
                                .get(i)
                                .copied()
                                .unwrap_or(if is_horizontal { cur.x } else { cur.y + cur.height });
                        animate_rect(*cur, prev, progress, is_horizontal, baseline)
                    })
                    .collect()
            })
            .collect();

        // For stacked layout, only the topmost segment of each stack should
        // round its end corners, so middle segments meet without gaps.
        let total_series = self.data.series.len();
        let is_stacked = self.data.layout == crate::mark::bar::Layout::Stacked;

        // The design's palette seed is used by fill encodings to build a
        // categorical sub-palette sized to the distinct-key count. Theme is
        // invariant across series, so compute the seed once per draw.
        let seed = theme.seed();

        // Track rail: a faint full-extent pill behind every bar showing the
        // axis range the bar grows within (Recharts `<Bar background>`).
        // Drawn first so bars paint on top, and built as one combined path
        // filled once — a single blend keeps the translucent color from
        // doubling up where bands overlap (stacked / overlaid share a band).
        // Uses the laid-out `series_rects` (not the animated rects) so the
        // rail stays put while bars grow into it.
        if let Some(track_color) = track {
            let plot = state.plot_bounds;
            let track_path = Path::new(|builder| {
                for rects in state.series_rects.iter() {
                    for r in rects {
                        if !(r.x.is_finite() && r.y.is_finite() && r.width.is_finite() && r.height.is_finite()) {
                            continue;
                        }
                        let track_rect = if is_horizontal {
                            Rectangle {
                                x: plot.x,
                                y: r.y,
                                width: plot.width,
                                height: r.height,
                            }
                        } else {
                            Rectangle {
                                x: r.x,
                                y: plot.y,
                                width: r.width,
                                height: plot.height,
                            }
                        };
                        // The far (value) end is always rounded; the baseline
                        // end rounds only when its axis spine is hidden, so a
                        // visible spine keeps the rail flush against it.
                        push_bar_path(builder, &track_rect, corners, is_horizontal, true, baseline_exposed);
                    }
                }
            });
            bar_frame.fill(&track_path, track_color);
        }

        // Draw each series
        for (series_idx, (series, rects)) in self.data.series.iter().zip(animated_series_rects.iter()).enumerate() {
            let round_this_series = max_end_radius > 0.0 && (!is_stacked || series_idx + 1 == total_series);
            // Determine base color for this series
            let base_color = if let Some(series_color) = series.color {
                series_color.resolve(background, text_pair, &seed, None)
            } else {
                palette
                    .get(color_offset + series_idx)
                    .resolve(background, text_pair, &seed, None)
            };

            // Materialize the cached fill plan for this series. The expensive
            // walk (extractor + distinct-key indexing) happened in `layout`;
            // here we only build the palette from the seed and look up by
            // pre-computed index. See GOG.md § 7 for priority and § 8a for
            // the integration contract.
            let fill_colors = state
                .series_fill_plans
                .get(series_idx)
                .and_then(|p| p.as_ref())
                .map(|plan| plan.materialize(&seed, chart_user_palette));

            // Resolve per-bar colors following the priority chain:
            // point_colors > color_by > series.color > palette fallback.
            let bar_colors: Vec<crate::core::Color> = rects
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    let resolved = if let Some(pc) = series.point_color(i) {
                        *pc
                    } else if let Some(c) = fill_colors.as_ref().and_then(|v| v[i]) {
                        c
                    } else if let Some(sc) = series.color {
                        sc
                    } else {
                        palette.get(color_offset + series_idx)
                    };
                    resolved.resolve(background, text_pair, &seed, None)
                })
                .collect();

            // Round the baseline end only when its axis spine is hidden — a
            // visible spine means bars should sit flush against it, so they
            // touch the line instead of floating as pills. In a stack only
            // the baseline-most segment (the first) rounds its baseline end.
            let round_baseline = max_end_radius > 0.0 && baseline_exposed && (!is_stacked || series_idx == 0);

            // Draw bars: per-bar fill if any per-point override OR any fill
            // encoding is present; otherwise batch into a single Path.
            if series.has_point_colors() || series.color_by.is_some() {
                for (rect, &color) in rects.iter().zip(bar_colors.iter()) {
                    let path = Path::new(|builder| {
                        push_bar_path(builder, rect, corners, is_horizontal, round_this_series, round_baseline);
                    });
                    bar_frame.fill(&path, color);
                }
            } else {
                let path = Path::new(|builder| {
                    for rect in rects {
                        push_bar_path(builder, rect, corners, is_horizontal, round_this_series, round_baseline);
                    }
                });
                bar_frame.fill(&path, base_color);
            }

            all_bar_colors.push(bar_colors);

            // Draw labels for this series if configured. Suppressed
            // mid-sweep so they don't pop in over bars that haven't
            // grown into their final positions yet.
            if let Some(label_config) = &series.label
                && !animating
            {
                let is_horizontal = self.data.direction == crate::mark::bar::Direction::Horizontal;

                for (bar_idx, (rect, point)) in rects.iter().zip(series.points.iter()).enumerate() {
                    if !point.y.is_finite() {
                        continue;
                    }
                    if (label_config.format)(point.y).is_empty() {
                        continue;
                    }

                    // Calculate label position using shared helper
                    let (label_x, label_y, align_x, align_y) =
                        label_position(rect, label_config.position, is_horizontal);

                    // Merge per-point label overrides
                    let point_label = series.point_label(bar_idx);
                    let effective_fill = point_label.and_then(|l| l.fill().copied()).or(label_config.fill);

                    // Draw fill background if specified
                    if let Some(fill_color_spec) = effective_fill
                        && let Some(Some(lr)) = state.label_rects.get(series_idx).and_then(|rects| rects.get(bar_idx))
                    {
                        let fill_resolved = fill_color_spec.resolve(background, text_pair, &seed, None);
                        let fill_path = Path::new(|b| {
                            b.rectangle(
                                crate::core::Point::new(lr.x, lr.y),
                                crate::core::Size::new(lr.width, lr.height),
                            );
                        });
                        label_frame.fill(&fill_path, fill_resolved);
                    }

                    // Resolve label color based on position
                    let label_color_spec = point_label
                        .and_then(|l| l.color().copied())
                        .or(label_config.color)
                        .unwrap_or(crate::color::Color::CONTRAST);

                    // Use per-bar resolved color for contrast
                    let this_bar_color = all_bar_colors[series_idx][bar_idx];

                    let label_color = match label_config.position {
                        Position::Above => {
                            let label_point = crate::core::Point::new(label_x, label_y);

                            let containing_bar = all_bar_colors.iter().zip(animated_series_rects.iter()).find_map(
                                |(colors, other_rects)| {
                                    other_rects
                                        .iter()
                                        .zip(colors.iter())
                                        .find(|(other_rect, _)| other_rect.contains(label_point))
                                        .map(|(_, color)| *color)
                                },
                            );

                            if let Some(other_color) = containing_bar {
                                label_color_spec.resolve(other_color, text_pair, &seed, Some(background))
                            } else {
                                label_color_spec.resolve(background, text_pair, &seed, None)
                            }
                        }
                        _ => label_color_spec.resolve(this_bar_color, text_pair, &seed, Some(background)),
                    };

                    // Anchor the cached paragraph so its top-left lands where
                    // `fill_text` would have placed the glyph box. `fill_text`
                    // shifts the box from `position` by the paragraph's
                    // `min_bounds` per alignment (Center: −½, Right/Bottom: −1);
                    // `fill_paragraph` draws from the top-left, so we apply the
                    // same shift here against the same `min_bounds`. Identical
                    // formula, identical metrics — pixel-identical placement.
                    let Some(paragraph) = state.labels.get(series_idx).and_then(|s| s.get(bar_idx)) else {
                        continue;
                    };
                    let bounds = paragraph.min_bounds();
                    let anchor_x = match align_x {
                        crate::core::alignment::Horizontal::Left => label_x,
                        crate::core::alignment::Horizontal::Center => label_x - bounds.width / 2.0,
                        crate::core::alignment::Horizontal::Right => label_x - bounds.width,
                    };
                    let anchor_y = match align_y {
                        crate::core::alignment::Vertical::Top => label_y,
                        crate::core::alignment::Vertical::Center => label_y - bounds.height / 2.0,
                        crate::core::alignment::Vertical::Bottom => label_y - bounds.height,
                    };
                    label_draws.push((series_idx, bar_idx, Point::new(anchor_x, anchor_y), label_color));
                }
            }
        }

        // Draw selection highlights — only painted once the mount sweep
        // completes so the highlight doesn't ride along with bars that
        // haven't grown into their final positions yet.
        let mut selection_frame = Frame::new(renderer, layout_bounds.size());

        if let Some(target) = selection
            && !animating
        {
            use crate::target::Target;
            use crate::widget::canvas::Stroke;

            let inner_color = crate::core::Color::from_rgba(0.0, 0.0, 0.0, 0.5);
            let outer_color = crate::core::Color::from_rgba(1.0, 1.0, 1.0, 0.6);

            let should_highlight = |series_idx: usize, bar_idx: usize| -> bool {
                match target {
                    Target::Mark(m) => *m == mark_index,
                    Target::Series { mark, series } => *mark == mark_index && *series == series_idx,
                    Target::Entry { mark, series, index } => {
                        *mark == mark_index && *series == series_idx && *index == bar_idx
                    }
                    _ => false,
                }
            };

            let should_highlight_label = |series_idx: usize, bar_idx: usize| -> bool {
                match target {
                    Target::SeriesLabel { mark, series } => *mark == mark_index && *series == series_idx,
                    Target::EntryLabel { mark, series, index } => {
                        *mark == mark_index && *series == series_idx && *index == bar_idx
                    }
                    _ => false,
                }
            };

            for (series_idx, rects) in state.series_rects.iter().enumerate() {
                for (bar_idx, rect) in rects.iter().enumerate() {
                    let rect_finite =
                        rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite();
                    if should_highlight(series_idx, bar_idx) && rect_finite {
                        // Outer white stroke on expanded rect
                        let outer_rect = Rectangle {
                            x: rect.x - 1.0,
                            y: rect.y - 1.0,
                            width: rect.width + 2.0,
                            height: rect.height + 2.0,
                        };
                        let outer_path = Path::new(|b| {
                            b.rectangle(
                                crate::core::Point::new(outer_rect.x, outer_rect.y),
                                crate::core::Size::new(outer_rect.width, outer_rect.height),
                            );
                        });
                        selection_frame.stroke(&outer_path, Stroke::default().with_color(outer_color).with_width(1.0));

                        // Inner black stroke on exact rect
                        let inner_path = Path::new(|b| {
                            b.rectangle(
                                crate::core::Point::new(rect.x, rect.y),
                                crate::core::Size::new(rect.width, rect.height),
                            );
                        });
                        selection_frame.stroke(&inner_path, Stroke::default().with_color(inner_color).with_width(1.0));
                    }

                    // Label selection highlight
                    if should_highlight_label(series_idx, bar_idx)
                        && let Some(Some(lr)) = state.label_rects.get(series_idx).and_then(|rects| rects.get(bar_idx))
                    {
                        let outer_rect = Rectangle {
                            x: lr.x - 1.0,
                            y: lr.y - 1.0,
                            width: lr.width + 2.0,
                            height: lr.height + 2.0,
                        };
                        let outer_path = Path::new(|b| {
                            b.rectangle(
                                crate::core::Point::new(outer_rect.x, outer_rect.y),
                                crate::core::Size::new(outer_rect.width, outer_rect.height),
                            );
                        });
                        selection_frame.stroke(&outer_path, Stroke::default().with_color(outer_color).with_width(1.0));

                        let inner_path = Path::new(|b| {
                            b.rectangle(
                                crate::core::Point::new(lr.x, lr.y),
                                crate::core::Size::new(lr.width, lr.height),
                            );
                        });
                        selection_frame.stroke(&inner_path, Stroke::default().with_color(inner_color).with_width(1.0));
                    }
                }
            }
        }

        // Draw all bars, then all labels, then selection on top
        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);

        let bar_geometry = bar_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(bar_geometry);
        });

        let label_geometry = label_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(label_geometry);
        });

        // Draw the data-label text on top of any background fills via cached
        // paragraphs shaped in `layout`. Anchors are layout-local, so add the
        // plot translation to reach absolute coordinates. Clip to the plot
        // rect (`layout_bounds`) — the same clip the label canvas frame
        // applied before — so an overhanging label is cut at the plot edge
        // exactly as it was when shaped via `fill_text`.
        for (series_idx, bar_idx, anchor, color) in label_draws {
            if let Some(paragraph) = state.labels.get(series_idx).and_then(|s| s.get(bar_idx)) {
                let anchor = anchor + translation;
                renderer.fill_paragraph(paragraph.raw(), anchor, color, layout_bounds);
            }
        }

        let selection_geometry = selection_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(selection_geometry);
        });
    }
}

/// Computes label position and alignment for a bar rectangle.
fn label_position(
    rect: &Rectangle,
    position: Position,
    is_horizontal: bool,
) -> (
    f32,
    f32,
    crate::core::alignment::Horizontal,
    crate::core::alignment::Vertical,
) {
    use crate::core::alignment::{Horizontal, Vertical};

    // Gap between a data label and the bar edge it sits against. Labels
    // tucked inside the bar (End/Base) get a roomier inset so the text isn't
    // cramped against the rounded value end; labels floating just outside
    // (Above) keep a smaller gap.
    const INSIDE: f32 = 8.0;
    const OUTSIDE: f32 = 4.0;

    if is_horizontal {
        match position {
            Position::Above => (
                rect.x + rect.width + OUTSIDE,
                rect.y + rect.height / 2.0,
                Horizontal::Left,
                Vertical::Center,
            ),
            Position::End => (
                rect.x + rect.width - INSIDE,
                rect.y + rect.height / 2.0,
                Horizontal::Right,
                Vertical::Center,
            ),
            Position::Center => (
                rect.x + rect.width / 2.0,
                rect.y + rect.height / 2.0,
                Horizontal::Center,
                Vertical::Center,
            ),
            Position::Base => (
                rect.x + INSIDE,
                rect.y + rect.height / 2.0,
                Horizontal::Left,
                Vertical::Center,
            ),
        }
    } else {
        match position {
            Position::Above => (
                rect.x + rect.width / 2.0,
                rect.y - OUTSIDE,
                Horizontal::Center,
                Vertical::Bottom,
            ),
            Position::End => (
                rect.x + rect.width / 2.0,
                rect.y + INSIDE,
                Horizontal::Center,
                Vertical::Top,
            ),
            Position::Center => (
                rect.x + rect.width / 2.0,
                rect.y + rect.height / 2.0,
                Horizontal::Center,
                Vertical::Center,
            ),
            Position::Base => (
                rect.x + rect.width / 2.0,
                rect.y + rect.height - INSIDE,
                Horizontal::Center,
                Vertical::Bottom,
            ),
        }
    }
}

/// Computes the on-screen rectangle for a bar at the current sweep
/// progress. With a `prev` rect (data change), x/y/width/height each
/// lerp linearly from `prev` to `cur`. Without one (fresh mount), the
/// bar grows from the column-shared `baseline`:
///
/// - Vertical bars: top edge slides up from `baseline` toward `cur.y`,
///   bottom edge slides up at the same speed. Height scales by
///   `progress`. For stacked segments sharing a column, every segment
///   collapses to the same `baseline` edge at `progress = 0` and pulls
///   apart proportionally as `progress` advances, so adjacent segments
///   stay flush against each other.
/// - Horizontal bars: left edge slides right from `baseline` toward
///   `cur.x`, right edge tracks at the same rate. Width scales by
///   `progress`.
///
/// At `progress == 1.0` the result equals `cur` exactly in every
/// branch, so disabling animation reproduces today's geometry.
fn animate_rect(
    cur: Rectangle,
    prev: Option<Rectangle>,
    progress: f32,
    is_horizontal: bool,
    baseline: f32,
) -> Rectangle {
    if let Some(prev) = prev {
        let x = prev.x + (cur.x - prev.x) * progress;
        let y = prev.y + (cur.y - prev.y) * progress;
        let width = prev.width + (cur.width - prev.width) * progress;
        let height = prev.height + (cur.height - prev.height) * progress;
        Rectangle { x, y, width, height }
    } else if is_horizontal {
        Rectangle {
            x: baseline + (cur.x - baseline) * progress,
            y: cur.y,
            width: cur.width * progress,
            height: cur.height,
        }
    } else {
        Rectangle {
            x: cur.x,
            y: baseline - (baseline - cur.y) * progress,
            width: cur.width,
            height: cur.height * progress,
        }
    }
}

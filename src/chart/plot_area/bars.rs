use super::{Plane, PlotInsets};
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size};
use crate::data::Datum;
use crate::mark::bar::label::Position;
use crate::widget::canvas::{Frame, Path, Text as CanvasText};
use crate::widget::renderer::geometry;

/// Builds a rectangle path with optionally rounded "end" corners.
///
/// For vertical bars, the top two corners are rounded; for horizontal bars,
/// the right two corners are rounded. A `radius` of `0.0` produces a plain
/// rectangle. The radius is clamped so it can never exceed `min(w, h) / 2`.
fn push_bar_path(
    builder: &mut crate::widget::canvas::path::Builder,
    rect: &Rectangle,
    radius: f32,
    is_horizontal: bool,
) {
    let w = rect.width;
    let h = rect.height;
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let r = radius.min(w * 0.5).min(h * 0.5);
    if r <= 0.0 {
        builder.rectangle(Point::new(rect.x, rect.y), Size::new(w, h));
        return;
    }

    // Corners: TL, TR, BR, BL.
    let tl = Point::new(rect.x, rect.y);
    let tr = Point::new(rect.x + w, rect.y);
    let br = Point::new(rect.x + w, rect.y + h);
    let bl = Point::new(rect.x, rect.y + h);

    if is_horizontal {
        // Round TR and BR (right edge).
        builder.move_to(tl);
        builder.line_to(Point::new(tr.x - r, tr.y));
        builder.arc_to(tr, Point::new(tr.x, tr.y + r), r);
        builder.line_to(Point::new(br.x, br.y - r));
        builder.arc_to(br, Point::new(br.x - r, br.y), r);
        builder.line_to(bl);
        builder.close();
    } else {
        // Round TL and TR (top edge).
        builder.move_to(Point::new(tl.x, tl.y + r));
        builder.arc_to(tl, Point::new(tl.x + r, tl.y), r);
        builder.line_to(Point::new(tr.x - r, tr.y));
        builder.arc_to(tr, Point::new(tr.x, tr.y + r), r);
        builder.line_to(br);
        builder.line_to(bl);
        builder.close();
    }
}
/// State for Bars - stores positioned bar rectangles
pub struct State {
    /// Pixel rectangles for each series, outer vec is series, inner vec is bars
    pub series_rects: Vec<Vec<Rectangle>>,
    /// Pixel rectangles for each label, outer vec is series, inner vec is bars
    pub label_rects: Vec<Vec<Option<Rectangle>>>,
    /// Theme-independent fill encoding plans, one per series. `None` for
    /// series with no `color_by` encoding. Computed in `layout` so the
    /// encoding's expensive walk (extractor calls, distinct-key indexing)
    /// only runs when the data changes, not on every repaint.
    pub series_fill_plans: Vec<Option<crate::encoding::FillPlan>>,
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
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Bars<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    /// Create a new Bars borrowing data
    pub fn new(data: &'a crate::bar::Bars) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Bars
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                series_rects: Vec::new(),
                label_rects: Vec::new(),
                series_fill_plans: Vec::new(),
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
    /// For each bar, solves `R >= extent - (extent - pad - label_width) / v`
    /// where `v ∈ [0, 1]` is the bar's normalized position along the value
    /// axis. Returns the max across all bars.
    pub(super) fn compute_label_insets(
        &self,
        plot_size: Size,
        x_bounds: (f64, f64),
        y_bounds: (f64, f64),
    ) -> PlotInsets {
        use crate::mark::bar::Direction;

        let mut insets = PlotInsets::default();
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

        for series in &self.data.series {
            let Some(label_config) = &series.label else {
                continue;
            };
            if label_config.position != Position::Above {
                continue;
            }

            let label_size = label_config.text.size.map(|p| p.0).unwrap_or(12.0);
            let char_width = label_size * 0.6;
            // Vertical text run for line-height-ish font metrics. 1.2
            // is the iced LineHeight::default() coefficient; +4 matches
            // the ~2px top/bottom pad drawn around a label.
            let label_height = label_size * 1.2 + 4.0;

            for point in &series.points {
                // point.y carries the bar length in both orientations
                let value = point.y;
                let v = ((value - min_v) / range) as f32;
                // Only bars past midpoint can really need a large inset; the
                // formula naturally yields <=0 otherwise, but skip v<=0 to
                // avoid negative-range division artefacts.
                if v <= 0.0 {
                    continue;
                }

                let text = (label_config.format)(value);
                if text.is_empty() {
                    continue;
                }
                // The relevant label extent is along the value axis:
                // text width for horizontal bars (label at the bar's
                // right end), text height for vertical bars (label
                // above the bar top). Using the wrong one produces
                // grossly wrong top insets on vertical bars — the
                // label clips against the plot area top because the
                // reserved space is derived from a horizontal measure.
                let label_extent_along_value_axis = if is_horizontal {
                    text.len() as f32 * char_width + 6.0
                } else {
                    label_height
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
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        use crate::mark::bar::Direction;

        let state = tree.state.downcast_mut::<State>();

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
                self.layout_horizontal(state, plane, num_bins, num_series, bar_length, spacing_prop);
            }
            Direction::Vertical => {
                self.layout_vertical(state, plane, num_bins, num_series, bar_length, spacing_prop);
            }
        }

        // Compute label rects for hit-testing
        state.label_rects = self.compute_label_rects(state);

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
    fn layout_vertical(
        &self,
        state: &mut State,
        plane: &Plane,
        num_bins: usize,
        num_series: usize,
        bar_length: f32,
        spacing_prop: f32,
    ) {
        use crate::mark::bar::Layout;

        // Calculate zero line position
        let zero_y = plane.to_pixel(Datum::ORIGIN).y;

        match self.data.layout {
            Layout::Grouped => {
                // Total width for all bins (categories)
                let total_width = plane.bounds.width;
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
                        series
                            .points
                            .iter()
                            .enumerate()
                            .map(|(bin_idx, point)| {
                                let pixel_point = plane.to_pixel(*point);
                                let bar_height = (zero_y - pixel_point.y).abs();

                                // Use plane to transform data coordinate to pixel
                                // For grouped layout, position bars within the group with spacing
                                let bar_center_x = plane.to_pixel(Datum::x(bin_idx as f64)).x;
                                let total_group_width = bin_width * bar_length;
                                let group_start = bar_center_x - total_group_width / 2.0;
                                let x = group_start + series_idx as f32 * (bar_width + spacing_px);

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
                let total_width = plane.bounds.width;
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
                                let pixel_point = plane.to_pixel(*point);
                                let bar_height = (zero_y - pixel_point.y).abs();

                                let y = cumulative_tops[bin_idx] - bar_height;
                                cumulative_tops[bin_idx] = y;

                                // Use plane to transform data coordinate to pixel
                                let bar_center_x = plane.to_pixel(Datum::x(bin_idx as f64)).x;
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
                let total_width = plane.bounds.width;
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
                                let pixel_point = plane.to_pixel(*point);
                                let bar_height = (zero_y - pixel_point.y).abs();

                                // Use plane to transform data coordinate to pixel
                                let bar_center_x = plane.to_pixel(Datum::x(bin_idx as f64)).x;
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
    fn layout_horizontal(
        &self,
        state: &mut State,
        plane: &Plane,
        num_bins: usize,
        num_series: usize,
        bar_length: f32,
        spacing_prop: f32,
    ) {
        use crate::mark::bar::Layout;

        // Calculate zero line position (x where value = 0)
        let zero_x = plane.to_pixel(Datum::ORIGIN).x;

        match self.data.layout {
            Layout::Grouped => {
                let total_height = plane.bounds.height;
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
                        series
                            .points
                            .iter()
                            .enumerate()
                            .map(|(bin_idx, point)| {
                                // point.x = category index, point.y = value
                                let value_x = plane.to_pixel(Datum::new(point.y, 0.0)).x;
                                let bar_width = (value_x - zero_x).abs();
                                let x = value_x.min(zero_x);

                                // Position category along y-axis
                                let bar_center_y = plane.to_pixel(Datum::new(0.0, bin_idx as f64)).y;
                                let total_group_height = bin_height * bar_length;
                                let group_start = bar_center_y - total_group_height / 2.0;
                                let y = group_start + series_idx as f32 * (bar_height + spacing_px);

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
                let total_height = plane.bounds.height;
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
                                let value_x = plane.to_pixel(Datum::new(point.y, 0.0)).x;
                                let bar_width = (value_x - zero_x).abs();

                                let x = cumulative_rights[bin_idx];
                                cumulative_rights[bin_idx] = x + bar_width;

                                let bar_center_y = plane.to_pixel(Datum::new(0.0, bin_idx as f64)).y;
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
                let total_height = plane.bounds.height;
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
                                let value_x = plane.to_pixel(Datum::new(point.y, 0.0)).x;
                                let bar_width = (value_x - zero_x).abs();
                                let x = value_x.min(zero_x);

                                let bar_center_y = plane.to_pixel(Datum::new(0.0, bin_idx as f64)).y;
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
    fn compute_label_rects(&self, state: &State) -> Vec<Vec<Option<Rectangle>>> {
        let is_horizontal = self.data.direction == crate::mark::bar::Direction::Horizontal;

        self.data
            .series
            .iter()
            .zip(state.series_rects.iter())
            .map(|(series, rects)| {
                let Some(label_config) = &series.label else {
                    return rects.iter().map(|_| None).collect();
                };

                let label_size = label_config.text.size.map(|p| p.0).unwrap_or(12.0);

                rects
                    .iter()
                    .zip(series.points.iter())
                    .enumerate()
                    .map(|(bar_idx, (rect, point))| {
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
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        let background = theme.background_color();
        let text_pair = theme.text_pair();

        // Get the layout bounds to offset the bars to their actual screen position
        let layout_bounds = _layout.bounds();

        // Single frame for all bar series
        let mut bar_frame = Frame::new(renderer, layout_bounds.size());
        // Single frame for all labels
        let mut label_frame = Frame::new(renderer, layout_bounds.size());

        // Resolve all bar colors via the full priority chain:
        // point_colors > color_by encoding > series.color > palette fallback.
        // See GOG.md § 7.
        let mut all_bar_colors: Vec<Vec<crate::core::Color>> = Vec::new();

        let is_horizontal = self.data.direction == crate::mark::bar::Direction::Horizontal;
        let radius = self.data.corner_radius;
        // For stacked layout, only the topmost segment of each stack should
        // round its end corners, so middle segments meet without gaps.
        let total_series = self.data.series.len();
        let is_stacked = self.data.layout == crate::mark::bar::Layout::Stacked;

        // The design's palette seed is used by fill encodings to build a
        // categorical sub-palette sized to the distinct-key count. Theme is
        // invariant across series, so compute the seed once per draw.
        let seed = theme.palette_seed();

        // Draw each series
        for (series_idx, (series, rects)) in self.data.series.iter().zip(state.series_rects.iter()).enumerate() {
            let round_this_series = radius > 0.0 && (!is_stacked || series_idx + 1 == total_series);
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

            let active_radius = if round_this_series { radius } else { 0.0 };

            // Draw bars: per-bar fill if any per-point override OR any fill
            // encoding is present; otherwise batch into a single Path.
            if series.has_point_colors() || series.color_by.is_some() {
                for (rect, &color) in rects.iter().zip(bar_colors.iter()) {
                    let path = Path::new(|builder| {
                        push_bar_path(builder, rect, active_radius, is_horizontal);
                    });
                    bar_frame.fill(&path, color);
                }
            } else {
                let path = Path::new(|builder| {
                    for rect in rects {
                        push_bar_path(builder, rect, active_radius, is_horizontal);
                    }
                });
                bar_frame.fill(&path, base_color);
            }

            all_bar_colors.push(bar_colors);

            // Draw labels for this series if configured
            if let Some(label_config) = &series.label {
                let label_size = label_config.text.size.map(|p| p.0).unwrap_or(12.0);
                let is_horizontal = self.data.direction == crate::mark::bar::Direction::Horizontal;

                for (bar_idx, (rect, point)) in rects.iter().zip(series.points.iter()).enumerate() {
                    // Format the label text
                    let label_text = (label_config.format)(point.y);

                    // Calculate label position using shared helper
                    let (label_x, label_y, align_x, align_y) =
                        label_position(rect, label_config.position, is_horizontal);

                    // Merge per-point label overrides
                    let point_label = series.point_label(bar_idx);
                    let effective_size = point_label.and_then(|l| l.size()).map(|p| p.0).unwrap_or(label_size);
                    let effective_weight = point_label.and_then(|l| l.weight()).or(label_config.text.weight);
                    let effective_style = point_label.and_then(|l| l.style()).or(label_config.text.style);
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

                            let containing_bar = all_bar_colors.iter().zip(state.series_rects.iter()).find_map(
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

                    // Resolve the font: label's family/weight/style layered
                    // on top of the theme's data-label default.
                    let base_font = theme.data_label_text().resolved_font(theme.font());
                    let mut font = label_config.text.resolved_font(base_font);
                    if let Some(w) = effective_weight {
                        font.weight = w;
                    }
                    if let Some(s) = effective_style {
                        font.style = s;
                    }

                    label_frame.fill_text(CanvasText {
                        content: label_text,
                        position: crate::core::Point::new(label_x, label_y),
                        color: label_color,
                        size: effective_size.into(),
                        font,
                        align_x: align_x.into(),
                        align_y,
                        line_height: crate::core::text::LineHeight::default(),
                        shaping: crate::core::text::Shaping::Basic,
                        ..CanvasText::default()
                    });
                }
            }
        }

        // Draw selection highlights
        let mut selection_frame = Frame::new(renderer, layout_bounds.size());

        if let Some(target) = selection {
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
                    if should_highlight(series_idx, bar_idx) {
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

    if is_horizontal {
        match position {
            Position::Above => (
                rect.x + rect.width + 4.0,
                rect.y + rect.height / 2.0,
                Horizontal::Left,
                Vertical::Center,
            ),
            Position::End => (
                rect.x + rect.width - 4.0,
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
                rect.x + 4.0,
                rect.y + rect.height / 2.0,
                Horizontal::Left,
                Vertical::Center,
            ),
        }
    } else {
        match position {
            Position::Above => (
                rect.x + rect.width / 2.0,
                rect.y - 4.0,
                Horizontal::Center,
                Vertical::Bottom,
            ),
            Position::End => (
                rect.x + rect.width / 2.0,
                rect.y + 4.0,
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
                rect.y + rect.height - 4.0,
                Horizontal::Center,
                Vertical::Bottom,
            ),
        }
    }
}

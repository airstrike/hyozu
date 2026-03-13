use super::Plane;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Rectangle, Size};
use crate::data::Datum;
use crate::mark::bar::label::Position;
use crate::widget::canvas::{Frame, Path, Text as CanvasText};
use crate::widget::renderer::geometry;
/// State for Bars - stores positioned bar rectangles
pub struct State {
    /// Pixel rectangles for each series, outer vec is series, inner vec is bars
    pub series_rects: Vec<Vec<Rectangle>>,
    /// Pixel rectangles for each label, outer vec is series, inner vec is bars
    pub label_rects: Vec<Vec<Option<Rectangle>>>,
}

/// A Bars series that renders vertical bar charts.
///
/// Like Guide and Title, this is widget-like but doesn't implement Widget.
pub struct Bars<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::bar::Bars,
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
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Bars state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
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

                let label_size = label_config.size.map(|p| p.0).unwrap_or(12.0);

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

        // Resolve all bar colors: per-point override → series color → palette
        let mut all_bar_colors: Vec<Vec<crate::core::Color>> = Vec::new();

        // Draw each series
        for (series_idx, (series, rects)) in self.data.series.iter().zip(state.series_rects.iter()).enumerate() {
            // Determine base color for this series
            let base_color = if let Some(series_color) = series.color {
                series_color.resolve(background, text_pair, None)
            } else {
                palette
                    .get(color_offset + series_idx)
                    .resolve(background, text_pair, None)
            };

            // Resolve per-bar colors
            let bar_colors: Vec<crate::core::Color> = rects
                .iter()
                .enumerate()
                .map(|(i, _)| {
                    if let Some(pc) = series.point_color(i) {
                        pc.resolve(background, text_pair, None)
                    } else {
                        base_color
                    }
                })
                .collect();

            // Draw bars: batch if all same color, otherwise draw individually
            if series.has_point_colors() {
                for (rect, &color) in rects.iter().zip(bar_colors.iter()) {
                    let path = Path::new(|builder| {
                        builder.rectangle(
                            crate::core::Point::new(rect.x, rect.y),
                            crate::core::Size::new(rect.width, rect.height),
                        );
                    });
                    bar_frame.fill(&path, color);
                }
            } else {
                let path = Path::new(|builder| {
                    for rect in rects {
                        builder.rectangle(
                            crate::core::Point::new(rect.x, rect.y),
                            crate::core::Size::new(rect.width, rect.height),
                        );
                    }
                });
                bar_frame.fill(&path, base_color);
            }

            all_bar_colors.push(bar_colors);

            // Draw labels for this series if configured
            if let Some(label_config) = &series.label {
                let label_size = label_config.size.map(|p| p.0).unwrap_or(12.0);
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
                    let effective_weight = point_label.and_then(|l| l.weight()).or(label_config.weight);
                    let effective_style = point_label.and_then(|l| l.style()).or(label_config.style);
                    let effective_fill = point_label.and_then(|l| l.fill().copied()).or(label_config.fill);

                    // Draw fill background if specified
                    if let Some(fill_color_spec) = effective_fill
                        && let Some(Some(lr)) = state.label_rects.get(series_idx).and_then(|rects| rects.get(bar_idx))
                    {
                        let fill_resolved = fill_color_spec.resolve(background, text_pair, None);
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
                                label_color_spec.resolve(other_color, text_pair, Some(background))
                            } else {
                                label_color_spec.resolve(background, text_pair, None)
                            }
                        }
                        _ => label_color_spec.resolve(this_bar_color, text_pair, Some(background)),
                    };

                    // Build font with weight/style overrides
                    let mut font = theme.font();
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

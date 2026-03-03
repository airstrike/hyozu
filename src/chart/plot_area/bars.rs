use super::Plane;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Rectangle, Size};
use crate::data::Datum;
use crate::mark::bar::label::Position;
/// State for Bars - stores positioned bar rectangles
pub struct State {
    /// Pixel rectangles for each series, outer vec is series, inner vec is bars
    pub series_rects: Vec<Vec<Rectangle>>,
}

/// A Bars series that renders vertical bar charts.
///
/// Like Guide and Title, this is widget-like but doesn't implement Widget.
pub struct Bars<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer,
{
    pub(super) data: &'a crate::bar::Bars,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Bars<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer,
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
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Bars state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the bars - calculates bar positions and sizes
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        plane: &Plane,
    ) -> Node {
        use crate::mark::bar::Layout;

        let state = tree.state.downcast_mut::<State>();

        if self.data.series.is_empty() {
            state.series_rects.clear();
            return Node::new(Size::ZERO);
        }

        // Determine number of bins (x-positions) - use max points across all series
        let num_bins = self
            .data
            .series
            .iter()
            .map(|s| s.points.len())
            .max()
            .unwrap_or(0);

        if num_bins == 0 {
            state.series_rects.clear();
            return Node::new(Size::ZERO);
        }

        let bar_length = self.data.size.get();
        let spacing_prop = self.data.spacing.get();
        let num_series = self.data.series.len();

        // Calculate zero line position
        let zero_y = plane.to_pixel(Datum::ORIGIN).y;

        // Layout based on strategy
        match self.data.layout {
            Layout::Grouped => {
                // Total width for all bins (categories)
                let total_width = plane.bounds.width;
                let bin_width = total_width / num_bins as f32;

                // Within each bin, calculate bar width based on:
                // - bar_length: proportion of bin width to use for bars
                // - spacing: spacing between bars as proportion of bar width
                // Total bar units: N bars + spacing * (N - 1) gaps
                let total_bar_units = num_series as f32
                    + spacing_prop * (num_series as f32 - 1.0);
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
                                let bar_center_x =
                                    plane.to_pixel(Datum::x(bin_idx as f64)).x;
                                let total_group_width = bin_width * bar_length;
                                let group_start =
                                    bar_center_x - total_group_width / 2.0;
                                let x = group_start
                                    + series_idx as f32
                                        * (bar_width + spacing_px);

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
                                let bar_center_x =
                                    plane.to_pixel(Datum::x(bin_idx as f64)).x;
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
                                let bar_center_x =
                                    plane.to_pixel(Datum::x(bin_idx as f64)).x;
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

        // Bars take no space - they're rendered within the plane
        Node::new(Size::ZERO)
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
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let palette = theme.data_colors();

        // Get the layout bounds to offset the bars to their actual screen position
        let layout_bounds = _layout.bounds();

        // Draw each series
        for (series_idx, (series, rects)) in self
            .data
            .series
            .iter()
            .zip(state.series_rects.iter())
            .enumerate()
        {
            // Determine color for this series
            let color = if let Some(series_color) = series.color {
                // Use series-specific color
                series_color.resolve(background, text_pair, None)
            } else {
                // Use color from palette based on series index
                palette
                    .get(series_idx % palette.len())
                    .map(|c| c.resolve(background, text_pair, None))
                    .unwrap_or(background)
            };

            // Draw each bar in this series
            for rect in rects {
                renderer.fill_quad(
                    crate::core::renderer::Quad {
                        bounds: Rectangle {
                            x: layout_bounds.x + rect.x,
                            y: layout_bounds.y + rect.y,
                            width: rect.width,
                            height: rect.height,
                        },
                        ..Default::default()
                    },
                    color,
                );
            }

            // Draw labels for this series if configured
            if let Some(label_config) = &series.label {
                let label_size = label_config.size.map(|p| p.0).unwrap_or(12.0);

                for (rect, point) in rects.iter().zip(series.points.iter()) {
                    // Format the label text
                    let label_text = (label_config.format)(point.y);

                    // Calculate label position based on configuration
                    let (label_x, label_y, align_x, align_y) =
                        match label_config.position {
                            Position::Above => {
                                // Center horizontally, position above the bar
                                (
                                    layout_bounds.x + rect.x + rect.width / 2.0,
                                    layout_bounds.y + rect.y - 4.0,
                                    crate::core::alignment::Horizontal::Center
                                        .into(),
                                    crate::core::alignment::Vertical::Bottom,
                                )
                            }
                            Position::End => {
                                // Center horizontally, position at the end (top) of the bar
                                (
                                    layout_bounds.x + rect.x + rect.width / 2.0,
                                    layout_bounds.y + rect.y + 4.0,
                                    crate::core::alignment::Horizontal::Center
                                        .into(),
                                    crate::core::alignment::Vertical::Top,
                                )
                            }
                            Position::Center => {
                                // Center both horizontally and vertically
                                (
                                    layout_bounds.x + rect.x + rect.width / 2.0,
                                    layout_bounds.y
                                        + rect.y
                                        + rect.height / 2.0,
                                    crate::core::alignment::Horizontal::Center
                                        .into(),
                                    crate::core::alignment::Vertical::Center,
                                )
                            }
                            Position::Base => {
                                // Center horizontally, position at the base (bottom) of the bar
                                (
                                    layout_bounds.x + rect.x + rect.width / 2.0,
                                    layout_bounds.y + rect.y + rect.height
                                        - 4.0,
                                    crate::core::alignment::Horizontal::Center
                                        .into(),
                                    crate::core::alignment::Vertical::Bottom,
                                )
                            }
                        };

                    // Resolve label color based on position
                    // Default to CONTRAST (adaptive color) if not specified
                    let label_color_spec = label_config
                        .color
                        .unwrap_or(crate::color::Color::CONTRAST);

                    let label_color = match label_config.position {
                        Position::Above => {
                            // Check if label position is inside any other bar's rectangle
                            let label_point =
                                crate::core::Point::new(label_x, label_y);

                            // Find which bar (if any) contains this label position
                            let containing_bar = state
                                .series_rects
                                .iter()
                                .enumerate()
                                .find_map(|(other_series_idx, other_rects)| {
                                    other_rects
                                        .iter()
                                        .find(|other_rect| {
                                            let screen_rect = Rectangle {
                                                x: layout_bounds.x
                                                    + other_rect.x,
                                                y: layout_bounds.y
                                                    + other_rect.y,
                                                width: other_rect.width,
                                                height: other_rect.height,
                                            };
                                            screen_rect.contains(label_point)
                                        })
                                        .map(|_| other_series_idx)
                                });

                            if let Some(other_series_idx) = containing_bar {
                                // Label is inside another bar - resolve against that bar's color
                                let other_series =
                                    &self.data.series[other_series_idx];
                                let other_color = if let Some(series_color) =
                                    other_series.color
                                {
                                    series_color
                                        .resolve(background, text_pair, None)
                                } else {
                                    palette
                                        .get(other_series_idx % palette.len())
                                        .map(|c| {
                                            c.resolve(
                                                background, text_pair, None,
                                            )
                                        })
                                        .unwrap_or(background)
                                };
                                label_color_spec.resolve(
                                    other_color,
                                    text_pair,
                                    Some(background),
                                )
                            } else {
                                // Label is outside all bars - resolve against chart background
                                label_color_spec
                                    .resolve(background, text_pair, None)
                            }
                        }
                        _ => {
                            // Inside label (End, Center, Base): resolve against bar's visual color
                            // Use chart background as fallback if pair options have poor contrast
                            label_color_spec.resolve(
                                color,
                                text_pair,
                                Some(background),
                            )
                        }
                    };

                    renderer.fill_text(
                        crate::core::text::Text {
                            content: label_text,
                            bounds: crate::core::Size::new(1000.0, 1000.0),
                            size: label_size.into(),
                            font: renderer.default_font(),
                            align_x,
                            align_y,
                            line_height: crate::core::text::LineHeight::default(
                            ),
                            shaping: crate::core::text::Shaping::Basic,
                            wrapping: crate::core::text::Wrapping::None,
                            ellipsis: crate::core::text::Ellipsis::default(),
                            hint_factor: renderer.scale_factor(),
                        },
                        crate::core::Point::new(label_x, label_y),
                        label_color,
                        *_viewport,
                    );
                }
            }
        }
    }
}

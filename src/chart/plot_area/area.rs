use super::Plane;
use super::line::{alignment_for_position, clamp_rect_to_bounds, compute_label_rect, find_best_label_placement};
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size};
use crate::data::Datum;
use crate::line::label::{Position, Show};
use crate::widget::canvas::gradient::Linear;
use crate::widget::canvas::{Fill, Frame, Path, Stroke, Text as CanvasText};
use crate::widget::renderer::geometry;

pub struct State {
    /// Upper line pixel points per series
    pub series_points: Vec<Vec<Point>>,
    /// Baseline pixel points per series (for fill polygon)
    pub series_baselines: Vec<Vec<Point>>,
    /// Label texts per series
    pub series_label_texts: Vec<Vec<String>>,
    /// Resolved label positions per series
    pub series_label_positions: Vec<Vec<Position>>,
    /// Pixel rectangles for each label per series
    pub series_label_rects: Vec<Vec<Rectangle>>,
}

pub struct Area<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub(crate) data: &'a crate::mark::area::Area,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Area<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub fn new(data: &'a crate::mark::area::Area) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                series_points: Vec::new(),
                series_baselines: Vec::new(),
                series_label_texts: Vec::new(),
                series_label_positions: Vec::new(),
                series_label_rects: Vec::new(),
            }),
            children: Vec::new(),
        }
    }

    pub(super) fn diff(&self, _tree: &mut Tree) {}

    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();

        let zero_y = plane.to_pixel(Datum::ORIGIN).y;

        match self.data.layout {
            crate::mark::area::Layout::Overlaid => {
                state.series_points = self
                    .data
                    .series
                    .iter()
                    .map(|s| s.points.iter().map(|p| plane.to_pixel(*p)).collect())
                    .collect();

                state.series_baselines = self
                    .data
                    .series
                    .iter()
                    .map(|s| {
                        s.points
                            .iter()
                            .map(|p| {
                                let px = plane.to_pixel(*p);
                                Point::new(px.x, zero_y)
                            })
                            .collect()
                    })
                    .collect();
            }
            crate::mark::area::Layout::Stacked => {
                let num_points = self.data.series.iter().map(|s| s.points.len()).max().unwrap_or(0);
                let mut cumulative_y = vec![0.0_f64; num_points];

                state.series_points.clear();
                state.series_baselines.clear();

                for series in &self.data.series {
                    let mut upper = Vec::new();
                    let mut baseline = Vec::new();

                    for (i, point) in series.points.iter().enumerate() {
                        if i < num_points {
                            let base_y = cumulative_y[i];
                            let new_y = base_y + point.y;

                            let base_pixel = plane.to_pixel(Datum::new(point.x, base_y));
                            let upper_pixel = plane.to_pixel(Datum::new(point.x, new_y));

                            baseline.push(base_pixel);
                            upper.push(upper_pixel);

                            cumulative_y[i] = new_y;
                        }
                    }

                    state.series_baselines.push(baseline);
                    state.series_points.push(upper);
                }
            }
        }

        // Build label info per series. Labels avoid:
        //   - every series' upper envelope (so a later series' label does
        //     not overlap an earlier series' line)
        //   - axis obstacles from the plane
        //   - labels already placed in this or earlier series
        state.series_label_texts.clear();
        state.series_label_positions.clear();
        state.series_label_rects.clear();
        state.series_label_texts.resize_with(self.data.series.len(), Vec::new);
        state
            .series_label_positions
            .resize_with(self.data.series.len(), Vec::new);
        state.series_label_rects.resize_with(self.data.series.len(), Vec::new);

        let all_segments: Vec<(Point, Point)> = state
            .series_points
            .iter()
            .flat_map(|pts| pts.windows(2).map(|w| (w[0], w[1])))
            .collect();

        let mut placed_rects: Vec<Rectangle> = Vec::new();

        for (series_idx, series) in self.data.series.iter().enumerate() {
            let Some(label_config) = &series.label else {
                continue;
            };

            let upper = &state.series_points[series_idx];
            let num_points = upper.len();
            if num_points == 0 {
                continue;
            }

            let label_size = label_config.text.size.map(|p| p.0).unwrap_or(12.0);

            let minmax_indices: Vec<usize> = match label_config.show {
                Show::MinMaxFirst | Show::MinMaxAll | Show::MinMaxLast => {
                    let (min_val, max_val) = series
                        .points
                        .iter()
                        .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), p| {
                            (min.min(p.y), max.max(p.y))
                        });

                    match label_config.show {
                        Show::MinMaxFirst => {
                            let min_idx = series.points.iter().position(|p| p.y == min_val);
                            let max_idx = series.points.iter().position(|p| p.y == max_val);
                            [min_idx, max_idx].into_iter().flatten().collect()
                        }
                        Show::MinMaxLast => {
                            let min_idx = series.points.iter().rposition(|p| p.y == min_val);
                            let max_idx = series.points.iter().rposition(|p| p.y == max_val);
                            [min_idx, max_idx].into_iter().flatten().collect()
                        }
                        Show::MinMaxAll => series
                            .points
                            .iter()
                            .enumerate()
                            .filter(|(_, p)| p.y == min_val || p.y == max_val)
                            .map(|(i, _)| i)
                            .collect(),
                        _ => vec![],
                    }
                }
                _ => vec![],
            };

            for (idx, pixel_point) in upper.iter().enumerate() {
                let should_show = match label_config.show {
                    Show::Any => true,
                    Show::FirstOnly => idx == 0,
                    Show::LastOnly => idx == num_points - 1,
                    Show::FirstAndLast => idx == 0 || idx == num_points - 1,
                    Show::MinMaxFirst | Show::MinMaxAll | Show::MinMaxLast => minmax_indices.contains(&idx),
                };

                if !should_show {
                    continue;
                }

                // For stacked layout, the displayed value should be the
                // original (unstacked) series value, not the cumulative.
                let Some(data_point) = series.points.get(idx) else {
                    continue;
                };
                let label_text = (label_config.format)(data_point.y);

                let char_width = label_size * 0.6;
                let label_width = label_text.len() as f32 * char_width;
                let label_height = label_size * 1.2;

                let (resolved_position, label_rect) = find_best_label_placement(
                    *pixel_point,
                    label_width,
                    label_height,
                    &all_segments,
                    &placed_rects,
                    &plane.obstacles,
                    Some(plane.bounds),
                );

                // If the user requested a fixed position, prefer it but still
                // let the placement fall back when the ideal slot is taken.
                let resolved_position = match label_config.position {
                    Position::Auto => resolved_position,
                    other => other,
                };
                let label_rect = if label_config.position == Position::Auto {
                    label_rect
                } else {
                    let preferred = compute_label_rect(*pixel_point, label_width, label_height, label_config.position);
                    // Only honor the fixed position if it doesn't collide.
                    if placed_rects
                        .iter()
                        .any(|r| crate::geometry::intersect::rects(*r, preferred))
                    {
                        label_rect
                    } else {
                        preferred
                    }
                };
                // Guarantee the label stays inside the plot area even at the
                // first/last data point, where the centered ideal rect would
                // otherwise bleed past the edge and get clipped.
                let label_rect = clamp_rect_to_bounds(label_rect, plane.bounds);

                placed_rects.push(label_rect);
                state.series_label_texts[series_idx].push(label_text);
                state.series_label_positions[series_idx].push(resolved_position);
                state.series_label_rects[series_idx].push(label_rect);
            }
        }

        Node::new(Size::ZERO)
    }

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
        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.palette_seed();
        let layout_bounds = layout.bounds();

        let mut fill_frame = Frame::new(renderer, layout_bounds.size());
        let mut stroke_frame = Frame::new(renderer, layout_bounds.size());

        for (series_idx, series) in self.data.series.iter().enumerate() {
            let upper = &state.series_points[series_idx];
            let baseline = &state.series_baselines[series_idx];

            if upper.len() < 2 {
                continue;
            }

            let base_color = if let Some(c) = series.color {
                c.resolve(background, text_pair, &seed, None)
            } else {
                palette
                    .get(color_offset + series_idx)
                    .resolve(background, text_pair, &seed, None)
            };

            let fill_color = crate::core::Color {
                a: base_color.a * series.opacity,
                ..base_color
            };

            let fill_path = Path::new(|builder| {
                if let Some(first) = upper.first() {
                    builder.move_to(*first);
                }
                for p in upper.iter().skip(1) {
                    builder.line_to(*p);
                }
                for p in baseline.iter().rev() {
                    builder.line_to(*p);
                }
                builder.close();
            });

            if self.data.gradient {
                // Vertical linear gradient: series color at the top of the
                // upper envelope, fully transparent at the baseline. Uses
                // absolute start/end points in frame-local pixel coordinates.
                let top_y = upper.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
                let bottom_y = baseline.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);

                let gradient = Linear::new(Point::new(0.0, top_y), Point::new(0.0, bottom_y))
                    .add_stop(0.0, fill_color)
                    .add_stop(1.0, crate::core::Color { a: 0.0, ..base_color });

                fill_frame.fill(&fill_path, Fill::from(gradient));
            } else {
                fill_frame.fill(&fill_path, fill_color);
            }

            if let Some(stroke_width) = series.stroke {
                let stroke_path = Path::new(|builder| {
                    if let Some(first) = upper.first() {
                        builder.move_to(*first);
                    }
                    for p in upper.iter().skip(1) {
                        builder.line_to(*p);
                    }
                });
                stroke_frame.stroke(
                    &stroke_path,
                    Stroke::default().with_width(stroke_width).with_color(base_color),
                );
            }
        }

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);

        let fill_geometry = fill_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(fill_geometry);
        });

        let stroke_geometry = stroke_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(stroke_geometry);
        });

        // Draw data labels per series. All series share one frame so that
        // we only emit a single `draw_geometry` for labels regardless of
        // how many series have labels configured.
        let mut label_frame = Frame::new(renderer, layout_bounds.size());
        let mut any_label = false;

        for (series_idx, series) in self.data.series.iter().enumerate() {
            let Some(label_config) = &series.label else {
                continue;
            };

            let label_size = label_config.text.size.map(|p| p.0).unwrap_or(12.0);

            let base_color = if let Some(c) = series.color {
                c.resolve(background, text_pair, &seed, None)
            } else {
                palette
                    .get(color_offset + series_idx)
                    .resolve(background, text_pair, &seed, None)
            };

            let label_color = if let Some(label_color_spec) = label_config.color {
                label_color_spec.resolve(background, text_pair, &seed, None)
            } else {
                base_color
            };

            let label_font = label_config
                .text
                .resolved_font(theme.data_label_text().resolved_font(theme.font()));

            let label_fill_color = label_config
                .fill
                .map(|spec| spec.resolve(background, text_pair, &seed, None));

            let texts = &state.series_label_texts[series_idx];
            let positions = &state.series_label_positions[series_idx];
            let rects = &state.series_label_rects[series_idx];

            for ((label_rect, label_text), resolved_pos) in rects.iter().zip(texts.iter()).zip(positions.iter()) {
                any_label = true;

                if let Some(fill_color) = label_fill_color {
                    let fill_path = Path::new(|b| {
                        b.rectangle(
                            Point::new(label_rect.x, label_rect.y),
                            crate::core::Size::new(label_rect.width, label_rect.height),
                        );
                    });
                    label_frame.fill(&fill_path, fill_color);
                }

                let (align_x, align_y) = alignment_for_position(*resolved_pos);

                let (anchor_x, anchor_y) = match resolved_pos {
                    Position::Auto | Position::Above => {
                        (label_rect.x + label_rect.width / 2.0, label_rect.y + label_rect.height)
                    }
                    Position::Below => (label_rect.x + label_rect.width / 2.0, label_rect.y),
                    Position::Left => (label_rect.x + label_rect.width, label_rect.y + label_rect.height / 2.0),
                    Position::Right => (label_rect.x, label_rect.y + label_rect.height / 2.0),
                };

                label_frame.fill_text(CanvasText {
                    content: label_text.clone(),
                    position: Point::new(anchor_x, anchor_y),
                    color: label_color,
                    size: label_size.into(),
                    font: label_font,
                    align_x: align_x.into(),
                    align_y,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }

        if any_label {
            let label_geometry = label_frame.into_geometry();
            renderer.with_translation(translation, |renderer| {
                renderer.draw_geometry(label_geometry);
            });
        }
    }
}

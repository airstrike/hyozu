use super::Plane;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size};
use crate::data::Datum;
use crate::widget::canvas::{Frame, Path, Stroke};
use crate::widget::renderer::geometry;

pub struct State {
    /// Upper line pixel points per series
    pub series_points: Vec<Vec<Point>>,
    /// Baseline pixel points per series (for fill polygon)
    pub series_baselines: Vec<Vec<Point>>,
}

pub struct Area<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::area::Area,
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
                c.resolve(background, text_pair, None)
            } else {
                palette
                    .get(color_offset + series_idx)
                    .resolve(background, text_pair, None)
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

            fill_frame.fill(&fill_path, fill_color);

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
    }
}

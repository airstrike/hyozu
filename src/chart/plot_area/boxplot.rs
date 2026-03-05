use super::Plane;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size};
use crate::data::Datum;
use crate::mark::boxplot::Direction;
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Layout data for a single box plot entry.
pub struct EntryLayout {
    pub center_x: f32,
    pub min_y: f32,
    pub q1_y: f32,
    pub median_y: f32,
    pub q3_y: f32,
    pub max_y: f32,
    pub box_width: f32,
    pub outlier_ys: Vec<f32>,
}

/// State for BoxPlot — stores pre-calculated layout positions.
pub struct State {
    /// Pixel rectangles for Q1-Q3 boxes (for hit-testing)
    pub box_rects: Vec<Rectangle>,
    /// Full layout data for each entry
    pub entries_layout: Vec<EntryLayout>,
}

/// A BoxPlot series that renders box-and-whisker charts.
pub struct BoxPlot<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::boxplot::BoxPlot,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> BoxPlot<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new BoxPlot borrowing data
    pub fn new(data: &'a crate::mark::boxplot::BoxPlot) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this BoxPlot
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                box_rects: Vec::new(),
                entries_layout: Vec::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current BoxPlot state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the box plot — calculate positions and sizes
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        plane: &Plane,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();

        if self.data.entries.is_empty() {
            state.box_rects.clear();
            state.entries_layout.clear();
            return Node::new(Size::ZERO);
        }

        let num_entries = self.data.entries.len();
        state.box_rects.clear();
        state.entries_layout.clear();

        match self.data.direction {
            Direction::Vertical => {
                let total_width = plane.bounds.width;
                let bin_width = total_width / num_entries as f32;
                let box_width = bin_width * self.data.width;

                for (i, entry) in self.data.entries.iter().enumerate() {
                    let center_x = plane.to_pixel(Datum::x(i as f64)).x;

                    let q1_y = plane.to_pixel(Datum::new(i as f64, entry.q1)).y;
                    let q3_y = plane.to_pixel(Datum::new(i as f64, entry.q3)).y;
                    let min_y =
                        plane.to_pixel(Datum::new(i as f64, entry.min)).y;
                    let max_y =
                        plane.to_pixel(Datum::new(i as f64, entry.max)).y;
                    let median_y =
                        plane.to_pixel(Datum::new(i as f64, entry.median)).y;

                    let outlier_ys: Vec<f32> = entry
                        .outliers
                        .iter()
                        .map(|&o| plane.to_pixel(Datum::new(i as f64, o)).y)
                        .collect();

                    // q3 is higher value -> lower y pixel
                    let rect_y = q3_y.min(q1_y);
                    let rect_height = (q1_y - q3_y).abs();

                    state.box_rects.push(Rectangle {
                        x: center_x - box_width / 2.0,
                        y: rect_y,
                        width: box_width,
                        height: rect_height,
                    });

                    state.entries_layout.push(EntryLayout {
                        center_x,
                        min_y,
                        q1_y,
                        median_y,
                        q3_y,
                        max_y,
                        box_width,
                        outlier_ys,
                    });
                }
            }
            Direction::Horizontal => {
                let total_height = plane.bounds.height;
                let bin_height = total_height / num_entries as f32;
                let box_height = bin_height * self.data.width;

                for (i, entry) in self.data.entries.iter().enumerate() {
                    let center_y = plane.to_pixel(Datum::y(i as f64)).y;

                    let q1_x = plane.to_pixel(Datum::new(entry.q1, i as f64)).x;
                    let q3_x = plane.to_pixel(Datum::new(entry.q3, i as f64)).x;
                    let min_x =
                        plane.to_pixel(Datum::new(entry.min, i as f64)).x;
                    let max_x =
                        plane.to_pixel(Datum::new(entry.max, i as f64)).x;
                    let median_x =
                        plane.to_pixel(Datum::new(entry.median, i as f64)).x;

                    let outlier_xs: Vec<f32> = entry
                        .outliers
                        .iter()
                        .map(|&o| plane.to_pixel(Datum::new(o, i as f64)).x)
                        .collect();

                    let rect_x = q1_x.min(q3_x);
                    let rect_width = (q3_x - q1_x).abs();

                    state.box_rects.push(Rectangle {
                        x: rect_x,
                        y: center_y - box_height / 2.0,
                        width: rect_width,
                        height: box_height,
                    });

                    // Reuse EntryLayout — for horizontal, we store x values in the y fields
                    // center_x -> center_y, min_y -> min_x, etc.
                    state.entries_layout.push(EntryLayout {
                        center_x: center_y,
                        min_y: min_x,
                        q1_y: q1_x,
                        median_y: median_x,
                        q3_y: q3_x,
                        max_y: max_x,
                        box_width: box_height,
                        outlier_ys: outlier_xs,
                    });
                }
            }
        }

        Node::new(Size::ZERO)
    }

    /// Draws the box plot using pre-calculated layout data
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

        if state.entries_layout.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        match self.data.direction {
            Direction::Vertical => {
                self.draw_vertical(
                    state,
                    &mut frame,
                    color_offset,
                    palette,
                    background,
                    text_pair,
                );
            }
            Direction::Horizontal => {
                self.draw_horizontal(
                    state,
                    &mut frame,
                    color_offset,
                    palette,
                    background,
                    text_pair,
                );
            }
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(
            crate::core::Vector::new(layout_bounds.x, layout_bounds.y),
            |renderer| {
                renderer.draw_geometry(geometry);
            },
        );
    }

    fn draw_vertical(
        &self,
        state: &State,
        frame: &mut Frame<Renderer>,
        color_offset: usize,
        palette: &crate::palette::Resolved,
        background: crate::core::Color,
        text_pair: crate::color::Pair,
    ) {
        for (i, (entry, el)) in self
            .data
            .entries
            .iter()
            .zip(state.entries_layout.iter())
            .enumerate()
        {
            let rect = &state.box_rects[i];

            // Resolve color
            let base_color = if let Some(entry_color) = entry.color {
                entry_color.resolve(background, text_pair, None)
            } else {
                palette
                    .get(color_offset + i)
                    .resolve(background, text_pair, None)
            };

            let fill_color = crate::core::Color {
                a: base_color.a * 0.3,
                ..base_color
            };

            let stroke_color = base_color;

            // Draw box (filled rect Q1-Q3)
            let box_path = Path::new(|b| {
                b.rectangle(
                    Point::new(rect.x, rect.y),
                    Size::new(rect.width, rect.height),
                );
            });
            frame.fill(&box_path, fill_color);
            frame.stroke(
                &box_path,
                Stroke::default().with_width(1.5).with_color(stroke_color),
            );

            // Median line
            let median_path = Path::new(|b| {
                b.move_to(Point::new(rect.x, el.median_y));
                b.line_to(Point::new(rect.x + rect.width, el.median_y));
            });
            frame.stroke(
                &median_path,
                Stroke::default().with_width(2.0).with_color(stroke_color),
            );

            // Whiskers (vertical lines from box to min/max)
            let whisker_path = Path::new(|b| {
                // Upper whisker: Q3 to max
                b.move_to(Point::new(el.center_x, el.q3_y));
                b.line_to(Point::new(el.center_x, el.max_y));
                // Lower whisker: Q1 to min
                b.move_to(Point::new(el.center_x, el.q1_y));
                b.line_to(Point::new(el.center_x, el.min_y));
            });
            frame.stroke(
                &whisker_path,
                Stroke::default().with_width(1.0).with_color(stroke_color),
            );

            // Whisker caps
            let cap_width = el.box_width * 0.4;
            let cap_path = Path::new(|b| {
                // Max cap
                b.move_to(Point::new(el.center_x - cap_width / 2.0, el.max_y));
                b.line_to(Point::new(el.center_x + cap_width / 2.0, el.max_y));
                // Min cap
                b.move_to(Point::new(el.center_x - cap_width / 2.0, el.min_y));
                b.line_to(Point::new(el.center_x + cap_width / 2.0, el.min_y));
            });
            frame.stroke(
                &cap_path,
                Stroke::default().with_width(1.0).with_color(stroke_color),
            );

            // Outliers
            for &oy in &el.outlier_ys {
                let outlier_path = Path::new(|b| {
                    b.circle(Point::new(el.center_x, oy), 3.0);
                });
                frame.fill(&outlier_path, stroke_color);
            }
        }
    }

    fn draw_horizontal(
        &self,
        state: &State,
        frame: &mut Frame<Renderer>,
        color_offset: usize,
        palette: &crate::palette::Resolved,
        background: crate::core::Color,
        text_pair: crate::color::Pair,
    ) {
        for (i, (entry, el)) in self
            .data
            .entries
            .iter()
            .zip(state.entries_layout.iter())
            .enumerate()
        {
            let rect = &state.box_rects[i];

            // Resolve color
            let base_color = if let Some(entry_color) = entry.color {
                entry_color.resolve(background, text_pair, None)
            } else {
                palette
                    .get(color_offset + i)
                    .resolve(background, text_pair, None)
            };

            let fill_color = crate::core::Color {
                a: base_color.a * 0.3,
                ..base_color
            };

            let stroke_color = base_color;

            // Draw box (filled rect Q1-Q3)
            let box_path = Path::new(|b| {
                b.rectangle(
                    Point::new(rect.x, rect.y),
                    Size::new(rect.width, rect.height),
                );
            });
            frame.fill(&box_path, fill_color);
            frame.stroke(
                &box_path,
                Stroke::default().with_width(1.5).with_color(stroke_color),
            );

            // For horizontal: el.center_x is actually center_y,
            // el.min_y is min_x, etc.
            let center_y = el.center_x;
            let min_x = el.min_y;
            let q1_x = el.q1_y;
            let median_x = el.median_y;
            let q3_x = el.q3_y;
            let max_x = el.max_y;
            let box_height = el.box_width;

            // Median line (vertical in horizontal mode)
            let median_path = Path::new(|b| {
                b.move_to(Point::new(median_x, rect.y));
                b.line_to(Point::new(median_x, rect.y + rect.height));
            });
            frame.stroke(
                &median_path,
                Stroke::default().with_width(2.0).with_color(stroke_color),
            );

            // Whiskers (horizontal lines from box to min/max)
            let whisker_path = Path::new(|b| {
                // Left whisker: Q1 to min
                b.move_to(Point::new(q1_x, center_y));
                b.line_to(Point::new(min_x, center_y));
                // Right whisker: Q3 to max
                b.move_to(Point::new(q3_x, center_y));
                b.line_to(Point::new(max_x, center_y));
            });
            frame.stroke(
                &whisker_path,
                Stroke::default().with_width(1.0).with_color(stroke_color),
            );

            // Whisker caps (vertical in horizontal mode)
            let cap_height = box_height * 0.4;
            let cap_path = Path::new(|b| {
                // Min cap
                b.move_to(Point::new(min_x, center_y - cap_height / 2.0));
                b.line_to(Point::new(min_x, center_y + cap_height / 2.0));
                // Max cap
                b.move_to(Point::new(max_x, center_y - cap_height / 2.0));
                b.line_to(Point::new(max_x, center_y + cap_height / 2.0));
            });
            frame.stroke(
                &cap_path,
                Stroke::default().with_width(1.0).with_color(stroke_color),
            );

            // Outliers
            for &ox in &el.outlier_ys {
                let outlier_path = Path::new(|b| {
                    b.circle(Point::new(ox, center_y), 3.0);
                });
                frame.fill(&outlier_path, stroke_color);
            }
        }
    }
}

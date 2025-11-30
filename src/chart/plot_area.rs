use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::Rectangle;
use crate::data::Datum;

use crate::core::text;
use crate::widget::renderer::geometry;

pub mod bars;
pub mod line;

pub use bars::Bars;
pub use line::Line;

/// The coordinate plane for transforming data coords to pixels.
#[derive(Debug, Clone)]
pub struct Plane {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub bounds: Rectangle,
    /// Obstacle rectangles that labels must avoid (axes, etc.)
    /// Coordinates are relative to plot area origin.
    pub obstacles: Vec<Rectangle>,
}

/// Axis layout dimensions passed from Scene to PlotArea
#[derive(Debug, Clone, Copy, Default)]
pub struct AxisLayout {
    pub left_width: f32,
    pub right_width: f32,
    pub top_height: f32,
    pub bottom_height: f32,
}

impl Plane {
    /// Transform a data point (f64) to pixel coordinates (f32) within the plane bounds.
    pub fn to_pixel(&self, datum: Datum) -> crate::core::Point {
        let x = if self.x_max > self.x_min {
            self.bounds.x
                + (((datum.x - self.x_min) / (self.x_max - self.x_min)) as f32)
                    * self.bounds.width
        } else {
            self.bounds.x + self.bounds.width / 2.0
        };

        let y = if self.y_max > self.y_min {
            self.bounds.y + self.bounds.height
                - (((datum.y - self.y_min) / (self.y_max - self.y_min)) as f32)
                    * self.bounds.height
        } else {
            self.bounds.y + self.bounds.height / 2.0
        };

        crate::core::Point::new(x, y)
    }
}

/// A series that can be rendered in the plot area
pub enum Series<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    Line(Line<'a, Message, Renderer>),
    Bars(Bars<'a, Message, Renderer>),
}

/// State for a PlotArea - stores the coordinate plane for rendering
pub struct State {
    /// The coordinate plane for transforming data to pixels
    pub plane: Option<Plane>,
}

/// A PlotArea renders the data series within the chart.
///
/// Like Guide and Title, this borrows data and is widget-like but doesn't
/// implement Widget. Keeps UI code separate from data viz.
/// It organizes and delegates to individual series (Line, Bars, etc.)
pub struct PlotArea<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    series: Vec<Series<'a, Message, Renderer>>,
}

impl<'a, Message, Renderer> PlotArea<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new PlotArea from mark data
    pub fn new(marks: &'a [crate::Mark]) -> Self {
        // Convert marks to series
        let series = marks
            .iter()
            .map(|mark| match mark {
                crate::Mark::Line(line) => Series::Line(Line::new(line)),
                crate::Mark::Bars(bars) => Series::Bars(Bars::new(bars)),
            })
            .collect();

        Self { series }
    }

    /// Returns the initial tree state for this PlotArea
    pub(super) fn state(&self) -> Tree {
        // Create children for each series
        let children = self
            .series
            .iter()
            .map(|s| match s {
                Series::Line(line) => line.state(),
                Series::Bars(bars) => bars.state(),
            })
            .collect();

        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State { plane: None }),
            children,
        }
    }

    /// Reconcile the tree with current PlotArea state
    pub(super) fn diff(&self, tree: &mut Tree) {
        // Diff each series
        tree.diff_children_custom(
            &self.series,
            |tree, series| match series {
                Series::Line(line) => line.diff(tree),
                Series::Bars(bars) => bars.diff(tree),
            },
            |series| match series {
                Series::Line(line) => line.state(),
                Series::Bars(bars) => bars.state(),
            },
        );
    }

    /// Compute data bounds from all series.
    fn compute_data_bounds(&self) -> (f64, f64, f64, f64) {
        use crate::mark::bar::Layout;
        use std::collections::HashMap;

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for series in &self.series {
            match series {
                Series::Bars(bars) => {
                    // For stacked layout, compute cumulative sums
                    if bars.data.layout == Layout::Stacked {
                        let mut sums: HashMap<i64, f64> = HashMap::new();

                        for bar_series in &bars.data.series {
                            for point in &bar_series.points {
                                let x_key = (point.x * 1000.0).round() as i64;
                                *sums.entry(x_key).or_insert(0.0) += point.y;

                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                            }
                        }

                        for sum in sums.values() {
                            y_min = y_min.min(*sum);
                            y_max = y_max.max(*sum);
                        }
                    } else {
                        // For grouped/overlaid, use individual values
                        for bar_series in &bars.data.series {
                            for point in &bar_series.points {
                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                                y_min = y_min.min(point.y);
                                y_max = y_max.max(point.y);
                            }
                        }
                    }
                }
                Series::Line(line) => {
                    for point in &line.data.points {
                        x_min = x_min.min(point.x);
                        x_max = x_max.max(point.x);
                        y_min = y_min.min(point.y);
                        y_max = y_max.max(point.y);
                    }
                }
            }
        }

        // Ensure we have valid bounds even for empty data
        if x_min.is_infinite() || x_max.is_infinite() {
            x_min = 0.0;
            x_max = 1.0;
        }
        if y_min.is_infinite() || y_max.is_infinite() {
            y_min = 0.0;
            y_max = 1.0;
        }

        // For bar charts, extend y to include zero
        for series in &self.series {
            if matches!(series, Series::Bars(_)) {
                y_min = y_min.min(0.0);
                break;
            }
        }

        (x_min, x_max, y_min, y_max)
    }

    /// Layout the plot area - creates plane and delegates to each series
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
        axis_bounds: Option<(f64, f64, f64, f64)>, // (x_min, x_max, y_min, y_max) from axes
        axis_layout: AxisLayout,                    // Physical dimensions of axes
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();
        let size = limits.max();

        // Use axis bounds if provided, otherwise compute from data
        let (x_min, x_max, y_min, y_max) =
            axis_bounds.unwrap_or_else(|| self.compute_data_bounds());

        // Compute obstacle rectangles from axis layout
        // These are relative to plot area origin (0,0 is top-left of plot)
        let mut obstacles = Vec::new();

        // Left axis obstacle (to the left of plot area)
        if axis_layout.left_width > 0.0 {
            obstacles.push(Rectangle {
                x: -axis_layout.left_width,
                y: 0.0,
                width: axis_layout.left_width,
                height: size.height,
            });
        }

        // Right axis obstacle (to the right of plot area)
        if axis_layout.right_width > 0.0 {
            obstacles.push(Rectangle {
                x: size.width,
                y: 0.0,
                width: axis_layout.right_width,
                height: size.height,
            });
        }

        // Top axis obstacle (above plot area)
        if axis_layout.top_height > 0.0 {
            obstacles.push(Rectangle {
                x: 0.0,
                y: -axis_layout.top_height,
                width: size.width,
                height: axis_layout.top_height,
            });
        }

        // Bottom axis obstacle (below plot area)
        if axis_layout.bottom_height > 0.0 {
            obstacles.push(Rectangle {
                x: 0.0,
                y: size.height,
                width: size.width,
                height: axis_layout.bottom_height,
            });
        }

        let plane = Plane {
            x_min,
            x_max,
            y_min,
            y_max,
            bounds: Rectangle {
                x: 0.0,
                y: 0.0,
                width: size.width,
                height: size.height,
            },
            obstacles,
        };

        // Layout each series with the plane
        for (i, series) in self.series.iter().enumerate() {
            let series_tree = &mut tree.children[i];
            match series {
                Series::Line(line) => {
                    line.layout(series_tree, renderer, limits, &plane);
                }
                Series::Bars(bars) => {
                    bars.layout(series_tree, renderer, limits, &plane);
                }
            }
        }

        // Store the plane for draw()
        state.plane = Some(plane);

        Node::new(size)
    }

    /// Draws the plot area by delegating to each series
    #[allow(clippy::too_many_arguments)]
    pub fn draw<D>(
        &self,
        tree: &crate::core::widget::Tree,
        renderer: &mut Renderer,
        design: &D,
        style: &crate::core::renderer::Style,
        layout: crate::core::Layout<'_>,
        cursor: crate::core::mouse::Cursor,
        viewport: &crate::core::Rectangle,
    ) where
        D: crate::design::Design + ?Sized,
    {
        // Draw each series
        for (i, series) in self.series.iter().enumerate() {
            let series_tree = &tree.children[i];
            match series {
                Series::Line(line) => {
                    line.draw(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                        i,
                    );
                }
                Series::Bars(bars) => {
                    bars.draw(
                        series_tree,
                        renderer,
                        design,
                        style,
                        layout,
                        cursor,
                        viewport,
                    );
                }
            }
        }
    }
}

use super::Plane;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size};
use crate::data::Datum;
use crate::widget::canvas::{Frame, Path, Text as CanvasText};
use crate::widget::renderer::geometry;

/// Interpolate between color stops in linear RGB at parameter `t` in 0..1.
fn interpolate_stops(stops: &[crate::core::Color], t: f32) -> crate::core::Color {
    if stops.is_empty() {
        return crate::core::Color::BLACK;
    }
    if stops.len() == 1 || t <= 0.0 {
        return stops[0];
    }
    if t >= 1.0 {
        return stops[stops.len() - 1];
    }

    let segment_t = t * (stops.len() - 1) as f32;
    let seg_idx = (segment_t.floor() as usize).min(stops.len() - 2);
    let local_t = segment_t - seg_idx as f32;

    let a = stops[seg_idx].into_linear();
    let b = stops[seg_idx + 1].into_linear();

    crate::core::Color::from_linear_rgba(
        a[0] + (b[0] - a[0]) * local_t,
        a[1] + (b[1] - a[1]) * local_t,
        a[2] + (b[2] - a[2]) * local_t,
        a[3] + (b[3] - a[3]) * local_t,
    )
}

/// State for Heatmap - stores positioned cell rectangles.
pub struct State {
    pub cell_rects: Vec<Rectangle>,
}

/// A Heatmap series that renders 2D grid cells colored by value.
pub struct Heatmap<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::heatmap::Heatmap,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Heatmap<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    /// Create a new Heatmap borrowing data.
    pub fn new(data: &'a crate::mark::heatmap::Heatmap) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Heatmap.
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State { cell_rects: Vec::new() }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Heatmap state.
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the heatmap - calculates cell positions and sizes.
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();
        state.cell_rects.clear();

        let rows = self.data.rows();
        let cols = self.data.cols();

        if rows == 0 || cols == 0 {
            return Node::new(Size::ZERO);
        }

        let cell_width = plane.bounds.width / cols as f32;
        let cell_height = plane.bounds.height / rows as f32;

        for row in 0..rows {
            for col in 0..cols {
                let center = plane.to_pixel(Datum::new(col as f64, row as f64));

                state.cell_rects.push(Rectangle {
                    x: center.x - cell_width / 2.0,
                    y: center.y - cell_height / 2.0,
                    width: cell_width,
                    height: cell_height,
                });
            }
        }

        Node::new(Size::ZERO)
    }

    /// Draws the heatmap cells and optional labels.
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
        _color_offset: usize,
        _palette: &crate::palette::Resolved,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();
        let layout_bounds = layout.bounds();
        let background = theme.background_color();
        let text_pair = theme.text_pair();

        let (v_min, v_max) = self.data.compute_range();
        let rows = self.data.rows();
        let cols = self.data.cols();

        let mut cell_frame = Frame::new(renderer, layout_bounds.size());
        let mut label_frame = Frame::new(renderer, layout_bounds.size());

        for row in 0..rows {
            for col in 0..cols {
                let idx = row * cols + col;
                let rect = &state.cell_rects[idx];
                let value = self.data.get(row, col);

                // Normalize to 0..1
                let t = if v_max > v_min {
                    ((value - v_min) / (v_max - v_min)).clamp(0.0, 1.0) as f32
                } else {
                    0.5_f32
                };

                // Interpolate color from the heatmap's own color stops
                let cell_color = interpolate_stops(&self.data.color_stops, t);

                let path = Path::new(|builder| {
                    builder.rectangle(Point::new(rect.x, rect.y), Size::new(rect.width, rect.height));
                });
                cell_frame.fill(&path, cell_color);

                // Draw label if configured
                if self.data.show_labels {
                    let label_text = (self.data.label_format)(value);

                    // Determine contrast color for label
                    let label_color = crate::color::Color::CONTRAST.resolve(cell_color, text_pair, Some(background));

                    label_frame.fill_text(CanvasText {
                        content: label_text,
                        position: Point::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0),
                        color: label_color,
                        size: 12.0.into(),
                        font: theme.font(),
                        align_x: crate::core::alignment::Horizontal::Center.into(),
                        align_y: crate::core::alignment::Vertical::Center,
                        line_height: crate::core::text::LineHeight::default(),
                        shaping: crate::core::text::Shaping::Basic,
                        ..CanvasText::default()
                    });
                }
            }
        }

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);

        let cell_geometry = cell_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(cell_geometry);
        });

        if self.data.show_labels {
            let label_geometry = label_frame.into_geometry();
            renderer.with_translation(translation, |renderer| {
                renderer.draw_geometry(label_geometry);
            });
        }
    }
}

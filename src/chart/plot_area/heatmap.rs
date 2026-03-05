use super::Plane;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Rectangle, Size};
use crate::data::Datum;
use crate::widget::canvas::{Frame, Path, Text as CanvasText};
use crate::widget::renderer::geometry;

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
        color_offset: usize,
        palette: &crate::palette::Resolved,
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

        let palette_len = palette.len();

        for row in 0..rows {
            for col in 0..cols {
                let idx = row * cols + col;
                let rect = &state.cell_rects[idx];
                let value = self.data.get(row, col);

                // Normalize to 0..1
                let t = if v_max > v_min {
                    ((value - v_min) / (v_max - v_min)).clamp(0.0, 1.0)
                } else {
                    0.5
                };

                // Sample color from palette gradient
                let color_idx = ((t * (palette_len as f64 - 1.0)).round() as usize).min(palette_len - 1);
                let cell_color = palette
                    .get(color_offset + color_idx)
                    .resolve(background, text_pair, None);

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

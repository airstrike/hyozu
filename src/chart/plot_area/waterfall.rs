use super::Plane;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Rectangle, Size};
use crate::data::Datum;
use crate::mark::waterfall::EntryKind;
use crate::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// State for Waterfall — stores pre-calculated bar rectangles and running totals
pub struct State {
    /// Pixel rectangles for each bar
    pub rects: Vec<Rectangle>,
    /// The kind of each entry (for coloring)
    pub kinds: Vec<EntryKind>,
    /// Top Y position of each bar (for connector lines)
    pub tops: Vec<f32>,
}

/// A Waterfall series that renders waterfall charts.
pub struct Waterfall<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::waterfall::Waterfall,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Waterfall<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Waterfall borrowing data
    pub fn new(data: &'a crate::mark::waterfall::Waterfall) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Waterfall
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                rects: Vec::new(),
                kinds: Vec::new(),
                tops: Vec::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Waterfall state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the waterfall — compute bar positions from running totals
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();

        if self.data.entries.is_empty() {
            state.rects.clear();
            state.kinds.clear();
            state.tops.clear();
            return Node::new(Size::ZERO);
        }

        let num_entries = self.data.entries.len();
        let total_width = plane.bounds.width;
        let bin_width = total_width / num_entries as f32;
        let bar_width = bin_width * 0.75;
        let zero_y = plane.to_pixel(Datum::ORIGIN).y;

        let mut running_total: f64 = 0.0;
        state.rects.clear();
        state.kinds.clear();
        state.tops.clear();

        for (i, entry) in self.data.entries.iter().enumerate() {
            let bar_center_x = plane.to_pixel(Datum::x(i as f64)).x;
            let x = bar_center_x - bar_width / 2.0;

            match entry.kind {
                EntryKind::Total => {
                    // Total bar starts from zero
                    running_total = entry.value;
                    let top_y = plane.to_pixel(Datum::new(0.0, entry.value)).y;
                    let height = (zero_y - top_y).abs();
                    let y = top_y.min(zero_y);

                    state.rects.push(Rectangle {
                        x,
                        y,
                        width: bar_width,
                        height,
                    });
                    state.tops.push(top_y);
                }
                EntryKind::Increase => {
                    let prev_total = running_total;
                    running_total += entry.value;
                    let bottom_y = plane.to_pixel(Datum::new(0.0, prev_total)).y;
                    let top_y = plane.to_pixel(Datum::new(0.0, running_total)).y;
                    let height = (bottom_y - top_y).abs();
                    let y = top_y.min(bottom_y);

                    state.rects.push(Rectangle {
                        x,
                        y,
                        width: bar_width,
                        height,
                    });
                    state.tops.push(top_y);
                }
                EntryKind::Decrease => {
                    let prev_total = running_total;
                    running_total += entry.value; // value is negative
                    let top_y = plane.to_pixel(Datum::new(0.0, prev_total)).y;
                    let bottom_y = plane.to_pixel(Datum::new(0.0, running_total)).y;
                    let height = (bottom_y - top_y).abs();
                    let y = top_y.min(bottom_y);

                    state.rects.push(Rectangle {
                        x,
                        y,
                        width: bar_width,
                        height,
                    });
                    state.tops.push(bottom_y);
                }
            }

            state.kinds.push(entry.kind);
        }

        Node::new(Size::ZERO)
    }

    /// Draws the waterfall chart
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
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        if state.rects.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.palette_seed();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        // Waterfall uses semantic colors directly from the design system.
        let increase_color = crate::color::Color::Success.resolve(background, text_pair, &seed, None);
        let decrease_color = crate::color::Color::Danger.resolve(background, text_pair, &seed, None);
        let total_color = crate::color::Color::Primary.resolve(background, text_pair, &seed, None);

        // Draw bars
        for (i, (rect, entry)) in state.rects.iter().zip(self.data.entries.iter()).enumerate() {
            let color = if let Some(entry_color) = entry.color {
                entry_color.resolve(background, text_pair, &seed, None)
            } else {
                match state.kinds[i] {
                    EntryKind::Increase => increase_color,
                    EntryKind::Decrease => decrease_color,
                    EntryKind::Total => total_color,
                }
            };

            let path = Path::new(|builder| {
                builder.rectangle(
                    crate::core::Point::new(rect.x, rect.y),
                    crate::core::Size::new(rect.width, rect.height),
                );
            });
            frame.fill(&path, color);
        }

        // Draw connector lines between bars
        if self.data.connector && state.rects.len() > 1 {
            let connector_color = crate::core::Color {
                a: 0.4,
                ..text_pair.on_light
            };

            for i in 0..state.rects.len() - 1 {
                let from_rect = &state.rects[i];
                let to_rect = &state.rects[i + 1];

                // Connect from the running total level of current bar to next bar
                let y = state.tops[i];

                let path = Path::new(|builder| {
                    builder.move_to(crate::core::Point::new(from_rect.x + from_rect.width, y));
                    builder.line_to(crate::core::Point::new(to_rect.x, y));
                });

                frame.stroke(&path, Stroke::default().with_width(1.0).with_color(connector_color));
            }
        }

        // Draw labels above bars
        let label_size = theme.font_size();
        let label_color = theme.text_color().resolve(background, text_pair, &seed, None);

        for (rect, entry) in state.rects.iter().zip(self.data.entries.iter()) {
            if let Some(label_text) = &entry.label {
                let negative = entry.value < 0.0;
                let (y, align_y) = if negative {
                    (rect.y + rect.height + 4.0, crate::core::alignment::Vertical::Top)
                } else {
                    (rect.y - 4.0, crate::core::alignment::Vertical::Bottom)
                };

                frame.fill_text(CanvasText {
                    content: label_text.clone(),
                    position: crate::core::Point::new(rect.x + rect.width / 2.0, y),
                    color: label_color,
                    size: label_size.into(),
                    font: theme.font(),
                    align_x: crate::core::alignment::Horizontal::Center.into(),
                    align_y,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }
}

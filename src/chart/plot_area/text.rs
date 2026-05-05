//! Renderer for [`crate::mark::text::Text`] — projects each item to a
//! pixel position (cartesian or geo, matching the underlying point
//! mark's `coord_kind`) and renders the label there with the configured
//! font size, color, and alignment.
//!
//! This is a static layer: no animation lifecycle (no `tick`, no
//! previous-snapshot diff). Composed atop an animating bubble layer it
//! still reads correctly because labels sit at their final positions
//! while the bubbles sweep underneath.

use super::{Plane, geo};
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size};
use crate::mark::text::TextAlign;
use crate::mark::xy::CoordKind;
use crate::widget::canvas::{Frame, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// State for Text — caches the pixel position of each item (already
/// projected and offset) so `draw` is a pure read with no per-frame
/// allocation aside from the canvas frame itself.
pub struct State {
    /// Pixel coordinates for each item, post-projection AND post-offset.
    pub pixel_points: Vec<Point>,
}

/// A Text overlay series.
pub struct Text<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(crate) data: &'a crate::mark::text::Text,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Text<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Text borrowing data.
    pub fn new(data: &'a crate::mark::text::Text) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Text.
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                pixel_points: Vec::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Text state.
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff.
    }

    /// Layout the labels — project each item to a pixel position via
    /// the cartesian or geo plane based on `coord_kind`, then apply the
    /// per-mark `offset`. Geo labels with no geo plane (misconfiguration)
    /// emit an empty `pixel_points`, which makes `draw` short-circuit.
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        plane: &Plane,
        geo_plane: Option<&geo::Plane>,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();
        let (mark_dx, mark_dy) = self.data.offset;
        // Per-item offset (radius-aware bubble-map labels, etc.)
        // overrides the mark-level offset. Mirrors the per-datum
        // `dy` accessor in Vega-Lite's `Plot.text`.
        let offset_for =
            |item: &crate::mark::text::TextItem| -> (f32, f32) { item.offset.unwrap_or((mark_dx, mark_dy)) };

        state.pixel_points = match self.data.coord_kind {
            CoordKind::Cartesian => self
                .data
                .items
                .iter()
                .map(|item| {
                    let p = plane.to_pixel(item.datum);
                    let (dx, dy) = offset_for(item);
                    Point::new(p.x + dx, p.y + dy)
                })
                .collect(),
            CoordKind::Geo => match geo_plane {
                Some(plane) => self
                    .data
                    .items
                    .iter()
                    .map(|item| {
                        let p = plane.project_point(item.datum.x as f32, item.datum.y as f32);
                        let (dx, dy) = offset_for(item);
                        Point::new(p.x + dx, p.y + dy)
                    })
                    .collect(),
                None => Vec::new(),
            },
        };

        Node::new(Size::ZERO)
    }

    /// Draws each label at its cached pixel position with the configured
    /// align/size/color. Falls back to the design's text color when the
    /// mark doesn't override.
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
        if state.pixel_points.is_empty() {
            return;
        }

        let layout_bounds = layout.bounds();
        // Resolve label color: per-mark override wins; otherwise the
        // design's `text_color` is resolved against the active theme so
        // labels read correctly on light or dark backgrounds.
        let color = self.data.color.unwrap_or_else(|| {
            let background = theme.background_color();
            let text_pair = theme.text_pair();
            let seed = theme.seed();
            theme.text_color().resolve(background, text_pair, &seed, None)
        });

        let align_x = match self.data.align {
            TextAlign::Left => crate::core::alignment::Horizontal::Left,
            TextAlign::Center => crate::core::alignment::Horizontal::Center,
            TextAlign::Right => crate::core::alignment::Horizontal::Right,
        };

        let mut frame = Frame::new(renderer, layout_bounds.size());
        for (i, point) in state.pixel_points.iter().enumerate() {
            let label = &self.data.items[i].label;
            if label.is_empty() {
                continue;
            }
            frame.fill_text(CanvasText {
                content: label.clone(),
                position: *point,
                color,
                size: crate::core::Pixels(self.data.size),
                font: crate::core::Font::default(),
                align_x: align_x.into(),
                align_y: crate::core::alignment::Vertical::Center,
                line_height: crate::core::text::LineHeight::default(),
                shaping: crate::core::text::Shaping::Basic,
                ..CanvasText::default()
            });
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(crate::core::Vector::new(layout_bounds.x, layout_bounds.y), |renderer| {
            renderer.draw_geometry(geometry);
        });
    }
}

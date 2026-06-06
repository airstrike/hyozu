//! Renderer for [`crate::mark::text::Text`] — projects each item to a
//! pixel position (cartesian or geo, matching the underlying point
//! mark's `coord_kind`) and renders the label there with the configured
//! font size, color, and alignment.
//!
//! This is a static layer: no animation lifecycle (no `tick`, no
//! previous-snapshot diff). Composed atop an animating bubble layer it
//! still reads correctly because labels sit at their final positions
//! while the bubbles sweep underneath.

use super::{Domain, geo, to_pixel};
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size};
use crate::mark::text::TextAlign;
use crate::mark::xy::CoordKind;

use crate::core::text::{self, paragraph};
use crate::widget::renderer::geometry;

/// State for Text — caches the pixel position of each item (already
/// projected and offset) so `draw` is a pure read with no per-frame
/// allocation aside from the canvas frame itself.
pub struct State<P>
where
    P: text::Paragraph,
{
    /// Pixel coordinates for each item, post-projection AND post-offset.
    pub pixel_points: Vec<Point>,
    /// Label paragraphs, one per item index (mirroring `pixel_points`),
    /// shaped once in `layout` and rendered in `draw` via
    /// [`crate::core::text::Renderer::fill_paragraph`]. Items with an
    /// empty label keep a default (empty) paragraph so the index stays
    /// aligned with the item order; `draw` skips them with the same guard.
    pub labels: Vec<paragraph::Plain<P>>,
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
    Renderer: text::Renderer<Font = crate::core::Font> + geometry::Renderer,
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
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                pixel_points: Vec::new(),
                labels: Vec::new(),
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
        renderer: &Renderer,
        _limits: &Limits,
        domain: &Domain,
        rect: crate::core::Rectangle,
        geo_plane: Option<&geo::Plane>,
    ) -> Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
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
                    let p = to_pixel(domain, rect, item.datum);
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

        self.shape_labels(state, renderer);

        Node::new(Size::ZERO)
    }

    /// Shapes one label paragraph per item index, theme-free (font is the
    /// mark's `Font::default()`, size is the mark-level `size`), so the
    /// text is laid out here and rendered in `draw` via `fill_paragraph`.
    /// Items with an empty label keep a default (empty) paragraph; the draw
    /// pass skips them with the same guard, so the index stays aligned with
    /// the item order.
    fn shape_labels(&self, state: &mut State<Renderer::Paragraph>, renderer: &Renderer) {
        let hint_factor = renderer.scale_factor();
        let item_count = self.data.items.len();

        while state.labels.len() < item_count {
            state.labels.push(paragraph::Plain::default());
        }
        state.labels.truncate(item_count);

        for (i, item) in self.data.items.iter().enumerate() {
            let paragraph = &mut state.labels[i];
            if item.label.is_empty() {
                continue;
            }

            let _ = paragraph.update(text::Text {
                content: &item.label,
                bounds: Size::INFINITE,
                size: crate::core::Pixels(self.data.size),
                line_height: text::LineHeight::default(),
                font: crate::core::Font::default(),
                align_x: text::Alignment::Left,
                align_y: crate::core::alignment::Vertical::Top,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
                ellipsis: text::Ellipsis::default(),
                hint_factor,
                font_features: Vec::new(),
                font_variations: Vec::new(),
                letter_spacing: Default::default(),
                weight: None,
            });
        }
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
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
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

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);
        for (i, point) in state.pixel_points.iter().enumerate() {
            let label = &self.data.items[i].label;
            if label.is_empty() {
                continue;
            }
            let Some(paragraph) = state.labels.get(i) else {
                continue;
            };

            // `fill_text` shifted the glyph box from `point` by the
            // paragraph's `min_bounds` per alignment (Left/Top→0,
            // Center→−½, Right/Bottom→−1); apply the identical shift against
            // the cached paragraph so `fill_paragraph` (top-left origin)
            // lands pixel-identically. `align_y` was always `Center`.
            let bounds = paragraph.min_bounds();
            let anchor_x = match align_x {
                crate::core::alignment::Horizontal::Left => point.x,
                crate::core::alignment::Horizontal::Center => point.x - bounds.width / 2.0,
                crate::core::alignment::Horizontal::Right => point.x - bounds.width,
            };
            let anchor_y = point.y - bounds.height / 2.0;

            renderer.fill_paragraph(
                paragraph.raw(),
                crate::core::Point::new(anchor_x, anchor_y) + translation,
                color,
                layout_bounds,
            );
        }
    }
}

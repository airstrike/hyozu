use super::{Domain, to_pixel};
use crate::animation;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::data::Datum;
use crate::mark::band::BandOrientation;
use crate::widget::canvas::{Frame, Path};

use crate::core::text::{self, paragraph};
use crate::widget::renderer::geometry;

/// State for Band — stores pixel-space rectangle spanning the plot area.
pub struct State<P>
where
    P: text::Paragraph,
{
    /// The band's edge label, shaped once in `layout` and rendered in `draw`
    /// via [`crate::core::text::Renderer::fill_paragraph`]. Empty (default)
    /// when the band has no label configured.
    pub label: paragraph::Plain<P>,
    /// Lower pixel coordinate along the band's orientation axis.
    pub lower_pixel: f32,
    /// Upper pixel coordinate along the band's orientation axis.
    pub upper_pixel: f32,
    /// Lower edge pixel from the most recent layout before the current
    /// one, captured by [`crate::chart::Chart::diff`] when data changes
    /// so the next sweep can interpolate from the previous edge to the
    /// current one. `None` on a fresh mount, in which case both edges
    /// grow apart from the band's center axis.
    pub previous_lower_pixel: Option<f32>,
    /// Upper edge pixel from the most recent layout before the current
    /// one. See [`State::previous_lower_pixel`] for the lifecycle.
    pub previous_upper_pixel: Option<f32>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// A Band series that renders a translucent shaded value range.
pub struct Band<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::band::Band,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Band<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer<Font = crate::core::Font> + geometry::Renderer,
{
    /// Create a new Band borrowing data
    pub fn new(data: &'a crate::mark::band::Band) -> Self {
        Self {
            data,
            animate: true,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets whether the mount/data-change sweep should run. Wired by
    /// [`super::PlotArea::with_animate`] from [`crate::Data::animate`].
    pub(crate) fn set_animate(&mut self, animate: bool) {
        self.animate = animate;
    }

    /// Returns the initial tree state for this Band
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                label: paragraph::Plain::default(),
                lower_pixel: 0.0,
                upper_pixel: 0.0,
                previous_lower_pixel: None,
                previous_upper_pixel: None,
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Band state
    pub(super) fn diff(&self, _tree: &mut Tree) {}

    /// Layout the band — convert data values to pixel coordinates and shape
    /// the edge label.
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        _limits: &Limits,
        domain: &Domain,
        rect: crate::core::Rectangle,
    ) -> Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        match self.data.orientation {
            BandOrientation::Horizontal => {
                state.lower_pixel = to_pixel(domain, rect, Datum::new(0.0, self.data.lower)).y;
                state.upper_pixel = to_pixel(domain, rect, Datum::new(0.0, self.data.upper)).y;
            }
            BandOrientation::Vertical => {
                state.lower_pixel = to_pixel(domain, rect, Datum::new(self.data.lower, 0.0)).x;
                state.upper_pixel = to_pixel(domain, rect, Datum::new(self.data.upper, 0.0)).x;
            }
        }

        // Shape the label paragraph theme-free; drawn in `draw` via
        // `fill_paragraph` so the text isn't reshaped every frame.
        if let Some(label_text) = &self.data.label {
            let _ = state.label.update(text::Text {
                content: label_text,
                bounds: Size::INFINITE,
                size: 11.0.into(),
                line_height: text::LineHeight::default(),
                font: renderer.default_font(),
                align_x: text::Alignment::Left,
                align_y: crate::core::alignment::Vertical::Top,
                shaping: text::Shaping::Basic,
                wrapping: text::Wrapping::None,
                ellipsis: text::Ellipsis::default(),
                hint_factor: renderer.scale_factor(),
                font_features: Vec::new(),
                font_variations: Vec::new(),
                letter_spacing: Default::default(),
                weight: None,
            });
        }

        Node::new(Size::ZERO)
    }

    /// Draws the band rectangle.
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

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let base_color = if let Some(color) = self.data.color {
            color.resolve(background, text_pair, &seed, None)
        } else {
            text_pair.on_light
        };

        let fill_color = crate::core::Color {
            a: base_color.a * self.data.opacity,
            ..base_color
        };

        // Sweep: each edge interpolates from its previous-layout pixel
        // toward its current pixel. With no previous (fresh mount) both
        // edges interpolate from the band's center axis outward, so the
        // band grows symmetrically. When animation is opted out, progress
        // pins to `1.0` and the animated edges equal `state.lower_pixel`
        // and `state.upper_pixel` exactly.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animating = progress < 1.0 - f32::EPSILON;
        let center = (state.lower_pixel + state.upper_pixel) / 2.0;
        let animated_lower = match state.previous_lower_pixel {
            Some(prev) => prev + (state.lower_pixel - prev) * progress,
            None => center + (state.lower_pixel - center) * progress,
        };
        let animated_upper = match state.previous_upper_pixel {
            Some(prev) => prev + (state.upper_pixel - prev) * progress,
            None => center + (state.upper_pixel - center) * progress,
        };

        let rect = match self.data.orientation {
            BandOrientation::Horizontal => {
                // Y-range band spans full plot width. Pixel values grow
                // downward, so `upper` (data) maps to a smaller pixel y.
                let (y0, y1) = if animated_upper <= animated_lower {
                    (animated_upper, animated_lower)
                } else {
                    (animated_lower, animated_upper)
                };
                (0.0_f32, y0, layout_bounds.width, (y1 - y0).max(0.0))
            }
            BandOrientation::Vertical => {
                let (x0, x1) = if animated_lower <= animated_upper {
                    (animated_lower, animated_upper)
                } else {
                    (animated_upper, animated_lower)
                };
                (x0, 0.0_f32, (x1 - x0).max(0.0), layout_bounds.height)
            }
        };

        let (rx, ry, rw, rh) = rect;
        if rw > 0.0 && rh > 0.0 {
            let path = Path::new(|builder| {
                builder.rectangle(crate::core::Point::new(rx, ry), crate::core::Size::new(rw, rh));
            });
            frame.fill(&path, fill_color);
        }

        // Optional label at the near edge (top for horizontal, left for vertical).
        // Suppressed mid-sweep so it doesn't pop in over edges that
        // haven't reached their final positions yet. Also gated on the
        // anchor being finite — a non-finite `data.lower`/`data.upper`
        // produces a NaN `ry`/`rx` that crashes text rendering.
        let label_anchor = if self.data.label.is_some() && !animating && rx.is_finite() && ry.is_finite() {
            let (position, align_x, align_y): (
                crate::core::Point,
                crate::core::alignment::Horizontal,
                crate::core::alignment::Vertical,
            ) = match self.data.orientation {
                BandOrientation::Horizontal => (
                    crate::core::Point::new(layout_bounds.width - 4.0, ry + 2.0),
                    crate::core::alignment::Horizontal::Right,
                    crate::core::alignment::Vertical::Top,
                ),
                BandOrientation::Vertical => (
                    crate::core::Point::new(rx + 4.0, 4.0),
                    crate::core::alignment::Horizontal::Left,
                    crate::core::alignment::Vertical::Top,
                ),
            };

            // `fill_text` shifted the glyph box from `position` by `min_bounds`
            // per alignment (Right: −width); apply the identical shift so
            // `fill_paragraph` (top-left origin) lands pixel-identically.
            let bounds = state.label.min_bounds();
            let anchor_x = match align_x {
                crate::core::alignment::Horizontal::Left => position.x,
                crate::core::alignment::Horizontal::Center => position.x - bounds.width / 2.0,
                crate::core::alignment::Horizontal::Right => position.x - bounds.width,
            };
            let anchor_y = match align_y {
                crate::core::alignment::Vertical::Top => position.y,
                crate::core::alignment::Vertical::Center => position.y - bounds.height / 2.0,
                crate::core::alignment::Vertical::Bottom => position.y - bounds.height,
            };
            Some(crate::core::Point::new(anchor_x, anchor_y))
        } else {
            None
        };

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);
        let geometry = frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(geometry);
        });

        // Draw the label text on top of the band fill via the cached paragraph.
        if let Some(anchor) = label_anchor {
            renderer.fill_paragraph(state.label.raw(), anchor + translation, base_color, layout_bounds);
        }
    }
}

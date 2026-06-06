use std::cell::RefCell;

use super::choropleth::palette_to_continuous_stops;
use super::{Domain, to_pixel};
use crate::animation;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::core::{Color, Point, Rectangle, Size};
use crate::data::Datum;
use crate::palette::Palette;
use crate::widget::canvas::{Frame, Path, Text as CanvasText};
use crate::widget::renderer::geometry;

/// Per-channel linear interpolation between `prev` and `cur`. At
/// `progress == 0.0` returns `prev`; at `1.0` returns `cur` exactly.
fn lerp_color(prev: Color, cur: Color, progress: f32) -> Color {
    Color {
        r: prev.r + (cur.r - prev.r) * progress,
        g: prev.g + (cur.g - prev.g) * progress,
        b: prev.b + (cur.b - prev.b) * progress,
        a: prev.a + (cur.a - prev.a) * progress,
    }
}

/// Default palette for a Heatmap when the user hasn't set one. A
/// sequential blue gradient matching the historical
/// [`crate::mark::heatmap::sequential_stops`] preset that the renderer
/// previously baked inline.
fn default_heatmap_palette() -> Palette {
    Palette::Gradient(
        crate::mark::heatmap::sequential_stops()
            .into_iter()
            .map(Into::into)
            .collect(),
    )
}

/// Compute the data-derived `(min, max)` value range for a Heatmap.
/// Skips non-finite values so the gradient sampler never sees NaN or Inf.
/// When all values are filtered out or the range collapses, returns a
/// `(min, min + 1.0)` degenerate range so downstream mapping has a
/// non-zero span.
fn compute_value_range(values: &[f64]) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for &v in values {
        if v.is_finite() {
            if v < lo {
                lo = v;
            }
            if v > hi {
                hi = v;
            }
        }
    }
    if lo.is_infinite() {
        lo = 0.0;
    }
    if hi.is_infinite() {
        hi = 1.0;
    }
    if (hi - lo).abs() < f64::EPSILON {
        hi = lo + 1.0;
    }
    (lo, hi)
}

/// State for Heatmap - stores positioned cell rectangles, the cached
/// data-derived value range, and the per-cell fill colors written by
/// `draw` for animation snapshotting.
pub struct State {
    pub cell_rects: Vec<Rectangle>,
    /// Cached `(min, max)` of the heatmap's values resolved against the
    /// color scale's domain override. Theme-independent; computed in
    /// [`Heatmap::layout`] and read by [`Heatmap::draw`] so the gradient
    /// mapping doesn't re-walk `self.data.values` each frame.
    pub value_range: (f64, f64),
    /// Per-cell fill colors written by [`Heatmap::draw`] at the start of
    /// each repaint, paired 1:1 with [`Self::cell_rects`]. Read by
    /// [`crate::chart::Chart::diff`] to seed `previous_cell_colors`
    /// during a data-change replant. Lives in a [`RefCell`] because fill
    /// resolution is theme-dependent (the palette can reference seed
    /// slots) and only `draw` carries the theme — `layout` cannot bake
    /// fills without staling on a theme swap.
    pub cell_colors: RefCell<Vec<Color>>,
    /// Per-cell fill colors from the most recent draw before the
    /// current one, captured by [`crate::chart::Chart::diff`] when data
    /// changes so the next sweep can interpolate from previous fills to
    /// current fills. Empty on a fresh mount, in which case the
    /// animation collapses to an alpha fade-in at the final color.
    pub previous_cell_colors: Vec<Color>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

/// A Heatmap series that renders 2D grid cells colored by value.
pub struct Heatmap<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: crate::core::text::Renderer + geometry::Renderer,
{
    pub(super) data: &'a crate::mark::heatmap::Heatmap,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
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
            animate: true,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets whether the mount/data-change sweep should run. Wired by
    /// [`super::PlotArea::with_animate`] from [`crate::Data::animate`].
    pub(crate) fn set_animate(&mut self, animate: bool) {
        self.animate = animate;
    }

    /// Returns the initial tree state for this Heatmap.
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                cell_rects: Vec::new(),
                value_range: (0.0, 1.0),
                cell_colors: RefCell::new(Vec::new()),
                previous_cell_colors: Vec::new(),
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Heatmap state.
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the heatmap - calculates cell positions and sizes, plus the
    /// data-derived value range. Per-cell fill colors are theme-dependent
    /// and resolve in [`Self::draw`] so a seed-based palette renders with
    /// the active theme's hues.
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        domain: &Domain,
        rect: Rectangle,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();
        state.cell_rects.clear();

        let rows = self.data.rows();
        let cols = self.data.cols();

        state.value_range = self
            .data
            .color
            .resolved_domain(|| compute_value_range(&self.data.values));

        if rows == 0 || cols == 0 {
            return Node::new(Size::ZERO);
        }

        let cell_width = rect.width / cols as f32;
        let cell_height = rect.height / rows as f32;

        for row in 0..rows {
            for col in 0..cols {
                let center = to_pixel(domain, rect, Datum::new(col as f64, row as f64));

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
        let seed = theme.seed();

        let rows = self.data.rows();
        let cols = self.data.cols();

        let mut cell_frame = Frame::new(renderer, layout_bounds.size());
        let mut label_frame = Frame::new(renderer, layout_bounds.size());

        // ── Resolve color scale ───────────────────────────────────
        // Theme-dependent (a palette referencing seed slots resolves
        // through the active theme's hues), so it stays in draw. The
        // user's explicit palette wins; otherwise the closure produces
        // the mark's default sequential gradient.
        let palette = self.data.color.resolved_palette(default_heatmap_palette);
        let stops: Vec<Color> = palette_to_continuous_stops(&palette, &seed);
        let transform = self.data.color.transform;
        let (v_min, v_max) = state.value_range;

        // ── Resolve target fill per cell ──────────────────────────
        // The cache is written to `state.cell_colors` so a subsequent
        // data-change replant in `chart::diff` can snapshot it onto the
        // post-rebuild tree's `previous_cell_colors`.
        let target_fills: Vec<Color> = (0..rows * cols)
            .map(|idx| {
                let row = idx / cols;
                let col = idx % cols;
                let value = self.data.get(row, col);
                let t = transform.map_to_unit(value, v_min, v_max).clamp(0.0, 1.0) as f32;
                // Non-finite value or domain produces a NaN `t`, which the
                // gradient sampler can propagate into the fill color. Treat
                // the cell as a gap (transparent) instead.
                if t.is_finite() {
                    crate::palette::sample_gradient(&stops, t)
                } else {
                    Color::TRANSPARENT
                }
            })
            .collect();

        // With a previous-draw snapshot (data change), each cell's fill
        // lerps per-channel from prev to cur. Without one (fresh mount),
        // the alpha component multiplies by progress so cells fade in
        // from fully transparent to their final color. With
        // `animate = false` progress pins to `1.0` and the rendered
        // color equals `target_fills[idx]` exactly.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animating = progress < 1.0 - f32::EPSILON;
        let has_prev = !state.previous_cell_colors.is_empty();

        for row in 0..rows {
            for col in 0..cols {
                let idx = row * cols + col;
                let rect = &state.cell_rects[idx];
                let value = self.data.get(row, col);
                let cur_color = target_fills[idx];

                let cell_color = if has_prev {
                    let prev_color = state.previous_cell_colors.get(idx).copied().unwrap_or(cur_color);
                    lerp_color(prev_color, cur_color, progress)
                } else {
                    Color {
                        a: cur_color.a * progress,
                        ..cur_color
                    }
                };

                let path = Path::new(|builder| {
                    builder.rectangle(Point::new(rect.x, rect.y), Size::new(rect.width, rect.height));
                });
                cell_frame.fill(&path, cell_color);

                // Labels suppressed mid-sweep so they don't pop in over
                // cells that haven't reached their final color yet.
                if self.data.show_labels && !animating && value.is_finite() {
                    let label_text = (self.data.label_format)(value);

                    let label_color =
                        crate::color::Color::CONTRAST.resolve(cur_color, text_pair, &seed, Some(background));

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

        // Snapshot the resolved targets for the next replant to seed
        // `previous_cell_colors` from. Always written, regardless of
        // `self.animate`, so toggling animation on later still has a
        // valid baseline.
        *state.cell_colors.borrow_mut() = target_fills;

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);

        let cell_geometry = cell_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(cell_geometry);
        });

        if self.data.show_labels && !animating {
            let label_geometry = label_frame.into_geometry();
            renderer.with_translation(translation, |renderer| {
                renderer.draw_geometry(label_geometry);
            });
        }
    }
}

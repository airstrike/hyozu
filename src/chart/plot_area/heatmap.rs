use super::Plane;
use super::choropleth::palette_to_continuous_stops;
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

/// Neutral seed used to bridge `Palette` into raw stops at layout time,
/// where no theme is available. Heatmap's preset palettes
/// (`sequential_stops`, `divergent_stops`, `warm_stops`) and any
/// user-supplied `Vec<core::Color>` route through `Color::Fixed`, which
/// ignores the seed entirely — so the neutral values matter only for the
/// rare case where a caller hands a `ColorScale` whose palette references
/// theme seed slots (`Color::Success` etc.). Those colors then resolve to
/// these neutral defaults rather than the active theme's hues; this is
/// the documented trade-off for keeping cell color resolution in layout.
fn neutral_seed() -> crate::palette::Seed {
    crate::palette::Seed {
        primary: Color::from_rgb(0.5, 0.5, 0.5),
        secondary: Color::from_rgb(0.5, 0.5, 0.5),
        success: Color::from_rgb(0.3, 0.7, 0.3),
        warning: Color::from_rgb(0.9, 0.7, 0.2),
        danger: Color::from_rgb(0.8, 0.3, 0.3),
        background: Color::from_rgb(1.0, 1.0, 1.0),
    }
}

/// State for Heatmap - stores positioned cell rectangles and pre-computed
/// per-cell fill colors (theme-independent: derived from the heatmap's
/// own value range and color stops).
pub struct State {
    pub cell_rects: Vec<Rectangle>,
    /// Per-cell fill colors computed in [`Heatmap::layout`], paired 1:1
    /// with [`Self::cell_rects`]. Cached so `draw` is a pure read and
    /// the data-change interpolation has a stable target.
    pub cell_colors: Vec<Color>,
    /// Per-cell fill colors from the most recent layout before the
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
                cell_colors: Vec::new(),
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

    /// Layout the heatmap - calculates cell positions and sizes.
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, _limits: &Limits, plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();
        state.cell_rects.clear();
        state.cell_colors.clear();

        let rows = self.data.rows();
        let cols = self.data.cols();

        if rows == 0 || cols == 0 {
            return Node::new(Size::ZERO);
        }

        let cell_width = plane.bounds.width / cols as f32;
        let cell_height = plane.bounds.height / rows as f32;

        let (v_min, v_max) = self
            .data
            .color
            .resolved_domain(|| compute_value_range(&self.data.values));
        let palette = self.data.color.resolved_palette(default_heatmap_palette);
        let seed = neutral_seed();
        let stops: Vec<Color> = palette_to_continuous_stops(&palette, &seed);
        let transform = self.data.color.transform;

        for row in 0..rows {
            for col in 0..cols {
                let center = plane.to_pixel(Datum::new(col as f64, row as f64));

                state.cell_rects.push(Rectangle {
                    x: center.x - cell_width / 2.0,
                    y: center.y - cell_height / 2.0,
                    width: cell_width,
                    height: cell_height,
                });

                let value = self.data.get(row, col);
                let t = transform.map_to_unit(value, v_min, v_max).clamp(0.0, 1.0) as f32;
                state.cell_colors.push(crate::palette::sample_gradient(&stops, t));
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

        // With a previous-layout snapshot (data change), each cell's
        // fill lerps per-channel from prev to cur. Without one (fresh
        // mount), the alpha component multiplies by progress so cells
        // fade in from fully transparent to their final color. With
        // `animate = false` progress pins to `1.0` and the rendered
        // color equals `state.cell_colors[idx]` exactly.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animating = progress < 1.0 - f32::EPSILON;
        let has_prev = !state.previous_cell_colors.is_empty();

        for row in 0..rows {
            for col in 0..cols {
                let idx = row * cols + col;
                let rect = &state.cell_rects[idx];
                let value = self.data.get(row, col);
                let cur_color = state.cell_colors[idx];

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
                if self.data.show_labels && !animating {
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

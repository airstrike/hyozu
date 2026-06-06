use super::Domain;
use crate::animation;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::mark::pie::label::{FormatKind, Position, Show};
use crate::widget::canvas::{Frame, Path, Stroke};

use crate::core::text::{self, paragraph};
use crate::widget::renderer::geometry;

/// Treats non-finite slice values (NaN, ±∞) as the gap convention:
/// the slice contributes nothing to the total and renders a zero-sweep
/// arc, so a stray `+∞` doesn't propagate into `proportion = ∞/∞ = NaN`
/// and crash the tessellator. See `feedback_nan_as_gap.md`.
fn finite_or_zero(v: f64) -> f64 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

/// Padding reserved around the pie when any visible slice has an outside label.
const OUTSIDE_LABEL_PAD: f32 = 24.0;

/// Distance from outer arc to label baseline for outside labels.
const OUTSIDE_LABEL_OFFSET: f32 = 14.0;

/// Distance from outer arc to leader-line knee for outside labels.
const OUTSIDE_LEADER_KNEE: f32 = 10.0;

/// Resolves the effective inter-slice gap in pixels. An explicit `gap`
/// always wins; otherwise rounded slices derive a gap from the corner
/// radius so they read as separated petals (a single slice never gaps).
///
/// Shared by the slice fill and the hover/selection overlays so the
/// separation can't drift between layers.
pub(crate) fn effective_gap(gap: f32, corner: f32, slice_count: usize) -> f32 {
    if slice_count <= 1 {
        0.0
    } else if gap > 0.0 {
        gap
    } else {
        corner
    }
}

/// Builds the path for one pie/donut slice: the gap-explode offset (each
/// slice nudged out along its bisector by `gap_offset`) plus corner
/// rounding via [`super::sector::push_sector_path`].
///
/// This is the single place a slice outline is constructed — the fill, the
/// selection highlight, and the hover overlay all call it, so their shapes
/// can never diverge again.
pub(crate) fn slice_path(
    center: crate::core::Point,
    inner_radius: f32,
    outer_radius: f32,
    start: f32,
    end: f32,
    gap_offset: f32,
    corner: f32,
) -> Path {
    let (scx, scy) = if gap_offset > 0.0 {
        let mid = (start + end) * 0.5;
        (center.x + gap_offset * mid.cos(), center.y + gap_offset * mid.sin())
    } else {
        (center.x, center.y)
    };
    Path::new(move |builder| {
        super::sector::push_sector_path(builder, scx, scy, inner_radius, outer_radius, start, end, corner);
    })
}

/// State for Pie — stores pre-calculated slice angles and geometry for hit-testing
pub struct State<P>
where
    P: text::Paragraph,
{
    /// Start and end angles for each slice (in radians)
    pub slice_angles: Vec<(f32, f32)>,
    /// Slice-label paragraphs, one per slice index (mirroring `slice_angles`),
    /// shaped once in [`Pie::layout`] and rendered in [`Pie::draw`] via
    /// [`crate::core::text::Renderer::fill_paragraph`]. Slices that are hidden
    /// or have no/empty label keep a default (empty) paragraph; `draw` skips
    /// them with the same visibility / empty-text guards, so the index stays
    /// aligned with the slice order.
    pub labels: Vec<paragraph::Plain<P>>,
    /// Center of the pie in local coordinates
    pub center: (f32, f32),
    /// Outer radius
    pub outer_radius: f32,
    /// Inner radius (0 for full pie, >0 for donut)
    pub inner_radius: f32,
    /// Pixel rectangles for each label
    pub label_rects: Vec<Option<crate::core::Rectangle>>,
    /// Slice angles from the most recent layout before the current one,
    /// captured by [`crate::chart::Chart::diff`] when data changes so the
    /// next sweep can interpolate from previous deltas to current deltas.
    /// Empty on a fresh mount, in which case the animation collapses to a
    /// 0 → full sweep.
    pub previous_angles: Vec<(f32, f32)>,
    /// Per-frame entrance/transition lifecycle (progress, pending-start
    /// flag, latest captured `Instant`) advanced by the chart widget.
    pub tick: animation::Tick,
}

impl<P> State<P>
where
    P: text::Paragraph,
{
    /// Returns the pie's center point and inner radius in plot-local
    /// coordinates, populated during [`Pie::layout`].
    ///
    /// Used by donut overlays to inscribe a center element inside the
    /// hole. Returns zeros when no slices have been laid out yet.
    pub fn center_geometry(&self) -> (crate::core::Point, f32) {
        (crate::core::Point::new(self.center.0, self.center.1), self.inner_radius)
    }
}

/// A Pie series that renders pie/donut charts.
pub struct Pie<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    pub(crate) data: &'a crate::mark::pie::Pie,
    /// Inner hole radius as a proportion of the outer radius. `0.0`
    /// renders a full pie; the donut variant of the chart widget
    /// writes a positive value here before layout runs.
    pub(crate) hole: f32,
    /// Resolved value-format closure for this mark (mark override → data
    /// scale). The renderer substitutes this closure for `label.format`
    /// when the slice's [`crate::mark::pie::label::FormatKind`] is
    /// `Value`. `None` means walk to
    /// [`crate::scale::default_f64_format`].
    pub(crate) value_format: Option<crate::scale::Format<f64>>,
    /// Whether the mount/data-change sweep runs. Mirrors
    /// [`crate::Data::animate`] (the chart-level toggle); `false` makes
    /// `draw` snap to the laid-out geometry.
    pub(crate) animate: bool,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Pie<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer<Font = crate::core::Font> + geometry::Renderer,
{
    /// Create a new Pie borrowing data
    pub fn new(data: &'a crate::mark::pie::Pie) -> Self {
        Self {
            data,
            hole: 0.0,
            value_format: None,
            animate: true,
            _marker: std::marker::PhantomData,
        }
    }

    /// Sets the resolved value-format closure for the value channel.
    /// Called by `PlotArea` after construction to thread the
    /// scene-level chain into the renderer; by default Pie leaves the
    /// slot empty (label closures use their own format).
    pub(crate) fn set_value_format(&mut self, value_format: Option<crate::scale::Format<f64>>) {
        self.value_format = value_format;
    }

    /// Sets whether the mount/data-change sweep should run. Wired by
    /// [`super::PlotArea::with_animate`] from [`crate::Data::animate`].
    pub(crate) fn set_animate(&mut self, animate: bool) {
        self.animate = animate;
    }

    /// Formats a slice label, walking the value-format chain when the
    /// label was built via [`crate::mark::pie::label::Label::value`] —
    /// for every other [`FormatKind`] the label's own closure wins.
    fn format_label(&self, label: &crate::mark::pie::label::Label, value: f64, pct: f64) -> String {
        match label.format_kind() {
            FormatKind::Value => match &self.value_format {
                Some(f) => f(&value),
                None => crate::scale::default_f64_format(value),
            },
            FormatKind::DefaultPercent | FormatKind::Percent | FormatKind::Custom => (label.format)(value, pct),
        }
    }

    /// Returns the initial tree state for this Pie
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State<Renderer::Paragraph>>(),
            state: tree::State::new(State::<Renderer::Paragraph> {
                slice_angles: Vec::new(),
                labels: Vec::new(),
                center: (0.0, 0.0),
                outer_radius: 0.0,
                inner_radius: 0.0,
                label_rects: Vec::new(),
                previous_angles: Vec::new(),
                tick: animation::Tick::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Pie state
    pub(super) fn diff(&self, _tree: &mut Tree) {}

    /// Layout the pie — compute angles from values
    pub fn layout(
        &self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &Limits,
        _domain: &Domain,
        _rect: crate::core::Rectangle,
        design: Option<&dyn crate::design::Design>,
    ) -> Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();

        // Base label size: the design's `data_label_text()` size (12 px when
        // the chart has no design), shared by the rect estimate below and the
        // paragraph shaping so the reserved box matches the shaped glyphs.
        let default_size = design
            .and_then(|d| d.data_label_text().size)
            .map(|p| p.0)
            .unwrap_or(12.0);

        let total: f64 = self.data.slices.iter().map(|s| finite_or_zero(s.value)).sum();

        if total == 0.0 {
            state.slice_angles.clear();
            state.label_rects.clear();
            state.labels.clear();
            return Node::new(Size::ZERO);
        }

        let visibility = compute_visibility(self.data, total);
        let needs_outside_pad =
            self.data.slices.iter().zip(visibility.iter()).any(|(s, &visible)| {
                visible && matches!(s.label.as_ref().map(|l| l.position), Some(Position::Outside))
            });

        let size = limits.max();
        let cx = size.width / 2.0;
        let cy = size.height / 2.0;
        let pad = if needs_outside_pad { OUTSIDE_LABEL_PAD } else { 0.0 };
        let radius = ((size.width.min(size.height) / 2.0 - pad).max(0.0)) * 0.95;
        let inner_radius = radius * self.hole;

        state.center = (cx, cy);
        state.outer_radius = radius;
        state.inner_radius = inner_radius;

        let mut current_angle: f32 = -std::f32::consts::FRAC_PI_2;
        state.slice_angles = self
            .data
            .slices
            .iter()
            .map(|slice| {
                let proportion = (finite_or_zero(slice.value) / total) as f32;
                let sweep = proportion * std::f32::consts::TAU;
                let start = current_angle;
                let end = current_angle + sweep;
                current_angle = end;
                (start, end)
            })
            .collect();

        state.label_rects = state
            .slice_angles
            .iter()
            .zip(self.data.slices.iter())
            .zip(visibility.iter())
            .map(|(((start, end), slice), &visible)| {
                if !visible {
                    return None;
                }
                let label = slice.label.as_ref()?;

                let mid = (start + end) / 2.0;
                let pct = finite_or_zero(slice.value) / total;
                let text = self.format_label(label, slice.value, pct);
                if text.is_empty() {
                    return None;
                }

                let font_size = label.text.size.map(|p| p.0).unwrap_or(default_size);
                let char_width = font_size * 0.6;
                let text_width = text.len() as f32 * char_width + 6.0;
                let text_height = font_size * 1.2 + 4.0;

                Some(label_rect(
                    LabelGeometry {
                        position: label.position,
                        mid,
                        cx,
                        cy,
                        radius,
                        inner_radius,
                    },
                    text_width,
                    text_height,
                ))
            })
            .collect();

        self.shape_labels(state, renderer, total, &visibility, design);

        Node::new(Size::ZERO)
    }

    /// Shapes one slice-label paragraph per slice index, theme-free (font
    /// from `renderer.default_font()` + the label's own family/weight/style,
    /// size from the label config defaulting to 12 px), so the text is laid
    /// out here and rendered in `draw` via `fill_paragraph`. Hidden slices,
    /// slices without a label, and slices whose formatted text is empty keep
    /// a default (empty) paragraph; the draw pass skips them with the same
    /// guards, so the index stays aligned with the slice order.
    fn shape_labels(
        &self,
        state: &mut State<Renderer::Paragraph>,
        renderer: &Renderer,
        total: f64,
        visibility: &[bool],
        design: Option<&dyn crate::design::Design>,
    ) {
        let default_font = design
            .map(|d| d.data_label_text().resolved_font(renderer.default_font()))
            .unwrap_or_else(|| renderer.default_font());
        let default_size = design
            .and_then(|d| d.data_label_text().size)
            .map(|p| p.0)
            .unwrap_or(12.0);
        let hint_factor = renderer.scale_factor();

        let slice_count = self.data.slices.len();
        while state.labels.len() < slice_count {
            state.labels.push(paragraph::Plain::default());
        }
        state.labels.truncate(slice_count);

        for (i, slice) in self.data.slices.iter().enumerate() {
            let paragraph = &mut state.labels[i];

            if !visibility.get(i).copied().unwrap_or(false) {
                continue;
            }
            let Some(label) = slice.label.as_ref() else {
                continue;
            };

            let pct = finite_or_zero(slice.value) / total;
            let content = self.format_label(label, slice.value, pct);
            if content.is_empty() {
                continue;
            }

            let size = label.text.size.map(|p| p.0).unwrap_or(default_size);
            let mut font = label.text.resolved_font(default_font);
            if let Some(w) = label.text.weight {
                font.weight = w;
            }
            if let Some(s) = label.text.style {
                font.style = s;
            }

            let _ = paragraph.update(text::Text {
                content: &content,
                bounds: Size::INFINITE,
                size: size.into(),
                line_height: text::LineHeight::default(),
                font,
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

    /// Draws the pie/donut chart
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
        mark_index: usize,
        selection: &Option<crate::target::Target>,
        corners: crate::core::border::Radius,
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();

        if state.slice_angles.is_empty() {
            return;
        }

        // Polar marks take a uniform cap radius (Recharts `cornerRadius`);
        // the per-corner `Radius` collapses to its `top_left` value.
        let corner = corners.top_left.max(0.0);

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let total: f64 = self.data.slices.iter().map(|s| finite_or_zero(s.value)).sum();
        let visibility = if total > 0.0 {
            compute_visibility(self.data, total)
        } else {
            vec![false; self.data.slices.len()]
        };
        let needs_outside_pad =
            self.data.slices.iter().zip(visibility.iter()).any(|(s, &visible)| {
                visible && matches!(s.label.as_ref().map(|l| l.position), Some(Position::Outside))
            });

        let cx = layout_bounds.width / 2.0;
        let cy = layout_bounds.height / 2.0;
        let pad = if needs_outside_pad { OUTSIDE_LABEL_PAD } else { 0.0 };
        let radius = ((layout_bounds.width.min(layout_bounds.height) / 2.0 - pad).max(0.0)) * 0.95;
        let inner_radius = radius * self.hole;

        // Half the effective inter-slice gap — the distance each slice is
        // nudged outward along its bisector. Rounded slices auto-derive a
        // gap so they separate into petals; see [`effective_gap`].
        let gap_offset = effective_gap(self.data.gap, corner, self.data.slices.len()) / 2.0;

        // Sweep: each slice's angular delta interpolates from its
        // previous-layout delta to its current delta, and the next
        // slice's start chains off the previous slice's *animated* end.
        // With no previous (fresh mount) prev_delta is `0`, so the
        // expression collapses to the 0 → full mount sweep. When
        // animation is opted out, progress pins to `1.0` and the
        // expression renders the laid-out geometry directly.
        let progress = if !self.animate { 1.0 } else { state.tick.progress() };
        let animating = progress < 1.0 - f32::EPSILON;
        let mut anim_cursor = state.slice_angles.first().map(|(s, _)| *s).unwrap_or(0.0);

        let mut slice_colors = Vec::with_capacity(self.data.slices.len());

        // Draw each slice
        for (i, ((orig_start, orig_end), slice)) in state.slice_angles.iter().zip(self.data.slices.iter()).enumerate() {
            let color = if let Some(slice_color) = slice.color {
                slice_color.resolve(background, text_pair, &seed, None)
            } else {
                palette
                    .get(color_offset + i)
                    .resolve(background, text_pair, &seed, None)
            };
            slice_colors.push(color);

            let cur_delta = *orig_end - *orig_start;
            let prev_delta = state.previous_angles.get(i).map(|(ps, pe)| pe - ps).unwrap_or(0.0);
            let delta = prev_delta + (cur_delta - prev_delta) * progress;
            let start = anim_cursor;
            let end = anim_cursor + delta;
            anim_cursor = end;

            let path = slice_path(
                crate::core::Point::new(cx, cy),
                inner_radius,
                radius,
                start,
                end,
                gap_offset,
                corner,
            );

            frame.fill(&path, color);
        }

        // Slice labels — suppressed mid-sweep so they don't pop in
        // before the slices they annotate are visible (matches
        // Recharts' `showLabels={!isAnimating}`). Background fills and
        // leader lines go into the geometry frame; the glyphs are collected
        // here and drawn after the frame via the cached paragraphs so they
        // sit on top of the slices.
        let mut label_draws: Vec<(usize, crate::core::Point, crate::core::Color)> = Vec::new();
        if total > 0.0 && !animating {
            for (i, ((start_angle, end_angle), slice)) in
                state.slice_angles.iter().zip(self.data.slices.iter()).enumerate()
            {
                if !visibility[i] {
                    continue;
                }
                let label = match &slice.label {
                    Some(l) => l,
                    None => continue,
                };

                let start = *start_angle;
                let end = *end_angle;
                let mid = (start + end) / 2.0;

                let pct = finite_or_zero(slice.value) / total;
                let label_text = self.format_label(label, slice.value, pct);
                if label_text.is_empty() {
                    continue;
                }

                if let Some(fill_color_spec) = label.fill
                    && let Some(Some(lr)) = state.label_rects.get(i)
                {
                    let fill_resolved = fill_color_spec.resolve(background, text_pair, &seed, None);
                    let fill_path = Path::new(|b| {
                        b.rectangle(
                            crate::core::Point::new(lr.x, lr.y),
                            crate::core::Size::new(lr.width, lr.height),
                        );
                    });
                    frame.fill(&fill_path, fill_resolved);
                }

                let (lx, ly, align_x, align_y) = label_anchor(label.position, mid, cx, cy, radius, inner_radius);

                let label_color = match label.position {
                    Position::Outside => {
                        if let Some(c) = label.color {
                            c.resolve(background, text_pair, &seed, None)
                        } else {
                            text_pair.resolve(background, None)
                        }
                    }
                    _ => {
                        let slice_fill = slice_colors[i];
                        if let Some(c) = label.color {
                            c.resolve(slice_fill, text_pair, &seed, Some(background))
                        } else {
                            text_pair.resolve(slice_fill, Some(background))
                        }
                    }
                };

                // Outside leader line: slice edge -> knee -> label edge
                if matches!(label.position, Position::Outside) {
                    let leader_color = crate::core::Color {
                        a: 0.4,
                        ..text_pair.on_light
                    };
                    let p0 = crate::core::Point::new(cx + radius * mid.cos(), cy + radius * mid.sin());
                    let p1 = crate::core::Point::new(
                        cx + (radius + OUTSIDE_LEADER_KNEE) * mid.cos(),
                        cy + (radius + OUTSIDE_LEADER_KNEE) * mid.sin(),
                    );
                    let p2 = crate::core::Point::new(lx, p1.y);

                    let leader_path = Path::new(|b| {
                        b.move_to(p0);
                        b.line_to(p1);
                        b.line_to(p2);
                    });
                    frame.stroke(&leader_path, Stroke::default().with_color(leader_color).with_width(1.0));
                }

                // `fill_text` shifted the glyph box from `(lx, ly)` by the
                // paragraph's `min_bounds` per alignment (Left/Top→0,
                // Center→−½, Right/Bottom→−1); apply the identical shift
                // against the cached paragraph so `fill_paragraph` (top-left
                // origin) lands pixel-identically with the polar anchor.
                if let Some(paragraph) = state.labels.get(i) {
                    let bounds = paragraph.min_bounds();
                    let anchor_x = match align_x {
                        crate::core::alignment::Horizontal::Left => lx,
                        crate::core::alignment::Horizontal::Center => lx - bounds.width / 2.0,
                        crate::core::alignment::Horizontal::Right => lx - bounds.width,
                    };
                    let anchor_y = match align_y {
                        crate::core::alignment::Vertical::Top => ly,
                        crate::core::alignment::Vertical::Center => ly - bounds.height / 2.0,
                        crate::core::alignment::Vertical::Bottom => ly - bounds.height,
                    };
                    label_draws.push((i, crate::core::Point::new(anchor_x, anchor_y), label_color));
                }
            }
        }

        // Selection highlights — only painted once the mount sweep
        // completes so they aren't drawn at angles the slices haven't
        // grown into yet.
        let mut selection_frame = Frame::new(renderer, layout_bounds.size());

        if let Some(target) = selection
            && !animating
        {
            use crate::target::Target;

            let inner_color = crate::core::Color::from_rgba(0.0, 0.0, 0.0, 0.5);
            let outer_color = crate::core::Color::from_rgba(1.0, 1.0, 1.0, 0.6);

            let should_highlight = |slice_idx: usize| -> bool {
                match target {
                    Target::Mark(m) => *m == mark_index,
                    Target::Series { mark, .. } => *mark == mark_index,
                    Target::Entry { mark, series: _, index } => *mark == mark_index && *index == slice_idx,
                    _ => false,
                }
            };

            let should_highlight_label = |slice_idx: usize| -> bool {
                match target {
                    Target::SeriesLabel { mark, .. } => *mark == mark_index,
                    Target::EntryLabel { mark, index, .. } => *mark == mark_index && *index == slice_idx,
                    _ => false,
                }
            };

            for (i, maybe_rect) in state.label_rects.iter().enumerate() {
                if !should_highlight_label(i) {
                    continue;
                }
                if let Some(lr) = maybe_rect {
                    let outer_rect = crate::core::Rectangle {
                        x: lr.x - 1.0,
                        y: lr.y - 1.0,
                        width: lr.width + 2.0,
                        height: lr.height + 2.0,
                    };
                    let outer_path = Path::new(|b| {
                        b.rectangle(
                            crate::core::Point::new(outer_rect.x, outer_rect.y),
                            crate::core::Size::new(outer_rect.width, outer_rect.height),
                        );
                    });
                    selection_frame.stroke(&outer_path, Stroke::default().with_color(outer_color).with_width(1.0));

                    let inner_path = Path::new(|b| {
                        b.rectangle(
                            crate::core::Point::new(lr.x, lr.y),
                            crate::core::Size::new(lr.width, lr.height),
                        );
                    });
                    selection_frame.stroke(&inner_path, Stroke::default().with_color(inner_color).with_width(1.0));
                }
            }

            for (i, (start_angle, end_angle)) in state.slice_angles.iter().enumerate() {
                if !should_highlight(i) {
                    continue;
                }

                let start = *start_angle;
                let end = *end_angle;

                let highlight_path = slice_path(
                    crate::core::Point::new(cx, cy),
                    inner_radius,
                    radius,
                    start,
                    end,
                    gap_offset,
                    corner,
                );

                selection_frame.stroke(
                    &highlight_path,
                    Stroke::default().with_color(outer_color).with_width(3.0),
                );
                selection_frame.stroke(
                    &highlight_path,
                    Stroke::default().with_color(inner_color).with_width(1.5),
                );
            }
        }

        let translation = crate::core::Vector::new(layout_bounds.x, layout_bounds.y);

        let geometry = frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(geometry);
        });

        // Slice-label glyphs sit above the slices (and their background fills
        // / leader lines) but below the selection highlights, matching the
        // previous draw order where the labels were baked into `frame`.
        for (i, anchor, color) in label_draws {
            if let Some(paragraph) = state.labels.get(i) {
                renderer.fill_paragraph(paragraph.raw(), anchor + translation, color, layout_bounds);
            }
        }

        let selection_geometry = selection_frame.into_geometry();
        renderer.with_translation(translation, |renderer| {
            renderer.draw_geometry(selection_geometry);
        });
    }
}

/// Returns a per-slice flag indicating whether the slice's label should
/// be displayed under the slice's `Show` policy.
fn compute_visibility(pie: &crate::mark::pie::Pie, total: f64) -> Vec<bool> {
    pie.slices
        .iter()
        .map(|s| {
            // Non-finite slice values are gaps — no slice arc, no label.
            if !s.value.is_finite() {
                return false;
            }
            match s.label.as_ref().map(|l| l.show) {
                None => true,
                Some(Show::All) => true,
                Some(Show::Threshold(t)) => {
                    let frac = (finite_or_zero(s.value) / total) as f32;
                    frac >= t.clamp(0.0, 1.0)
                }
                Some(Show::Top(n)) => {
                    if n == 0 {
                        return false;
                    }
                    let mut values: Vec<f64> = pie.slices.iter().map(|s| finite_or_zero(s.value)).collect();
                    values.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                    let cutoff = values.get(n.saturating_sub(1)).copied().unwrap_or(f64::INFINITY);
                    finite_or_zero(s.value) >= cutoff && finite_or_zero(s.value) > 0.0
                }
            }
        })
        .collect()
}

/// Computes the anchor point and alignment for a slice label.
fn label_anchor(
    position: Position,
    mid: f32,
    cx: f32,
    cy: f32,
    radius: f32,
    inner_radius: f32,
) -> (
    f32,
    f32,
    crate::core::alignment::Horizontal,
    crate::core::alignment::Vertical,
) {
    use crate::core::alignment::{Horizontal, Vertical};
    match position {
        Position::Inside => {
            let label_r = if inner_radius > 0.0 {
                inner_radius + (radius - inner_radius) * 0.5
            } else {
                radius * 0.65
            };
            (
                cx + label_r * mid.cos(),
                cy + label_r * mid.sin(),
                Horizontal::Center,
                Vertical::Center,
            )
        }
        Position::Edge => {
            let label_r = radius * 0.92;
            (
                cx + label_r * mid.cos(),
                cy + label_r * mid.sin(),
                Horizontal::Center,
                Vertical::Center,
            )
        }
        Position::Outside => {
            let label_r = radius + OUTSIDE_LABEL_OFFSET;
            let h = if mid.cos() >= 0.0 {
                Horizontal::Left
            } else {
                Horizontal::Right
            };
            (cx + label_r * mid.cos(), cy + label_r * mid.sin(), h, Vertical::Center)
        }
    }
}

/// Geometry inputs shared between layout and the renderer for label placement.
struct LabelGeometry {
    position: Position,
    mid: f32,
    cx: f32,
    cy: f32,
    radius: f32,
    inner_radius: f32,
}

/// Computes the bounding rect for a label given its geometry and text size.
fn label_rect(g: LabelGeometry, text_width: f32, text_height: f32) -> crate::core::Rectangle {
    let (lx, ly, align_x, align_y) = label_anchor(g.position, g.mid, g.cx, g.cy, g.radius, g.inner_radius);
    let x = match align_x {
        crate::core::alignment::Horizontal::Left => lx,
        crate::core::alignment::Horizontal::Center => lx - text_width / 2.0,
        crate::core::alignment::Horizontal::Right => lx - text_width,
    };
    let y = match align_y {
        crate::core::alignment::Vertical::Top => ly,
        crate::core::alignment::Vertical::Center => ly - text_height / 2.0,
        crate::core::alignment::Vertical::Bottom => ly - text_height,
    };
    crate::core::Rectangle {
        x,
        y,
        width: text_width,
        height: text_height,
    }
}

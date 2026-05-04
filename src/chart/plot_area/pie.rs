use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::mark::pie::label::{Position, Show};
use crate::widget::canvas::{Frame, Path, Stroke, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Number of line segments per full circle for arc approximation.
const ARC_SEGMENTS_PER_TAU: usize = 64;

/// Padding reserved around the pie when any visible slice has an outside label.
const OUTSIDE_LABEL_PAD: f32 = 24.0;

/// Distance from outer arc to label baseline for outside labels.
const OUTSIDE_LABEL_OFFSET: f32 = 14.0;

/// Distance from outer arc to leader-line knee for outside labels.
const OUTSIDE_LEADER_KNEE: f32 = 10.0;

/// State for Pie — stores pre-calculated slice angles and geometry for hit-testing
pub struct State {
    /// Start and end angles for each slice (in radians)
    pub slice_angles: Vec<(f32, f32)>,
    /// Center of the pie in local coordinates
    pub center: (f32, f32),
    /// Outer radius
    pub outer_radius: f32,
    /// Inner radius (0 for full pie, >0 for donut)
    pub inner_radius: f32,
    /// Pixel rectangles for each label
    pub label_rects: Vec<Option<crate::core::Rectangle>>,
}

impl State {
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
    pub(super) data: &'a crate::mark::pie::Pie,
    _marker: std::marker::PhantomData<(Message, Renderer)>,
}

impl<'a, Message, Renderer> Pie<'a, Message, Renderer>
where
    Message: 'a,
    Renderer: text::Renderer + geometry::Renderer,
{
    /// Create a new Pie borrowing data
    pub fn new(data: &'a crate::mark::pie::Pie) -> Self {
        Self {
            data,
            _marker: std::marker::PhantomData,
        }
    }

    /// Returns the initial tree state for this Pie
    pub(super) fn state(&self) -> Tree {
        Tree {
            tag: tree::Tag::of::<State>(),
            state: tree::State::new(State {
                slice_angles: Vec::new(),
                center: (0.0, 0.0),
                outer_radius: 0.0,
                inner_radius: 0.0,
                label_rects: Vec::new(),
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Pie state
    pub(super) fn diff(&self, _tree: &mut Tree) {}

    /// Layout the pie — compute angles from values
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, limits: &Limits, _plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();

        let total: f64 = self.data.slices.iter().map(|s| s.value.max(0.0)).sum();

        if total == 0.0 {
            state.slice_angles.clear();
            state.label_rects.clear();
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
        let inner_radius = radius * self.data.hole;

        state.center = (cx, cy);
        state.outer_radius = radius;
        state.inner_radius = inner_radius;

        let mut current_angle: f32 = -std::f32::consts::FRAC_PI_2;
        state.slice_angles = self
            .data
            .slices
            .iter()
            .map(|slice| {
                let proportion = (slice.value.max(0.0) / total) as f32;
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
                let pct = slice.value.max(0.0) / total;
                let text = (label.format)(slice.value, pct);
                if text.is_empty() {
                    return None;
                }

                let font_size = label.text.size.map(|p| p.0).unwrap_or(12.0);
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

        Node::new(Size::ZERO)
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
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        if state.slice_angles.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let seed = theme.seed();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        let total: f64 = self.data.slices.iter().map(|s| s.value.max(0.0)).sum();
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
        let inner_radius = radius * self.data.hole;

        let has_gap = self.data.gap > 0.0 && self.data.slices.len() > 1;
        let gap_offset = self.data.gap / 2.0;

        let mut slice_colors = Vec::with_capacity(self.data.slices.len());

        // Draw each slice
        for (i, ((start_angle, end_angle), slice)) in state.slice_angles.iter().zip(self.data.slices.iter()).enumerate()
        {
            let color = if let Some(slice_color) = slice.color {
                slice_color.resolve(background, text_pair, &seed, None)
            } else {
                palette
                    .get(color_offset + i)
                    .resolve(background, text_pair, &seed, None)
            };
            slice_colors.push(color);

            let start = *start_angle;
            let end = *end_angle;

            let (scx, scy) = if has_gap {
                let mid = (start + end) / 2.0;
                (cx + gap_offset * mid.cos(), cy + gap_offset * mid.sin())
            } else {
                (cx, cy)
            };

            let path = Path::new(|builder| {
                if inner_radius > 0.0 {
                    let inner_start =
                        crate::core::Point::new(scx + inner_radius * start.cos(), scy + inner_radius * start.sin());
                    let outer_start = crate::core::Point::new(scx + radius * start.cos(), scy + radius * start.sin());

                    builder.move_to(inner_start);
                    builder.line_to(outer_start);
                    trace_arc(builder, scx, scy, radius, start, end);

                    let inner_end =
                        crate::core::Point::new(scx + inner_radius * end.cos(), scy + inner_radius * end.sin());
                    builder.line_to(inner_end);
                    trace_arc(builder, scx, scy, inner_radius, end, start);
                    builder.close();
                } else {
                    builder.move_to(crate::core::Point::new(scx, scy));
                    let outer_start = crate::core::Point::new(scx + radius * start.cos(), scy + radius * start.sin());
                    builder.line_to(outer_start);
                    trace_arc(builder, scx, scy, radius, start, end);
                    builder.close();
                }
            });

            frame.fill(&path, color);
        }

        // Slice labels
        if total > 0.0 {
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

                let pct = slice.value.max(0.0) / total;
                let label_text = (label.format)(slice.value, pct);
                if label_text.is_empty() {
                    continue;
                }

                let font_size = label.text.size.unwrap_or(crate::core::Pixels(theme.font_size()));

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

                let font = label
                    .text
                    .resolved_font(theme.data_label_text().resolved_font(theme.font()));

                frame.fill_text(CanvasText {
                    content: label_text,
                    position: crate::core::Point::new(lx, ly),
                    color: label_color,
                    size: font_size,
                    font,
                    align_x: align_x.into(),
                    align_y,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }

        // Selection highlights
        let mut selection_frame = Frame::new(renderer, layout_bounds.size());

        if let Some(target) = selection {
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

                let (scx, scy) = if has_gap {
                    let mid = (start + end) / 2.0;
                    (cx + gap_offset * mid.cos(), cy + gap_offset * mid.sin())
                } else {
                    (cx, cy)
                };

                let highlight_path = Path::new(|builder| {
                    if inner_radius > 0.0 {
                        let inner_start =
                            crate::core::Point::new(scx + inner_radius * start.cos(), scy + inner_radius * start.sin());
                        let outer_start =
                            crate::core::Point::new(scx + radius * start.cos(), scy + radius * start.sin());

                        builder.move_to(inner_start);
                        builder.line_to(outer_start);
                        trace_arc(builder, scx, scy, radius, start, end);

                        let inner_end =
                            crate::core::Point::new(scx + inner_radius * end.cos(), scy + inner_radius * end.sin());
                        builder.line_to(inner_end);
                        trace_arc(builder, scx, scy, inner_radius, end, start);
                        builder.close();
                    } else {
                        builder.move_to(crate::core::Point::new(scx, scy));
                        let outer_start =
                            crate::core::Point::new(scx + radius * start.cos(), scy + radius * start.sin());
                        builder.line_to(outer_start);
                        trace_arc(builder, scx, scy, radius, start, end);
                        builder.close();
                    }
                });

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
        .map(|s| match s.label.as_ref().map(|l| l.show) {
            None => true,
            Some(Show::All) => true,
            Some(Show::Threshold(t)) => {
                let frac = (s.value.max(0.0) / total) as f32;
                frac >= t.clamp(0.0, 1.0)
            }
            Some(Show::Top(n)) => {
                if n == 0 {
                    return false;
                }
                let mut values: Vec<f64> = pie.slices.iter().map(|s| s.value.max(0.0)).collect();
                values.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                let cutoff = values.get(n.saturating_sub(1)).copied().unwrap_or(f64::INFINITY);
                s.value.max(0.0) >= cutoff && s.value.max(0.0) > 0.0
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

/// Trace an arc using line segments (avoids iced's `arc()` which uses `move_to` internally).
///
/// Draws from `start_angle` to `end_angle` around `(cx, cy)` at the given `radius`.
/// The direction depends on the sign of `end_angle - start_angle`.
fn trace_arc(
    builder: &mut crate::widget::canvas::path::Builder,
    cx: f32,
    cy: f32,
    radius: f32,
    start_angle: f32,
    end_angle: f32,
) {
    let sweep = end_angle - start_angle;
    let segments = ((sweep.abs() / std::f32::consts::TAU) * ARC_SEGMENTS_PER_TAU as f32).ceil() as usize;
    let segments = segments.max(1);

    for i in 1..=segments {
        let t = i as f32 / segments as f32;
        let angle = start_angle + sweep * t;
        builder.line_to(crate::core::Point::new(
            cx + radius * angle.cos(),
            cy + radius * angle.sin(),
        ));
    }
}

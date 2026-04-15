use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::widget::canvas::{Frame, Path, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Number of line segments per full circle for arc approximation.
const ARC_SEGMENTS_PER_TAU: usize = 64;

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
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the pie — compute angles from values
    pub fn layout(&self, tree: &mut Tree, _renderer: &Renderer, limits: &Limits, _plane: &Plane) -> Node {
        let state = tree.state.downcast_mut::<State>();

        let total: f64 = self.data.slices.iter().map(|s| s.value.max(0.0)).sum();

        if total == 0.0 {
            state.slice_angles.clear();
            return Node::new(Size::ZERO);
        }

        // Compute geometry for hit-testing (same formula as draw)
        let size = limits.max();
        let cx = size.width / 2.0;
        let cy = size.height / 2.0;
        let radius = size.width.min(size.height) / 2.0 * 0.9;
        let inner_radius = radius * self.data.hole;

        state.center = (cx, cy);
        state.outer_radius = radius;
        state.inner_radius = inner_radius;

        let mut current_angle: f32 = -std::f32::consts::FRAC_PI_2; // Start at top (12 o'clock)
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

        // Compute label rects for hit-testing
        state.label_rects = state
            .slice_angles
            .iter()
            .zip(self.data.slices.iter())
            .map(|((start, end), slice)| {
                let label = match &slice.label {
                    Some(l) => l,
                    None => return None,
                };

                let mid = (start + end) / 2.0;
                let label_r = if inner_radius > 0.0 {
                    inner_radius + (radius - inner_radius) * 0.5
                } else {
                    radius * 0.65
                };

                let lx = cx + label_r * mid.cos();
                let ly = cy + label_r * mid.sin();

                let pct = slice.value.max(0.0) / total;
                let text = (label.format)(slice.value, pct);
                if text.is_empty() {
                    return None;
                }

                let font_size = label.text.size.map(|p| p.0).unwrap_or(12.0);
                let char_width = font_size * 0.6;
                let text_width = text.len() as f32 * char_width + 6.0;
                let text_height = font_size * 1.2 + 4.0;

                // Center-aligned
                Some(crate::core::Rectangle {
                    x: lx - text_width / 2.0,
                    y: ly - text_height / 2.0,
                    width: text_width,
                    height: text_height,
                })
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
        let seed = theme.palette_seed();

        let layout_bounds = layout.bounds();
        let mut frame = Frame::new(renderer, layout_bounds.size());

        // Center and radius
        let cx = layout_bounds.width / 2.0;
        let cy = layout_bounds.height / 2.0;
        let radius = layout_bounds.width.min(layout_bounds.height) / 2.0 * 0.9;
        let inner_radius = radius * self.data.hole;

        // Compute gap offset for each slice
        let has_gap = self.data.gap > 0.0 && self.data.slices.len() > 1;
        let gap_offset = self.data.gap / 2.0;

        // Compute total for label percentages
        let total: f64 = self.data.slices.iter().map(|s| s.value.max(0.0)).sum();

        // Track resolved colors for label contrast
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

            // Apply gap offset by shifting slice center outward
            let (scx, scy) = if has_gap {
                let mid = (start + end) / 2.0;
                (cx + gap_offset * mid.cos(), cy + gap_offset * mid.sin())
            } else {
                (cx, cy)
            };

            let path = Path::new(|builder| {
                if inner_radius > 0.0 {
                    // Donut wedge
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
                    // Full pie wedge from center
                    builder.move_to(crate::core::Point::new(scx, scy));
                    let outer_start = crate::core::Point::new(scx + radius * start.cos(), scy + radius * start.sin());
                    builder.line_to(outer_start);
                    trace_arc(builder, scx, scy, radius, start, end);
                    builder.close();
                }
            });

            frame.fill(&path, color);
        }

        // Draw slice labels (second pass — on top of slices)
        if total > 0.0 {
            for (i, ((start_angle, end_angle), slice)) in
                state.slice_angles.iter().zip(self.data.slices.iter()).enumerate()
            {
                let label = match &slice.label {
                    Some(l) => l,
                    None => continue,
                };

                let start = *start_angle;
                let end = *end_angle;
                let mid = (start + end) / 2.0;

                // Position label between inner and outer radius
                let label_r = if inner_radius > 0.0 {
                    inner_radius + (radius - inner_radius) * 0.5
                } else {
                    radius * 0.65
                };

                let lx = cx + label_r * mid.cos();
                let ly = cy + label_r * mid.sin();

                let pct = slice.value.max(0.0) / total;
                let label_text = (label.format)(slice.value, pct);

                let font_size = label.text.size.unwrap_or(crate::core::Pixels(theme.font_size()));

                // Draw fill background if specified
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

                // Resolve label color for contrast against the slice
                let slice_fill = slice_colors[i];
                let label_color = if let Some(c) = label.color {
                    c.resolve(slice_fill, text_pair, &seed, Some(background))
                } else {
                    text_pair.resolve(slice_fill, Some(background))
                };

                // Resolve the font: label's family/weight/style layered
                // on top of the theme's data-label default.
                let font = label
                    .text
                    .resolved_font(theme.data_label_text().resolved_font(theme.font()));

                frame.fill_text(CanvasText {
                    content: label_text,
                    position: crate::core::Point::new(lx, ly),
                    color: label_color,
                    size: font_size,
                    font,
                    align_x: crate::core::alignment::Horizontal::Center.into(),
                    align_y: crate::core::alignment::Vertical::Center,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }

        // Draw selection highlight
        let mut selection_frame = Frame::new(renderer, layout_bounds.size());

        if let Some(target) = selection {
            use crate::target::Target;
            use crate::widget::canvas::Stroke;

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

            // Label selection highlights
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

                // Apply gap offset (same as draw)
                let has_gap = self.data.gap > 0.0 && self.data.slices.len() > 1;
                let gap_offset = self.data.gap / 2.0;
                let (scx, scy) = if has_gap {
                    let mid = (start + end) / 2.0;
                    (cx + gap_offset * mid.cos(), cy + gap_offset * mid.sin())
                } else {
                    (cx, cy)
                };

                // Build the wedge path for stroking
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

                // Outer white stroke (slightly expanded)
                selection_frame.stroke(
                    &highlight_path,
                    Stroke::default().with_color(outer_color).with_width(3.0),
                );

                // Inner black stroke
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

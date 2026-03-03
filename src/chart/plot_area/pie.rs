use super::Plane;
use crate::core::Size;
use crate::core::layout::{Limits, Node};
use crate::core::widget::{Tree, tree};
use crate::widget::canvas::{Frame, Path, Text as CanvasText};

use crate::core::text;
use crate::widget::renderer::geometry;

/// Number of line segments per full circle for arc approximation.
const ARC_SEGMENTS_PER_TAU: usize = 64;

/// State for Pie — stores pre-calculated slice angles
pub struct State {
    /// Start and end angles for each slice (in radians)
    pub slice_angles: Vec<(f32, f32)>,
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
            }),
            children: Vec::new(),
        }
    }

    /// Reconcile the tree with current Pie state
    pub(super) fn diff(&self, _tree: &mut Tree) {
        // No children to diff
    }

    /// Layout the pie — compute angles from values
    pub fn layout(
        &self,
        tree: &mut Tree,
        _renderer: &Renderer,
        _limits: &Limits,
        _plane: &Plane,
    ) -> Node {
        let state = tree.state.downcast_mut::<State>();

        let total: f64 =
            self.data.slices.iter().map(|s| s.value.max(0.0)).sum();

        if total == 0.0 {
            state.slice_angles.clear();
            return Node::new(Size::ZERO);
        }

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
    ) where
        Theme: crate::design::Design + ?Sized,
    {
        let state = tree.state.downcast_ref::<State>();

        if state.slice_angles.is_empty() {
            return;
        }

        let background = theme.background_color();
        let text_pair = theme.text_pair();
        let palette = theme.data_colors();

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
        let total: f64 =
            self.data.slices.iter().map(|s| s.value.max(0.0)).sum();

        // Track resolved colors for label contrast
        let mut slice_colors = Vec::with_capacity(self.data.slices.len());

        // Draw each slice
        for (i, ((start_angle, end_angle), slice)) in state
            .slice_angles
            .iter()
            .zip(self.data.slices.iter())
            .enumerate()
        {
            let color = if let Some(slice_color) = slice.color {
                slice_color.resolve(background, text_pair, None)
            } else {
                palette
                    .get(i % palette.len())
                    .map(|c| c.resolve(background, text_pair, None))
                    .unwrap_or(background)
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
                    let inner_start = crate::core::Point::new(
                        scx + inner_radius * start.cos(),
                        scy + inner_radius * start.sin(),
                    );
                    let outer_start = crate::core::Point::new(
                        scx + radius * start.cos(),
                        scy + radius * start.sin(),
                    );

                    builder.move_to(inner_start);
                    builder.line_to(outer_start);
                    trace_arc(builder, scx, scy, radius, start, end);

                    let inner_end = crate::core::Point::new(
                        scx + inner_radius * end.cos(),
                        scy + inner_radius * end.sin(),
                    );
                    builder.line_to(inner_end);
                    trace_arc(builder, scx, scy, inner_radius, end, start);
                    builder.close();
                } else {
                    // Full pie wedge from center
                    builder.move_to(crate::core::Point::new(scx, scy));
                    let outer_start = crate::core::Point::new(
                        scx + radius * start.cos(),
                        scy + radius * start.sin(),
                    );
                    builder.line_to(outer_start);
                    trace_arc(builder, scx, scy, radius, start, end);
                    builder.close();
                }
            });

            frame.fill(&path, color);
        }

        // Draw slice labels (second pass — on top of slices)
        if total > 0.0 {
            for (i, ((start_angle, end_angle), slice)) in state
                .slice_angles
                .iter()
                .zip(self.data.slices.iter())
                .enumerate()
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

                // Resolve label color for contrast against the slice
                let slice_fill = slice_colors[i];
                let label_color = if let Some(c) = label.color {
                    c.resolve(slice_fill, text_pair, Some(background))
                } else {
                    text_pair.resolve(slice_fill, Some(background))
                };

                let font_size = label
                    .size
                    .unwrap_or(crate::core::Pixels(theme.font_size()));

                frame.fill_text(CanvasText {
                    content: label_text,
                    position: crate::core::Point::new(lx, ly),
                    color: label_color,
                    size: font_size,
                    font: theme.font(),
                    align_x: crate::core::alignment::Horizontal::Center.into(),
                    align_y: crate::core::alignment::Vertical::Center,
                    line_height: crate::core::text::LineHeight::default(),
                    shaping: crate::core::text::Shaping::Basic,
                    ..CanvasText::default()
                });
            }
        }

        let geometry = frame.into_geometry();
        renderer.with_translation(
            crate::core::Vector::new(layout_bounds.x, layout_bounds.y),
            |renderer| {
                renderer.draw_geometry(geometry);
            },
        );
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
    let segments = ((sweep.abs() / std::f32::consts::TAU)
        * ARC_SEGMENTS_PER_TAU as f32)
        .ceil() as usize;
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

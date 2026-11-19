use crate::core::text::{self, paragraph};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size};
use crate::design::Design;
use crate::mark::diagram::Diagram;
use crate::widget::canvas::{Frame, Path, Stroke};
use crate::widget::renderer::geometry;

struct Region {
    path: Path,
    /// Screen coordinates also serve exact polygon picking.
    vertices: Vec<Point>,
    entry: Option<usize>,
    closed: bool,
    opacity: f32,
    lightness: f32,
}

pub(crate) struct State<P: text::Paragraph> {
    regions: Vec<Region>,
    labels: Vec<(Point, paragraph::Plain<P>)>,
}

pub(super) fn state<P: text::Paragraph + 'static>() -> Tree {
    Tree {
        tag: tree::Tag::of::<State<P>>(),
        state: tree::State::new(State::<P> {
            regions: Vec::new(),
            labels: Vec::new(),
        }),
        ..Tree::empty()
    }
}

impl<P: text::Paragraph> State<P> {
    pub(crate) fn pick(&self, point: Point) -> Option<usize> {
        self.regions.iter().rev().find_map(|region| {
            let index = region.entry?;
            if !region.closed {
                return region
                    .vertices
                    .windows(2)
                    .any(|w| {
                        let edge = w[1] - w[0];
                        let hit = point - w[0];
                        let denom = edge.x * edge.x + edge.y * edge.y;
                        let along = if denom == 0. {
                            0.
                        } else {
                            ((hit.x * edge.x + hit.y * edge.y) / denom).clamp(0., 1.)
                        };
                        let dx = hit.x - along * edge.x;
                        let dy = hit.y - along * edge.y;
                        dx * dx + dy * dy <= 9.
                    })
                    .then_some(index);
            }
            if region.vertices.len() < 3 {
                return None;
            }
            let mut prev = *region.vertices.last()?;
            let mut inside = false;
            for &p in &region.vertices {
                if (prev.y > point.y) != (p.y > point.y)
                    && point.x < (p.x - prev.x) * (point.y - prev.y) / (p.y - prev.y) + prev.x
                {
                    inside = !inside;
                }
                prev = p;
            }
            inside.then_some(index)
        })
    }
}

pub(super) fn layout<R: text::Renderer>(
    d: &Diagram,
    tree: &mut Tree,
    renderer: &R,
    size: Size,
    domain: &super::Domain,
    rect: crate::core::Rectangle,
    design: Option<&dyn Design>,
) {
    let state = tree.state.downcast_mut::<State<R::Paragraph>>();
    let (w, h) = if d.square {
        let side = size.width.min(size.height);
        (side, side)
    } else {
        (size.width, size.height)
    };
    let point = |[x, y]: [f64; 2]| {
        if d.cartesian {
            super::to_pixel(domain, rect, crate::Datum::new(x, y))
        } else {
            Point::new(
                (size.width - w) / 2. + x as f32 * w,
                (size.height - h) / 2. + y as f32 * h,
            )
        }
    };
    state.regions.clear();
    for shape in &d.shapes {
        let vertices: Vec<_> = shape.points.iter().copied().map(point).collect();
        let Some((&first, rest)) = vertices.split_first() else {
            continue;
        };
        let path = Path::new(|builder| {
            builder.move_to(first);
            for &v in rest {
                builder.line_to(v);
            }
            if shape.closed {
                builder.close();
            }
        });
        state.regions.push(Region {
            path,
            vertices,
            entry: shape.entry,
            closed: shape.closed,
            opacity: shape.opacity,
            lightness: shape.lightness,
        });
    }
    state
        .labels
        .resize_with(d.labels.len(), || (Point::ORIGIN, paragraph::Plain::default()));
    let font = design
        .map(|d| d.data_label_text().resolved_font(renderer.font()))
        .unwrap_or_else(|| renderer.font());
    let size = design.and_then(|d| d.data_label_text().size).unwrap_or(12.into());
    for ((pos, label), (anchor, paragraph)) in d.labels.iter().zip(&mut state.labels) {
        *anchor = point(pos.map(f64::from));
        let _ = paragraph.update(text::Text {
            content: label,
            bounds: Size::INFINITE,
            size,
            font,
            line_height: text::LineHeight::default(),
            align_x: if pos[0] < 0.46 {
                text::Alignment::Right
            } else if pos[0] > 0.54 {
                text::Alignment::Left
            } else {
                text::Alignment::Center
            },
            align_y: crate::core::alignment::Vertical::Center,
            shaping: text::Shaping::Basic,
            wrapping: text::Wrapping::None,
            ellipsis: text::Ellipsis::default(),
            hint_factor: renderer.hint_factor(),
            font_features: Vec::new(),
            font_variations: Vec::new(),
            letter_spacing: Default::default(),
            weight: None,
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw<'p, R, D: Design + ?Sized>(
    d: &Diagram,
    tree: &'p Tree,
    renderer: &mut R,
    theme: &D,
    layout: crate::core::Layout,
    offset: usize,
    palette: &crate::palette::Resolved,
    labels: &mut crate::chart::labels::Layer<'p, R::Paragraph>,
    selected: Option<usize>,
) where
    R: text::Renderer + geometry::Renderer,
{
    let state = tree.state.downcast_ref::<State<R::Paragraph>>();
    let bounds = layout.bounds();
    let translation = bounds.position() - Point::ORIGIN;
    let mut frame = Frame::new(renderer, bounds.size());
    let background = theme.background_color();
    let text = theme.text_pair();
    let seed = theme.seed();
    let ink = text.resolve(background, None);
    for region in &state.regions {
        let color = region.entry.map_or(ink, |i| {
            let resolve = |j| palette.get(offset + j).resolve(background, text, &seed, None);
            if let Some(color) = d.entries[i].color {
                color.resolve(background, text, &seed, None)
            } else if let Some((lo, hi)) = d.color_domain {
                let unit = if hi > lo {
                    (d.entries[i].value - lo) / (hi - lo)
                } else {
                    0.5
                };
                let position = unit.clamp(0., 1.) * (d.entries.len() - 1) as f64;
                let a = position.floor() as usize;
                let b = (a + 1).min(d.entries.len() - 1);
                resolve(a).mix(resolve(b), position.fract() as f32)
            } else {
                resolve(i)
            }
        });
        let color = color.mix(crate::core::Color::BLACK, 1. - region.lightness);
        if region.closed {
            frame.fill(&region.path, color.scale_alpha(region.opacity));
            if (!d.cartesian && !d.smooth_faces) || region.opacity < 1. {
                frame.stroke(
                    &region.path,
                    Stroke::default()
                        .with_color(if region.opacity < 1. { color } else { background })
                        .with_width(1.),
                );
            }
        } else {
            frame.stroke(
                &region.path,
                Stroke::default()
                    .with_color(color.scale_alpha(region.opacity))
                    .with_width(1.),
            );
        }
    }
    if let Some(index) = selected {
        for region in state.regions.iter().filter(|s| s.entry == Some(index)) {
            frame.stroke(&region.path, Stroke::default().with_color(ink).with_width(2.));
        }
    }
    let geometry = frame.into_geometry();
    renderer.with_translation(translation, |r| r.draw_geometry(geometry));
    for (anchor, p) in &state.labels {
        labels.push(p.raw(), *anchor + translation, ink);
    }
}

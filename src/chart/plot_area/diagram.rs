use crate::core::text::{self, paragraph};
use crate::core::widget::{Tree, tree};
use crate::core::{Point, Size};
use crate::design::Design;
use crate::mark::diagram::Diagram;
use crate::widget::canvas::{Frame, Path, Stroke};
use crate::widget::renderer::geometry;

struct Region {
    holes: Vec<Vec<Point>>,
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
    #[cfg(feature = "raster")]
    image_bounds: Option<crate::core::Rectangle>,
    labels: Vec<(Point, paragraph::Plain<P>)>,
}

pub(super) fn state<P: text::Paragraph + 'static>() -> Tree {
    Tree {
        tag: tree::Tag::of::<State<P>>(),
        state: tree::State::new(State::<P> {
            regions: Vec::new(),
            #[cfg(feature = "raster")]
            image_bounds: None,
            labels: Vec::new(),
        }),
        ..Tree::empty()
    }
}

impl<P: text::Paragraph> State<P> {
    /// Interior keyboard stops for native category layouts. A scanline midpoint
    /// stays inside a concave polygon or annular sector where its centroid can
    /// fall in the hole. Only one region is needed for a logical entry.
    pub(crate) fn category_targets(&self, entries: usize) -> Vec<Point> {
        let mut points = vec![None; entries];
        for region in self.regions.iter().rev() {
            let Some(id) = region.entry else {
                continue;
            };
            if points[id].is_some() || region.vertices.len() < 2 {
                continue;
            }
            if !region.closed {
                points[id] = Some(region.vertices[0] + (region.vertices[1] - region.vertices[0]) * 0.5);
                continue;
            }
            let (lo, hi) = region
                .vertices
                .iter()
                .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), p| (a.min(p.y), b.max(p.y)));
            let y = lo.midpoint(hi);
            let mut cuts = Vec::new();
            for ring in std::iter::once(&region.vertices).chain(&region.holes) {
                let Some(mut before) = ring.last().copied() else {
                    continue;
                };
                for &after in ring {
                    if (before.y > y) != (after.y > y) {
                        cuts.push(before.x + (y - before.y) * (after.x - before.x) / (after.y - before.y));
                    }
                    before = after;
                }
            }
            cuts.sort_by(f32::total_cmp);
            if let Some(pair) = cuts
                .as_chunks::<2>()
                .0
                .iter()
                .max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0])))
            {
                points[id] = Some(Point::new(pair[0].midpoint(pair[1]), y));
            }
        }
        points.into_iter().flatten().collect()
    }

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
            let mut inside = false;
            for ring in std::iter::once(&region.vertices).chain(&region.holes) {
                let Some(mut previous) = ring.last().copied() else {
                    continue;
                };
                for &p in ring {
                    if (previous.y > point.y) != (p.y > point.y)
                        && point.x < (p.x - previous.x) * (point.y - previous.y) / (p.y - previous.y) + previous.x
                    {
                        inside = !inside;
                    }
                    previous = p;
                }
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
    #[cfg(feature = "raster")]
    {
        state.image_bounds = d.image.as_ref().and(d.bounds()).map(|b| {
            let upper = point([b.x_min, b.y_max]);
            let lower = point([b.x_max, b.y_min]);
            crate::core::Rectangle::new(upper, crate::core::Size::new(lower.x - upper.x, lower.y - upper.y))
        });
    }
    state.regions.clear();
    for shape in &d.shapes {
        let vertices: Vec<_> = shape.points.iter().copied().map(point).collect();
        let Some((&first, rest)) = vertices.split_first() else {
            continue;
        };
        let holes: Vec<Vec<_>> = shape
            .holes
            .iter()
            .map(|ring| ring.iter().copied().map(point).collect())
            .collect();
        let path = Path::new(|builder| {
            builder.move_to(first);
            for &v in rest {
                builder.line_to(v);
            }
            if shape.closed {
                builder.close();
                for ring in &holes {
                    if let Some((&first, rest)) = ring.split_first() {
                        builder.move_to(first);
                        for &p in rest {
                            builder.line_to(p);
                        }
                        builder.close();
                    }
                }
            }
        });
        state.regions.push(Region {
            holes,
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
            align_x: text::Alignment::Left,
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
        let bounds = paragraph.min_bounds();
        let dx = if d.centered_labels || (0.46..=0.54).contains(&pos[0]) {
            bounds.width / 2.
        } else if pos[0] < 0.46 {
            bounds.width
        } else {
            0.
        };
        *anchor -= crate::core::Vector::new(dx, bounds.height / 2.);
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
    #[cfg(feature = "raster")]
    if let (Some(bounds), Some(image)) = (state.image_bounds, &d.image) {
        frame.draw_image(bounds, image.clone());
    }
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
            let mut fill = crate::widget::canvas::Fill::from(color.scale_alpha(region.opacity));
            if !region.holes.is_empty() {
                fill.rule = crate::widget::canvas::fill::Rule::EvenOdd;
            }
            frame.fill(&region.path, fill);
            if d.cartesian && d.smooth_faces && region.opacity == 1. {
                // Cover antialiasing seams where two opaque terrain triangles share an edge.
                frame.stroke(&region.path, Stroke::default().with_color(color).with_width(0.65));
            } else if (!d.cartesian && !d.smooth_faces) || region.opacity < 1. {
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
        labels.push_backed(
            p.raw(),
            *anchor + translation,
            d.label_color.unwrap_or(ink),
            d.label_background,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "simple-features")]
    #[test]
    fn geographic_holes_are_transparent_to_picking_and_keyboard_targets() {
        use crate::core::renderer::{Headless, Settings};
        use crate::geo::features::{Crs, Labels, Layer};
        type Renderer = crate::widget::Renderer;
        type Paragraph = <Renderer as text::Renderer>::Paragraph;
        let l = Layer::from_geojson(
            r#"{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]],[[3,3],[7,3],[7,7],[3,7],[3,3]]]}"#,
            Crs::parse("EPSG:4326").expect("WGS 84"),
        )
        .expect("polygon with hole");
        let d = l.diagram(Labels::None, |_| "region".to_owned());
        let renderer = iced::futures::executor::block_on(Renderer::new(Settings::default(), Some("tiny-skia")))
            .expect("software renderer");
        let mut tree = state::<Paragraph>();
        let size = Size::new(200., 200.);
        let unit = super::super::Range { min: 0., max: 1. };
        let domain = super::super::Domain {
            x: unit,
            y: unit,
            y_transform: crate::Transform::default(),
        };
        layout(
            &d,
            &mut tree,
            &renderer,
            size,
            &domain,
            crate::core::Rectangle::with_size(size),
            None,
        );
        let state = tree.state.downcast_ref::<State<Paragraph>>();
        assert_eq!(state.pick(Point::new(100., 100.)), None);
        assert_eq!(state.pick(Point::new(30., 100.)), Some(0));
        assert_eq!(state.pick(state.category_targets(1)[0]), Some(0));
    }

    #[test]
    fn keyboard_stops_land_inside_annular_sectors() {
        use crate::core::renderer::{Headless, Settings};
        type Renderer = crate::widget::Renderer;
        type Paragraph = <Renderer as text::Renderer>::Paragraph;
        let renderer = iced::futures::executor::block_on(Renderer::new(Settings::default(), Some("tiny-skia")))
            .expect("software renderer");
        let d = crate::rings(&["a", "b"], &[[1., 2.], [2., 1.]], 0.4).expect("two rings");
        let mut tree = state::<Paragraph>();
        let size = Size::new(400., 280.);
        let unit = super::super::Range { min: 0., max: 1. };
        let domain = super::super::Domain {
            x: unit,
            y: unit,
            y_transform: crate::Transform::default(),
        };
        layout(
            &d,
            &mut tree,
            &renderer,
            size,
            &domain,
            crate::core::Rectangle::with_size(size),
            None,
        );
        let st = tree.state.downcast_ref::<State<Paragraph>>();
        let stops = st.category_targets(2);
        assert_eq!(stops.len(), 2);
        for (i, p) in stops.into_iter().enumerate() {
            assert_eq!(st.pick(p), Some(i));
        }
    }
}

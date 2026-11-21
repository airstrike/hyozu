//! Draws a [`crate::mark::diagram::Diagram`]: projected regions for picking,
//! labels that fit their geometry or step aside, and the entrance animation.
use crate::animation;
use crate::core::text::{self, paragraph};
use crate::core::widget::{Tree, tree};
use crate::core::{Color, Point, Rectangle, Size, Vector};
use crate::design::Design;
use crate::mark::diagram::{Entrance, Sample, label};
use crate::widget::canvas::{Frame, Path, Stroke};
use crate::widget::renderer::geometry;
use std::f32::consts::TAU;

/// Distance in pixels within which the cursor touches an open path.
const STROKE: f32 = 3.;
/// Distance in pixels within which the cursor snaps to an observation.
const SNAP: f32 = 8.;
/// Clearance in pixels between a fitted label and its region's edge.
const INSET: f32 = 3.;
/// Component of a direction beyond which a label leans to that side
/// instead of centering on it, about sin(22.5°).
const LEAN: f32 = 0.38;

/// A diagram mark as drawn in the plot area.
pub struct Diagram<'a> {
    pub(crate) data: &'a crate::mark::diagram::Diagram,
    /// Mirrors [`crate::Data::animate`]. `false` draws the settled geometry.
    animate: bool,
}

impl<'a> Diagram<'a> {
    pub(crate) fn new(data: &'a crate::mark::diagram::Diagram) -> Self {
        Self { data, animate: true }
    }

    /// Wired by [`super::PlotArea::with_animate`] from [`crate::Data::animate`].
    pub(crate) fn set_animate(&mut self, animate: bool) {
        self.animate = animate;
    }
}

struct Region {
    holes: Vec<Vec<Point>>,
    path: Path,
    /// Screen coordinates also serve exact polygon picking.
    vertices: Vec<Point>,
    samples: Vec<Vertex>,
    entry: Option<usize>,
    closed: bool,
    opacity: f32,
    lightness: f32,
}

/// An observed vertex of an open path, in plot-local pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Vertex {
    pub(crate) pixel: Point,
    pub(crate) sample: Sample,
}

/// What lies under a point: an entry and, on an open path that records its
/// observations, the observation nearest the point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hit {
    pub(crate) entry: usize,
    pub(crate) vertex: Option<Vertex>,
}

/// A shaped label and its top-left corner, or `None` when it is hidden.
struct Placed<P: text::Paragraph> {
    paragraph: paragraph::Plain<P>,
    origin: Option<Point>,
    entry: Option<usize>,
    /// Sits on its entry's fill, so its color contrasts with that fill.
    inside: bool,
}

pub(crate) struct State<P: text::Paragraph> {
    regions: Vec<Region>,
    #[cfg(feature = "raster")]
    image_bounds: Option<Rectangle>,
    labels: Vec<Placed<P>>,
    /// Center of the square, the origin of a radial entrance.
    center: Point,
    entrance: Entrance,
    /// Entrance lifecycle advanced by the chart widget.
    pub(crate) tick: animation::Tick,
}

pub(super) fn state<P: text::Paragraph + 'static>() -> Tree {
    Tree {
        tag: tree::Tag::of::<State<P>>(),
        state: tree::State::new(State::<P> {
            regions: Vec::new(),
            #[cfg(feature = "raster")]
            image_bounds: None,
            labels: Vec::new(),
            center: Point::ORIGIN,
            entrance: Entrance::default(),
            tick: animation::Tick::new(),
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

    /// The entry under `point`.
    pub(crate) fn pick(&self, point: Point) -> Option<usize> {
        self.hit(point).map(|hit| hit.entry)
    }

    /// The topmost entry under `point`. On an open path with observations the
    /// cursor also counts as touching when it is near one of them, and the
    /// hit carries the nearest observation.
    pub(crate) fn hit(&self, point: Point) -> Option<Hit> {
        self.regions.iter().rev().find_map(|region| {
            let entry = region.entry?;
            if region.closed {
                return region.contains(point).then_some(Hit { entry, vertex: None });
            }
            let nearest = region
                .samples
                .iter()
                .min_by(|a, b| a.pixel.distance(point).total_cmp(&b.pixel.distance(point)))
                .copied();
            let snapped = nearest.is_some_and(|v| v.pixel.distance(point) <= SNAP);
            (snapped || region.touches(point)).then_some(Hit { entry, vertex: nearest })
        })
    }
}

impl Region {
    fn touches(&self, point: Point) -> bool {
        self.vertices.windows(2).any(|w| {
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
            dx * dx + dy * dy <= STROKE * STROKE
        })
    }

    fn contains(&self, point: Point) -> bool {
        if self.vertices.len() < 3 {
            return false;
        }
        let mut inside = false;
        for ring in std::iter::once(&self.vertices).chain(&self.holes) {
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
        inside
    }
}

/// Maps normalized or Cartesian diagram coordinates onto the plot.
struct Plane<'a> {
    cartesian: bool,
    /// Extent of the normalized square or rectangle, in pixels.
    extent: Size,
    offset: Vector,
    domain: &'a super::Domain,
    rect: Rectangle,
}

impl<'a> Plane<'a> {
    fn new(d: &crate::mark::diagram::Diagram, size: Size, domain: &'a super::Domain, rect: Rectangle) -> Self {
        let extent = if d.square {
            let side = size.width.min(size.height);
            Size::new(side, side)
        } else {
            size
        };
        Self {
            cartesian: d.cartesian,
            extent,
            offset: Vector::new((size.width - extent.width) / 2., (size.height - extent.height) / 2.),
            domain,
            rect,
        }
    }

    fn point(&self, [x, y]: [f64; 2]) -> Point {
        if self.cartesian {
            super::to_pixel(self.domain, self.rect, crate::Datum::new(x, y))
        } else {
            Point::new(x as f32 * self.extent.width, y as f32 * self.extent.height) + self.offset
        }
    }

    fn area(&self, region: &label::Region) -> Area {
        match region {
            label::Region::Rect { min, max } => {
                let a = self.point(min.map(f64::from));
                let b = self.point(max.map(f64::from));
                Area::Rect(Rectangle::new(
                    Point::new(a.x.min(b.x), a.y.min(b.y)),
                    Size::new((b.x - a.x).abs(), (b.y - a.y).abs()),
                ))
            }
            label::Region::Convex(points) => {
                Area::Convex(points.iter().map(|p| self.point(p.map(f64::from))).collect())
            }
            // Sectors belong to square layouts, so either side scales radii.
            label::Region::Sector {
                inner,
                outer,
                start,
                span,
            } => Area::Sector {
                center: self.point([0.5, 0.5]),
                inner: inner * self.extent.width,
                outer: outer * self.extent.width,
                start: *start,
                span: *span,
            },
        }
    }
}

/// A label region in plot-local pixels.
#[derive(Clone, Debug)]
enum Area {
    Rect(Rectangle),
    Convex(Vec<Point>),
    /// Angles are screen radians (y down); the sector sweeps `span`
    /// clockwise on screen from `start`.
    Sector {
        center: Point,
        inner: f32,
        outer: f32,
        start: f32,
        span: f32,
    },
}

impl Area {
    /// Whether `rect` lies entirely within the area.
    fn holds(&self, rect: Rectangle) -> bool {
        let corners = [
            rect.position(),
            Point::new(rect.x + rect.width, rect.y),
            Point::new(rect.x + rect.width, rect.y + rect.height),
            Point::new(rect.x, rect.y + rect.height),
        ];
        match *self {
            Self::Rect(area) => {
                area.x <= rect.x
                    && area.y <= rect.y
                    && rect.x + rect.width <= area.x + area.width
                    && rect.y + rect.height <= area.y + area.height
            }
            // A box is inside a convex polygon exactly when its corners are.
            Self::Convex(ref polygon) => {
                let cross = |a: Point, b: Point, p: Point| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
                let edges = || polygon.iter().zip(polygon.iter().cycle().skip(1));
                let winding: f32 = edges().map(|(&a, &b)| a.x * b.y - b.x * a.y).sum();
                polygon.len() >= 3
                    && corners
                        .iter()
                        .all(|&p| edges().all(|(&a, &b)| cross(a, b, p) * winding.signum() >= 0.))
            }
            Self::Sector {
                center,
                inner,
                outer,
                start,
                span,
            } => {
                let full = span >= TAU - 1e-4;
                let within = |p: Point| {
                    let v = p - center;
                    let r = v.x.hypot(v.y);
                    let angle = (v.y.atan2(v.x) - start).rem_euclid(TAU);
                    (inner..=outer).contains(&r) && (full || angle <= span)
                };
                if !corners.into_iter().all(within) {
                    return false;
                }
                // Corners clear of the hole can still straddle it along an edge.
                let nearest = Point::new(
                    center.x.clamp(rect.x, rect.x + rect.width),
                    center.y.clamp(rect.y, rect.y + rect.height),
                );
                if nearest.distance(center) < inner {
                    return false;
                }
                // With every corner inside, a box can leave a sector wider than
                // a half turn only by crossing one of its two straight edges.
                full || [start, start + span].into_iter().all(|angle| {
                    let along = Vector::new(angle.cos(), angle.sin());
                    !crosses(center + along * inner, center + along * outer, rect)
                })
            }
        }
    }
}

/// Whether the segment from `a` to `b` passes through `rect` (Liang–Barsky).
fn crosses(a: Point, b: Point, rect: Rectangle) -> bool {
    let d = b - a;
    let (mut enter, mut exit) = (0_f32, 1_f32);
    for (p, q) in [
        (-d.x, a.x - rect.x),
        (d.x, rect.x + rect.width - a.x),
        (-d.y, a.y - rect.y),
        (d.y, rect.y + rect.height - a.y),
    ] {
        if p == 0. {
            if q < 0. {
                return false;
            }
        } else {
            let t = q / p;
            if p < 0. {
                enter = enter.max(t);
            } else {
                exit = exit.min(t);
            }
            if enter > exit {
                return false;
            }
        }
    }
    enter < exit
}

/// Top-left corner of a box of `size` set beside `anchor`, on the side
/// that `toward` (a screen-space direction) points to.
fn beside(anchor: Point, size: Size, [dx, dy]: [f32; 2]) -> Point {
    let lean = |d: f32, extent: f32| {
        if d > LEAN {
            0.
        } else if d < -LEAN {
            -extent
        } else {
            -extent / 2.
        }
    };
    Point::new(anchor.x + lean(dx, size.width), anchor.y + lean(dy, size.height))
}

fn regions(d: &crate::mark::diagram::Diagram, plane: &Plane<'_>) -> Vec<Region> {
    let mut regions = Vec::with_capacity(d.shapes.len());
    for shape in &d.shapes {
        let vertices: Vec<_> = shape.points.iter().map(|&p| plane.point(p)).collect();
        let Some((&first, rest)) = vertices.split_first() else {
            continue;
        };
        let holes: Vec<Vec<_>> = shape
            .holes
            .iter()
            .map(|ring| ring.iter().map(|&p| plane.point(p)).collect())
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
        regions.push(Region {
            holes,
            path,
            samples: shape
                .samples
                .iter()
                .map(|&(at, sample)| Vertex {
                    pixel: plane.point(at),
                    sample,
                })
                .collect(),
            vertices,
            entry: shape.entry,
            closed: shape.closed,
            opacity: shape.opacity,
            lightness: shape.lightness,
        });
    }
    regions
}

/// Where a label of `bounds` lands, and whether it sits on its entry's fill.
fn place(
    d: &crate::mark::diagram::Diagram,
    spec: &label::Label,
    bounds: Size,
    plane: &Plane<'_>,
) -> (Option<Point>, bool) {
    let anchor = plane.point(spec.anchor.map(f64::from));
    let centered = anchor - Vector::new(bounds.width / 2., bounds.height / 2.);
    match &spec.place {
        label::Place::Free => {
            let dx = if d.centered_labels || (0.46..=0.54).contains(&spec.anchor[0]) {
                bounds.width / 2.
            } else if spec.anchor[0] < 0.46 {
                bounds.width
            } else {
                0.
            };
            (Some(anchor - Vector::new(dx, bounds.height / 2.)), false)
        }
        label::Place::Toward(toward) => (Some(beside(anchor, bounds, *toward)), false),
        label::Place::Inside { region, otherwise } => {
            if plane.area(region).holds(Rectangle::new(centered, bounds).expand(INSET)) {
                return (Some(centered), true);
            }
            match otherwise {
                label::Overflow::Hide => (None, false),
                label::Overflow::Beside { anchor, toward } => {
                    (Some(beside(plane.point(anchor.map(f64::from)), bounds, *toward)), false)
                }
            }
        }
    }
}

pub(super) fn layout<R: text::Renderer>(
    d: &crate::mark::diagram::Diagram,
    tree: &mut Tree,
    renderer: &R,
    size: Size,
    domain: &super::Domain,
    rect: Rectangle,
    design: Option<&dyn Design>,
) {
    let state = tree.state.downcast_mut::<State<R::Paragraph>>();
    let plane = Plane::new(d, size, domain, rect);
    #[cfg(feature = "raster")]
    {
        state.image_bounds = d.image.as_ref().and(d.bounds()).map(|b| {
            let upper = plane.point([b.x_min, b.y_max]);
            let lower = plane.point([b.x_max, b.y_min]);
            Rectangle::new(upper, Size::new(lower.x - upper.x, lower.y - upper.y))
        });
    }
    state.regions = regions(d, &plane);
    state.center = plane.point([0.5, 0.5]);
    state.entrance = d.entrance;
    state.labels.resize_with(d.labels.len(), || Placed {
        paragraph: paragraph::Plain::default(),
        origin: None,
        entry: None,
        inside: false,
    });
    let font = design
        .map(|d| d.data_label_text().resolved_font(renderer.font()))
        .unwrap_or_else(|| renderer.font());
    let size = design.and_then(|d| d.data_label_text().size).unwrap_or(12.into());
    for (spec, placed) in d.labels.iter().zip(&mut state.labels) {
        let _ = placed.paragraph.update(text::Text {
            content: &spec.text,
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
        (placed.origin, placed.inside) = place(d, spec, placed.paragraph.min_bounds(), &plane);
        placed.entry = spec.entry;
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw<'p, R, D: Design + ?Sized>(
    d: &Diagram<'_>,
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
    let data = d.data;
    let state = tree.state.downcast_ref::<State<R::Paragraph>>();
    let bounds = layout.bounds();
    let translation = bounds.position() - Point::ORIGIN;
    let mut frame = Frame::new(renderer, bounds.size());
    #[cfg(feature = "raster")]
    if let (Some(bounds), Some(image)) = (state.image_bounds, &data.image) {
        frame.draw_image(bounds, image.clone());
    }
    let progress = if d.animate { state.tick.progress() } else { 1. };
    let settled = progress >= 1. - f32::EPSILON;
    let background = theme.background_color();
    let text = theme.text_pair();
    let seed = theme.seed();
    let ink = text.resolve(background, None);
    let resolve = |j| palette.get(offset + j).resolve(background, text, &seed, None);
    let colors: Vec<Color> = data
        .entries
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            if let Some(color) = entry.color {
                color.resolve(background, text, &seed, None)
            } else if let Some((lo, hi)) = data.color_domain {
                let unit = if hi > lo { (entry.value - lo) / (hi - lo) } else { 0.5 };
                let position = unit.clamp(0., 1.) * (data.entries.len() - 1) as f64;
                let a = position.floor() as usize;
                let b = (a + 1).min(data.entries.len() - 1);
                resolve(a).mix(resolve(b), position.fract() as f32)
            } else {
                resolve(i)
            }
        })
        .collect();
    let paint = |frame: &mut Frame<R>, region: &Region| {
        let color = region
            .entry
            .and_then(|i| colors.get(i).copied())
            .unwrap_or(ink)
            .mix(Color::BLACK, 1. - region.lightness);
        if region.closed {
            let mut fill = crate::widget::canvas::Fill::from(color.scale_alpha(region.opacity));
            if !region.holes.is_empty() {
                fill.rule = crate::widget::canvas::fill::Rule::EvenOdd;
            }
            frame.fill(&region.path, fill);
            if data.cartesian && data.smooth_faces && region.opacity == 1. {
                // Cover antialiasing seams where two opaque terrain triangles share an edge.
                frame.stroke(&region.path, Stroke::default().with_color(color).with_width(0.65));
            } else if (!data.cartesian && !data.smooth_faces) || region.opacity < 1. {
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
    };
    if settled {
        for region in &state.regions {
            paint(&mut frame, region);
        }
    } else {
        // Guides hold still under the entering geometry.
        for region in state.regions.iter().filter(|r| r.entry.is_none()) {
            paint(&mut frame, region);
        }
        let entering = |frame: &mut Frame<R>| {
            for region in state.regions.iter().filter(|r| r.entry.is_some()) {
                paint(frame, region);
            }
        };
        let size = bounds.size();
        if progress > 0. {
            match state.entrance {
                Entrance::Radial => frame.with_save(|frame| {
                    let center = state.center - Point::ORIGIN;
                    frame.translate(center);
                    frame.scale(progress);
                    frame.translate(-center);
                    entering(frame);
                }),
                Entrance::Rightward => frame.with_clip(
                    Rectangle::new(Point::ORIGIN, Size::new(size.width * progress, size.height)),
                    entering,
                ),
                Entrance::Downward => frame.with_clip(
                    Rectangle::new(Point::ORIGIN, Size::new(size.width, size.height * progress)),
                    entering,
                ),
            }
        }
    }
    if settled && let Some(index) = selected {
        for region in state.regions.iter().filter(|s| s.entry == Some(index)) {
            frame.stroke(&region.path, Stroke::default().with_color(ink).with_width(2.));
        }
    }
    let geometry = frame.into_geometry();
    renderer.with_translation(translation, |r| r.draw_geometry(geometry));
    for placed in &state.labels {
        let Some(origin) = placed.origin else {
            continue;
        };
        // Data labels appear once their geometry has arrived.
        if placed.entry.is_some() && !settled {
            continue;
        }
        let color = data.label_color.unwrap_or_else(|| {
            match placed.entry.filter(|_| placed.inside).and_then(|i| colors.get(i)) {
                Some(&fill) => text.resolve(fill, Some(background)),
                None => ink,
            }
        });
        labels.push_backed(
            placed.paragraph.raw(),
            origin + translation,
            color,
            data.label_background,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit() -> super::super::Domain {
        let unit = super::super::Range { min: 0., max: 1. };
        super::super::Domain::linear(unit, unit)
    }

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
        layout(
            &d,
            &mut tree,
            &renderer,
            size,
            &unit(),
            Rectangle::with_size(size),
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
        layout(
            &d,
            &mut tree,
            &renderer,
            size,
            &unit(),
            Rectangle::with_size(size),
            None,
        );
        let st = tree.state.downcast_ref::<State<Paragraph>>();
        let stops = st.category_targets(2);
        assert_eq!(stops.len(), 2);
        for (i, p) in stops.into_iter().enumerate() {
            assert_eq!(st.pick(p), Some(i));
        }
    }

    /// Geometry-only state for a Cartesian diagram drawn over `[0, 10]²` on
    /// a 100 px square, so one data unit spans ten pixels.
    fn traced(d: &crate::mark::diagram::Diagram) -> State<<crate::widget::Renderer as text::Renderer>::Paragraph> {
        let span = super::super::Range { min: 0., max: 10. };
        let domain = super::super::Domain::linear(span, span);
        let size = Size::new(100., 100.);
        let plane = Plane::new(d, size, &domain, Rectangle::with_size(size));
        State {
            regions: regions(d, &plane),
            #[cfg(feature = "raster")]
            image_bounds: None,
            labels: Vec::new(),
            center: plane.point([0.5, 0.5]),
            entrance: d.entrance,
            tick: animation::Tick::new(),
        }
    }

    #[test]
    fn step_hover_snaps_to_observations_not_corners() {
        let d = crate::cartesian::steps(&[[0., 2.], [5., 8.], [10., 4.]], crate::cartesian::Step::After);
        let st = traced(&d);
        // The run up to the corner at (5, 2) belongs to the first
        // observation, and the riser's top to the second. The corner itself
        // is drawn but never observed.
        let run = st.hit(Point::new(45., 80.)).expect("on the run");
        assert_eq!(run.entry, 0);
        assert_eq!(run.vertex.map(|v| v.sample), Some(Sample::Cartesian { x: 0., y: 2. }));
        let riser = st
            .hit(Point::new(50., 25.))
            .and_then(|h| h.vertex)
            .expect("on the riser");
        assert_eq!(riser.sample, Sample::Cartesian { x: 5., y: 8. });
        assert_eq!(riser.pixel, Point::new(50., 20.));
        // Close to an observation but off the stroke still snaps.
        let near = st.hit(Point::new(96., 64.)).expect("snapped");
        assert_eq!(near.vertex.map(|v| v.sample), Some(Sample::Cartesian { x: 10., y: 4. }));
        assert_eq!(st.hit(Point::new(30., 50.)), None);
    }

    #[test]
    fn smooth_scatter_hover_reports_input_samples() {
        let data = crate::cartesian::smooth_scatter(&[[0., 0.], [5., 10.], [10., 0.]], false);
        let crate::Mark::Diagram(d) = &data.marks()[0] else {
            panic!("smooth path");
        };
        let st = traced(d);
        let path = st.regions.iter().find(|r| r.entry.is_some()).expect("path");
        assert!(path.vertices.len() > 3);
        // A point on the curve between the first two observations.
        let between = path.vertices[8];
        let vertex = st.hit(between).and_then(|h| h.vertex).expect("vertex");
        assert!(matches!(vertex.sample, Sample::Cartesian { x, .. } if x == 0. || x == 5.));
    }

    #[test]
    fn closed_regions_hit_without_a_vertex() {
        let d = crate::funnel(&[("a", 4.), ("b", 2.)]);
        let st = traced(&d);
        // Normalized funnels ignore the Cartesian domain.
        let hit = st.hit(Point::new(50., 25.)).expect("first stage");
        assert_eq!(hit, Hit { entry: 0, vertex: None });
    }

    #[test]
    fn rectangles_hold_only_boxes_inside_them() {
        let area = Area::Rect(Rectangle::new(Point::new(10., 10.), Size::new(80., 30.)));
        assert!(area.holds(Rectangle::new(Point::new(20., 15.), Size::new(60., 20.))));
        assert!(!area.holds(Rectangle::new(Point::new(20., 15.), Size::new(80., 20.))));
        assert!(!area.holds(Rectangle::new(Point::new(20., 15.), Size::new(20., 40.))));
    }

    #[test]
    fn sectors_hold_boxes_clear_of_their_hole_and_straight_edges() {
        let center = Point::new(100., 100.);
        // A quarter ring from three to six o'clock on screen.
        let quarter = Area::Sector {
            center,
            inner: 40.,
            outer: 95.,
            start: 0.,
            span: TAU / 4.,
        };
        let label = |x: f32, y: f32, w: f32| Rectangle::new(Point::new(x, y), Size::new(w, 14.));
        assert!(quarter.holds(label(140., 140., 24.)));
        // Too wide: a corner leaves the outer arc.
        assert!(!quarter.holds(label(140., 140., 60.)));
        // Above three o'clock lies outside the sweep.
        assert!(!quarter.holds(label(150., 90., 20.)));
        // Corners outside the hole while the near edge cuts through it.
        let ring = Area::Sector {
            center,
            inner: 40.,
            outer: 95.,
            start: 0.,
            span: TAU,
        };
        assert!(!ring.holds(Rectangle::new(Point::new(60., 130.), Size::new(80., 14.))));
        assert!(ring.holds(Rectangle::new(Point::new(80., 150.), Size::new(40., 14.))));
        // Wider than a half turn: every corner is swept, but the box spans
        // the unswept wedge around twelve o'clock.
        let open = Area::Sector {
            center,
            inner: 20.,
            outer: 95.,
            start: -TAU / 4. + 0.3,
            span: TAU - 0.6,
        };
        assert!(!open.holds(Rectangle::new(Point::new(80., 30.), Size::new(40., 14.))));
        assert!(open.holds(Rectangle::new(Point::new(80., 160.), Size::new(40., 14.))));
    }

    #[test]
    fn sunburst_labels_fit_their_sectors_or_hide() {
        use crate::mark::diagram::Node;
        let tree = [
            Node::branch("Wide", vec![Node::leaf("Big", 90.), Node::leaf("x", 1.)]),
            Node::leaf("Thin", 1.),
        ];
        let d = crate::sunburst(&tree);
        let domain = unit();
        let plane = Plane::new(
            &d,
            Size::new(400., 400.),
            &domain,
            Rectangle::with_size(Size::new(400., 400.)),
        );
        let fits = |name: &str, width: f32| {
            let spec = d.labels.iter().find(|l| l.text == name).expect("label");
            place(&d, spec, Size::new(width, 14.), &plane).0.is_some()
        };
        assert!(fits("Wide", 30.));
        assert!(fits("Big", 30.));
        assert!(!fits("x", 30.));
        assert!(!fits("Thin", 30.));
        assert!(!fits("Big", 400.));
    }

    #[test]
    fn funnel_labels_step_beside_a_stage_too_narrow_for_them() {
        let d = crate::funnel(&[("Visited", 100.), ("Bought", 4.)]);
        let size = Size::new(400., 200.);
        let domain = unit();
        let plane = Plane::new(&d, size, &domain, Rectangle::with_size(size));
        assert_eq!(d.labels[0].text, "Visited · 100");
        let bounds = Size::new(80., 14.);
        let (inside, on_fill) = place(&d, &d.labels[0], bounds, &plane);
        assert!(on_fill);
        assert_eq!(inside, Some(Point::new(160., 43.)));
        let (beside, on_fill) = place(&d, &d.labels[1], bounds, &plane);
        assert!(!on_fill);
        assert!(beside.is_some_and(|p| p.x > 200. && p.y < 150.));
    }

    #[test]
    fn labels_beside_an_anchor_read_away_from_it() {
        let size = Size::new(40., 10.);
        let anchor = Point::new(100., 100.);
        assert_eq!(beside(anchor, size, [1., 0.]), Point::new(100., 95.));
        assert_eq!(beside(anchor, size, [-1., 0.]), Point::new(60., 95.));
        assert_eq!(beside(anchor, size, [0., -1.]), Point::new(80., 90.));
        assert_eq!(beside(anchor, size, [0.7, 0.7]), Point::new(100., 100.));
    }
}

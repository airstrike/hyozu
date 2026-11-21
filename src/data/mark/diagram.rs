//! Indexed geometry for coordinate layouts and Cartesian marks.
pub(crate) mod label;

use crate::color::Color;
use label::Label;
use std::f64::consts::{FRAC_PI_2, TAU};

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub value: f64,
    pub color: Option<Color>,
    pub(crate) quantitative: bool,
}

/// An observed vertex of an open path. Hover snaps to these, never to the
/// interpolated or corner vertices drawn between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sample {
    /// An observation in data coordinates.
    Cartesian { x: f64, y: f64 },
    /// A data angle in radians and a radius in data units.
    Polar { angle: f64, radius: f64 },
    /// A value on one of a radar's named spokes, by spoke index.
    Spoke { spoke: usize, value: f64 },
    /// A Sankey link carrying `value` from entry `source` to entry `target`.
    Flow { source: usize, target: usize, value: f64 },
}

#[derive(Clone, Debug)]
pub(crate) struct Shape {
    pub holes: Vec<Vec<[f64; 2]>>,
    pub points: Vec<[f64; 2]>,
    /// Observations along an open path, each at its position in the same
    /// space as `points`.
    pub samples: Vec<([f64; 2], Sample)>,
    pub entry: Option<usize>,
    pub closed: bool,
    pub opacity: f32,
    pub lightness: f32,
}

/// How entry geometry appears when the chart mounts. Guides never move.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Entrance {
    /// Revealed left to right, like a line sweep.
    #[default]
    Rightward,
    /// Revealed top to bottom, following a funnel's stage order.
    Downward,
    /// Grown outward from the center of the square.
    Radial,
}

/// Native plot mark shared by hierarchical, flow and polar layouts.
#[derive(Clone, Debug, Default)]
pub struct Diagram {
    pub(crate) entries: Vec<Entry>,
    pub(crate) shapes: Vec<Shape>,
    pub(crate) labels: Vec<Label>,
    /// Names of a radar's spokes, which [`Sample::Spoke`] indexes.
    pub(crate) spokes: Vec<String>,
    pub(crate) centered_labels: bool,
    pub(crate) entrance: Entrance,
    pub(crate) label_color: Option<crate::core::Color>,
    pub(crate) label_background: Option<(crate::core::Color, f32)>,
    pub(crate) square: bool,
    pub(crate) cartesian: bool,
    pub(crate) smooth_faces: bool,
    #[cfg(feature = "raster")]
    pub(crate) image: Option<crate::core::image::Image>,
    pub(crate) range: Option<crate::data::area::Bounds>,
    pub(crate) color_domain: Option<(f64, f64)>,
    /// Ascending edges of the value bins a banded field fills, one more
    /// than its entries. Set by banded constructors such as
    /// [`crate::field::Mesh::contour_bands`] so a scale legend steps
    /// through the same levels.
    pub(crate) bins: Option<Vec<f64>>,
    /// Scale-legend placement for a scalar diagram.
    pub(crate) scale_legend: Option<crate::data::legend::Config>,
    /// Title shown above the scale legend.
    pub(crate) legend_title: Option<String>,
}

impl Diagram {
    pub(crate) fn cartesian() -> Self {
        Self {
            cartesian: true,
            ..Self::default()
        }
    }

    pub(crate) fn transpose(&mut self) {
        for shape in &mut self.shapes {
            for p in &mut shape.points {
                p.swap(0, 1);
            }
            for (p, sample) in &mut shape.samples {
                p.swap(0, 1);
                if let Sample::Cartesian { x, y } = sample {
                    std::mem::swap(x, y);
                }
            }
        }
        if let Some(b) = self.range.as_mut() {
            std::mem::swap(&mut b.x_min, &mut b.y_min);
            std::mem::swap(&mut b.x_max, &mut b.y_max);
        }
    }

    pub(crate) fn bounds(&self) -> Option<crate::data::area::Bounds> {
        self.range
    }

    pub(crate) fn scalar_colors(&mut self) {
        self.color_domain = self.entries.iter().map(|e| e.value).fold(None, |range, x| {
            Some(range.map_or((x, x), |(lo, hi): (f64, f64)| (lo.min(x), hi.max(x))))
        });
    }

    pub(crate) fn path(&mut self, index: usize, points: Vec<[f64; 2]>, closed: bool, opacity: f32) {
        self.push_path(index, points, closed, opacity);
    }

    /// An open path whose hover snaps to `samples`, the observations it
    /// passes through. Samples share the coordinate space of `points`.
    pub(crate) fn trace(&mut self, index: usize, points: Vec<[f64; 2]>, samples: Vec<([f64; 2], Sample)>) {
        self.sampled(index, points, false, 1., samples);
    }

    /// A path, open or closed, whose hover reports `samples`, the
    /// observations at its vertices: a radar polygon's spoke values, or the
    /// point a scatter disc stands for.
    pub(crate) fn sampled(
        &mut self,
        index: usize,
        points: Vec<[f64; 2]>,
        closed: bool,
        opacity: f32,
        samples: Vec<([f64; 2], Sample)>,
    ) {
        if self.push_path(index, points, closed, opacity)
            && let Some(shape) = self.shapes.last_mut()
        {
            shape.samples = samples;
        }
    }

    /// The name of radar spoke `index`.
    pub(crate) fn spoke(&self, index: usize) -> Option<&str> {
        self.spokes.get(index).map(String::as_str)
    }

    /// A guide label aligned by its horizontal position.
    pub(crate) fn label(&mut self, anchor: [f32; 2], text: impl Into<String>) {
        self.labels.push(Label::free(anchor, text));
    }

    fn push_path(&mut self, index: usize, points: Vec<[f64; 2]>, closed: bool, opacity: f32) -> bool {
        let valid = points.len() >= 2 && points.iter().flatten().all(|v| v.is_finite());
        if valid {
            if self.cartesian {
                for &[x, y] in &points {
                    let b = self.range.get_or_insert(crate::data::area::Bounds {
                        x_min: x,
                        x_max: x,
                        y_min: y,
                        y_max: y,
                    });
                    b.x_min = b.x_min.min(x);
                    b.x_max = b.x_max.max(x);
                    b.y_min = b.y_min.min(y);
                    b.y_max = b.y_max.max(y);
                }
            }
            self.shapes.push(Shape {
                holes: Vec::new(),
                points,
                samples: Vec::new(),
                entry: Some(index),
                closed,
                opacity,
                lightness: 1.,
            });
        }
        valid
    }

    /// Configures a scale legend for a diagram colored by value, such as
    /// a contour plot, a hexbin or a tile grid: a color bar, or one color
    /// step per band for filled contours. The legend replaces the
    /// per-entry swatches in the series legend. Diagrams that color by
    /// category ignore it.
    pub fn scale_legend(mut self, legend: impl Into<Option<crate::data::legend::Config>>) -> Self {
        self.scale_legend = legend.into();
        self
    }

    /// Sets the title shown above the scale legend.
    pub fn legend_title(mut self, title: impl Into<String>) -> Self {
        self.legend_title = Some(title.into());
        self
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn set_color(&mut self, index: usize, color: impl Into<Color>) {
        if let Some(entry) = self.entries.get_mut(index) {
            entry.color = Some(color.into());
        }
    }

    /// A series spanning multiple values has a name without a single measured value.
    pub(crate) fn series(&mut self, name: &str) -> usize {
        let index = self.entry(name, 0.);
        self.entries[index].quantitative = false;
        index
    }

    pub(crate) fn entry(&mut self, name: &str, value: f64) -> usize {
        let index = self.entries.len();
        self.entries.push(Entry {
            name: name.to_owned(),
            value,
            color: None,
            quantitative: true,
        });
        index
    }

    fn polygon(&mut self, index: usize, points: Vec<[f32; 2]>, opacity: f32) {
        self.shapes.push(Shape {
            holes: Vec::new(),
            points: points.into_iter().map(|p| p.map(f64::from)).collect(),
            samples: Vec::new(),
            entry: Some(index),
            closed: true,
            opacity,
            lightness: 1.,
        });
    }

    pub(crate) fn guide(&mut self, points: Vec<[f32; 2]>) {
        self.shapes.push(Shape {
            holes: Vec::new(),
            points: points.into_iter().map(|p| p.map(f64::from)).collect(),
            samples: Vec::new(),
            entry: None,
            closed: false,
            opacity: 0.35,
            lightness: 1.,
        });
    }
}

fn polar(radius: f64, turn: f64) -> [f32; 2] {
    let theta = turn * TAU - FRAC_PI_2;
    [(0.5 + radius * theta.cos()) as f32, (0.5 + radius * theta.sin()) as f32]
}

fn sector(inner: f64, outer: f64, start: f64, end: f64) -> Vec<[f32; 2]> {
    let steps = ((end - start).abs() * 144.).ceil().max(1.) as usize;
    let turn = |i: usize| start + (end - start) * i as f64 / steps as f64;
    let mut points: Vec<_> = (0..=steps).map(|i| polar(outer, turn(i))).collect();
    points.extend((0..=steps).rev().map(|i| polar(inner, turn(i))));
    points
}

/// Stage widths encode values relative to the widest stage. The last stage
/// terminates at its own width. Invalid or negative values are omitted.
/// Each stage carries its name and value, inside the stage when the text
/// fits and beside its right edge otherwise.
pub fn funnel<S: AsRef<str>>(stages: &[(S, f64)]) -> Diagram {
    let mut d = Diagram {
        entrance: Entrance::Downward,
        ..Diagram::default()
    };
    let stages: Vec<_> = stages.iter().filter(|(_, v)| v.is_finite() && *v >= 0.).collect();
    let max = stages.iter().map(|s| s.1).fold(0., f64::max);
    if max <= 0. {
        return d;
    }
    for (i, &(name, v)) in stages.iter().enumerate() {
        let a = (*v / max * 0.48) as f32;
        let b = (stages.get(i + 1).map_or(*v, |s| s.1) / max * 0.48) as f32;
        let y = i as f32 / stages.len() as f32;
        let end = (i + 1) as f32 / stages.len() as f32;
        let id = d.entry(name.as_ref(), *v);
        let stage = vec![[0.5 - a, y], [0.5 + a, y], [0.5 + b, end], [0.5 - b, end]];
        let region = label::Region::Convex(stage.clone());
        d.polygon(id, stage, 1.);
        let middle = y.midpoint(end);
        let text = format!("{} · {}", name.as_ref(), crate::scale::default_f64_format(*v));
        d.labels.push(
            Label::inside(id, [0.5, middle], region, text).or_beside([0.5 + a.midpoint(b) + 0.012, middle], [1., 0.]),
        );
    }
    d
}

/// One concentric bar per observation. Its swept angle encodes the fraction
/// of a common maximum starting at twelve o'clock and growing clockwise.
/// See [`crate::polar::Axes::radial_bars`] to configure the value ticks,
/// start angle and direction.
pub fn radial_bars<S: AsRef<str>>(rows: &[(S, f64)], maximum: f64) -> Diagram {
    if !maximum.is_finite() || maximum <= 0. {
        return Diagram {
            square: true,
            ..Diagram::default()
        };
    }
    crate::polar::Axes::compass().maximum(maximum).radial_bars(rows)
}

/// Radar axes in input order. All series use the same positive maximum.
/// Incomplete series and nonfinite values are omitted.
pub fn radar<S: AsRef<str>, T: AsRef<str>, V: AsRef<[f64]>>(axes: &[S], series: &[(T, V)], maximum: f64) -> Diagram {
    radar_with_style(axes, series, maximum, RadarStyle::Filled)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadarStyle {
    Outline,
    Markers,
    Filled,
}

/// A common radial scale with a closed outline, optional vertex markers or
/// filled series. See [`crate::polar::Axes::radar`] to configure the radial
/// ticks, their format, the start angle and the direction.
pub fn radar_with_style<S: AsRef<str>, T: AsRef<str>, V: AsRef<[f64]>>(
    axes: &[S],
    series: &[(T, V)],
    maximum: f64,
    style: RadarStyle,
) -> Diagram {
    if !maximum.is_finite() || maximum <= 0. {
        return Diagram {
            square: true,
            ..Diagram::default()
        };
    }
    crate::polar::Axes::compass()
        .maximum(maximum)
        .radar(axes, series, style)
}

/// A hierarchy whose branch weights are calculated from their descendant leaves.
#[derive(Clone, Debug)]
pub enum Node {
    Leaf(String, f64),
    Branch(String, Vec<Node>),
}

impl Node {
    pub fn leaf(name: impl Into<String>, value: f64) -> Self {
        Self::Leaf(name.into(), value)
    }
    pub fn branch(name: impl Into<String>, children: Vec<Self>) -> Self {
        Self::Branch(name.into(), children)
    }
    fn name(&self) -> &str {
        match self {
            Self::Leaf(name, _) | Self::Branch(name, _) => name,
        }
    }
    fn weight(&self) -> f64 {
        match self {
            Self::Leaf(_, v) if v.is_finite() && *v > 0. => *v,
            Self::Leaf(_, _) => 0.,
            Self::Branch(_, nodes) => nodes.iter().map(Self::weight).sum(),
        }
    }
    fn depth(&self) -> usize {
        match self {
            Self::Leaf(_, _) => 1,
            Self::Branch(_, nodes) => 1 + nodes.iter().map(Self::depth).max().unwrap_or(0),
        }
    }
}

/// Concentric partition. Children divide exactly the angular extent of their
/// parent. Each sector is labelled with its name when the text fits inside it.
pub fn sunburst(roots: &[Node]) -> Diagram {
    fn walk(d: &mut Diagram, nodes: &[Node], depth: usize, thick: f64, start: f64, end: f64) {
        let total: f64 = nodes.iter().map(Node::weight).sum();
        if !total.is_finite() || total <= 0. {
            return;
        }
        let mut angle = start;
        for node in nodes {
            let value = node.weight();
            if value <= 0. {
                continue;
            }
            let next = angle + (end - start) * (value / total);
            let id = d.entry(node.name(), value);
            let inner = 0.035 + thick * depth as f64;
            let outer = 0.035 + thick * (depth + 1) as f64;
            d.polygon(id, sector(inner, outer, angle, next), 1.);
            let region = label::Region::Sector {
                inner: inner as f32,
                outer: outer as f32,
                start: (angle * TAU - FRAC_PI_2) as f32,
                span: ((next - angle) * TAU) as f32,
            };
            let anchor = polar(inner.midpoint(outer), angle.midpoint(next));
            d.labels.push(Label::inside(id, anchor, region, node.name()));
            if let Node::Branch(_, children) = node {
                walk(d, children, depth + 1, thick, angle, next);
            }
            angle = next;
        }
    }
    let mut d = Diagram {
        square: true,
        ..Diagram::default()
    };
    d.entrance = Entrance::Radial;
    let levels = roots.iter().map(Node::depth).max().unwrap_or(1);
    walk(&mut d, roots, 0, 0.45 / levels as f64, 0., 1.);
    d
}

/// Recursively squarifies children inside their parent, using the same
/// rectangle-packing implementation as the flat treemap. Parent gutters
/// expose the hierarchy. Entry indices follow depth-first order. Leaves are
/// labelled with their names when the text fits inside them.
pub fn hierarchical_treemap(roots: &[Node]) -> Diagram {
    use crate::core::Rectangle;
    fn walk(d: &mut Diagram, nodes: &[Node], bounds: Rectangle) {
        let mut weights: Vec<_> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n.weight()))
            .filter(|(_, w)| *w > 0.)
            .collect();
        weights.sort_by(|a, b| b.1.total_cmp(&a.1));
        let total: f64 = weights.iter().map(|x| x.1).sum();
        if !total.is_finite() || total <= 0. {
            return;
        }
        let normalized: Vec<_> = weights.into_iter().map(|(i, w)| (i, (w / total) as f32)).collect();
        for (i, rect) in crate::chart::plot_area::treemap::squarify(&normalized, 1., bounds) {
            let node = &nodes[i];
            let id = d.entry(node.name(), node.weight());
            let x = rect.x;
            let y = rect.y;
            let right = (x + rect.width).min(bounds.x + bounds.width);
            let bottom = (y + rect.height).min(bounds.y + bounds.height);
            d.polygon(id, vec![[x, y], [right, y], [right, bottom], [x, bottom]], 1.);
            if let Node::Leaf(name, _) = node {
                let region = label::Region::Rect {
                    min: [x, y],
                    max: [right, bottom],
                };
                d.labels
                    .push(Label::inside(id, [x.midpoint(right), y.midpoint(bottom)], region, name));
            }
            if let Node::Branch(_, children) = node {
                let pad = 0.008_f32.min(rect.width * 0.08).min(rect.height * 0.08);
                walk(d, children, Rectangle {
                    x: x + pad,
                    y: y + pad,
                    width: rect.width - 2. * pad,
                    height: rect.height - 2. * pad,
                });
            }
        }
    }
    let mut d = Diagram::default();
    walk(&mut d, roots, Rectangle {
        x: 0.,
        y: 0.,
        width: 1.,
        height: 1.,
    });
    d
}

/// A directed flow between indices in the Sankey node list.
#[derive(Clone, Copy, Debug)]
pub struct Flow {
    pub source: usize,
    pub target: usize,
    pub value: f64,
}

/// One left-to-right column per dependency level. All nodes and links share a
/// single value-to-height scale, including flows that skip columns. Node value
/// equals the larger of its incoming and outgoing totals. A cycle, unknown
/// endpoint, invalid value or nonfinite total returns `None`. Node names sit
/// beside their nodes: right of every column but the last, left of the last.
/// Each link carries its value a third of the way along when the text fits
/// inside the band, and hovering a link reports its source, target and
/// value.
pub fn sankey<S: AsRef<str>>(names: &[S], flows: &[Flow]) -> Option<Diagram> {
    let count = names.len();
    let mut incoming = vec![0.; count];
    let mut outgoing = vec![0.; count];
    let mut indegree = vec![0; count];
    let mut edges = vec![Vec::new(); count];
    for (i, f) in flows.iter().enumerate() {
        if f.source >= count || f.target >= count || !f.value.is_finite() || f.value < 0. {
            return None;
        }
        if f.value == 0. {
            continue;
        }
        incoming[f.target] += f.value;
        outgoing[f.source] += f.value;
        indegree[f.target] += 1;
        edges[f.source].push(i);
    }
    let mut queue: std::collections::VecDeque<_> = (0..count).filter(|&i| indegree[i] == 0).collect();
    let mut levels = vec![0; count];
    let mut seen = 0;
    while let Some(node) = queue.pop_front() {
        seen += 1;
        for &i in &edges[node] {
            let to = flows[i].target;
            levels[to] = levels[to].max(levels[node] + 1);
            indegree[to] -= 1;
            if indegree[to] == 0 {
                queue.push_back(to);
            }
        }
    }
    if seen != count {
        return None;
    }
    let mut d = Diagram::default();
    let mut columns = vec![Vec::new(); levels.iter().copied().max().unwrap_or(0) + 1];
    for (i, name) in names.iter().enumerate() {
        let value = incoming[i].max(outgoing[i]);
        d.entry(name.as_ref(), value);
        if value > 0. {
            columns[levels[i]].push(i);
        }
    }
    let gap = 0.025_f64.min(0.35 / columns.iter().map(Vec::len).max().unwrap_or(1).max(1) as f64);
    let mut scale = f64::INFINITY;
    for col in &columns {
        let total: f64 = col.iter().map(|&i| d.entries[i].value).sum();
        if !total.is_finite() {
            return None;
        }
        if total > 0. {
            scale = scale.min((0.94 - gap * col.len().saturating_sub(1) as f64) / total);
        }
    }
    if !scale.is_finite() {
        return Some(d);
    }
    let width = 0.035_f64.min(0.4 / columns.len() as f64);
    let mut pos = vec![(0., 0.); count];
    let mut node_shapes = Vec::new();
    let last = columns.len() - 1;
    for (level, col) in columns.iter().enumerate() {
        let x = 0.01 + level as f64 / columns.len().saturating_sub(1).max(1) as f64 * (0.98 - width);
        let height =
            col.iter().map(|&i| d.entries[i].value * scale).sum::<f64>() + gap * col.len().saturating_sub(1) as f64;
        let mut y = (1. - height) / 2.;
        for &i in col {
            let h = d.entries[i].value * scale;
            pos[i] = (x, y);
            let middle = (y + h / 2.) as f32;
            let name = names[i].as_ref();
            d.labels.push(if level == last && last > 0 {
                Label::toward([(x - 0.006) as f32, middle], [-1., 0.], name).of(i)
            } else {
                Label::toward([(x + width + 0.006) as f32, middle], [1., 0.], name).of(i)
            });
            node_shapes.push((i, vec![
                [x as f32, y as f32],
                [(x + width) as f32, y as f32],
                [(x + width) as f32, (y + h) as f32],
                [x as f32, (y + h) as f32],
            ]));
            y += h + gap;
        }
    }
    let mut source_offset = vec![0.; count];
    let mut target_offset = vec![0.; count];
    for f in flows.iter().filter(|f| f.value > 0.) {
        let (xa, ya) = pos[f.source];
        let (xb, yb) = pos[f.target];
        let y1 = ya + source_offset[f.source];
        let y2 = yb + target_offset[f.target];
        let h = f.value * scale;
        let boundary = |i, dy: f64| {
            let t = i as f64 / 20.;
            let ease = t * t * (3. - 2. * t);
            [
                ((xa + width) * (1. - t) + xb * t) as f32,
                (y1 + (y2 - y1) * ease + dy) as f32,
            ]
        };
        let mut path: Vec<_> = (0..=20).map(|i| boundary(i, 0.)).collect();
        path.extend((0..=20).rev().map(|i| boundary(i, h)));
        // Hover anchors at the band's middle. The value label sits a third
        // of the way along instead, where a short stretch of band is near
        // enough a parallelogram to hold it, and where links that cross
        // (symmetric about their middles) don't stack their labels.
        let [x, y] = boundary(10, h / 2.);
        let anchor = [f64::from(x), f64::from(y)];
        let stretch = vec![boundary(5, 0.), boundary(9, 0.), boundary(9, h), boundary(5, h)];
        let label_at = boundary(7, h / 2.);
        let flow = Sample::Flow {
            source: f.source,
            target: f.target,
            value: f.value,
        };
        d.sampled(
            f.source,
            path.into_iter().map(|p| p.map(f64::from)).collect(),
            true,
            0.36,
            vec![(anchor, flow)],
        );
        d.labels.push(
            Label::inside(
                f.source,
                label_at,
                label::Region::Convex(stretch),
                crate::scale::default_f64_format(f.value),
            )
            .on_band(),
        );
        source_offset[f.source] += h;
        target_offset[f.target] += h;
    }
    for (i, shape) in node_shapes {
        d.polygon(i, shape, 1.);
    }
    Some(d)
}

/// Concentric doughnuts, with each ring normalized independently and a
/// consistent category color across rings. The first row is the innermost ring.
pub fn rings<S: AsRef<str>, V: AsRef<[f64]>>(categories: &[S], rows: &[V], hole: f64) -> Option<Diagram> {
    if categories.is_empty() || rows.is_empty() || !(0.0..1.0).contains(&hole) {
        return None;
    }
    let mut d = Diagram {
        square: true,
        entrance: Entrance::Radial,
        ..Diagram::default()
    };
    for name in categories {
        d.series(name.as_ref());
    }
    let width = (1. - hole) * 0.46 / rows.len() as f64;
    for (i, row) in rows.iter().enumerate() {
        let values = row.as_ref();
        let total: f64 = values.iter().sum();
        if values.len() != categories.len() || !total.is_finite() || total <= 0. || values.iter().any(|v| *v < 0.) {
            return None;
        }
        let inner = 0.46 * hole + i as f64 * width;
        let mut t = 0.;
        for (j, &v) in values.iter().enumerate() {
            let end = t + v / total;
            if end > t {
                d.polygon(j, sector(inner, inner + width * 0.96, t, end), 1.);
            }
            t = end;
        }
    }
    Some(d)
}

/// A flat pie with an independent radial offset, in radius units, per wedge.
pub fn exploded_pie<S: AsRef<str>>(items: &[(S, f64, f64)]) -> Option<Diagram> {
    if items
        .iter()
        .any(|(_, v, e)| !v.is_finite() || *v < 0. || !e.is_finite() || *e < 0.)
    {
        return None;
    }
    let total: f64 = items.iter().map(|i| i.1).sum();
    if !total.is_finite() || total <= 0. {
        return None;
    }
    let maximum = items.iter().map(|i| i.2).fold(0., f64::max);
    let r = 0.46 / (1. + maximum);
    let mut t = 0.;
    let mut d = Diagram {
        square: true,
        entrance: Entrance::Radial,
        ..Diagram::default()
    };
    for (name, v, explode) in items {
        let id = d.entry(name.as_ref(), *v);
        if *v == 0. {
            continue;
        }
        let end = t + v / total;
        let [dx, dy] = polar(explode * r, t.midpoint(end));
        let points = sector(0., r, t, end)
            .into_iter()
            .map(|[x, y]| [x + dx - 0.5, y + dy - 0.5])
            .collect();
        d.polygon(id, points, 1.);
        t = end;
    }
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sankey_rejects_cycles_and_invalid_endpoints() {
        let nodes = ["a", "b"];
        let edge = Flow {
            source: 0,
            target: 1,
            value: 2.,
        };
        assert!(
            sankey(&nodes, &[edge, Flow {
                source: 1,
                target: 0,
                ..edge
            }])
            .is_none()
        );
        assert!(sankey(&nodes, &[Flow { target: 2, ..edge }]).is_none());
        let chart = sankey(&nodes, &[edge]).expect("directed acyclic flow");
        assert_eq!(chart.entries[1].value, 2.);
        assert_eq!(chart.shapes.len(), 3);
    }
    #[test]
    fn hierarchy_preserves_parent_child_weights() {
        let tree = [
            Node::branch("A", vec![Node::leaf("a1", 3.), Node::leaf("a2", 1.)]),
            Node::leaf("B", 2.),
        ];
        for d in [sunburst(&tree), hierarchical_treemap(&tree)] {
            assert_eq!(d.entries.iter().map(|e| e.value).collect::<Vec<_>>(), [4., 3., 1., 2.]);
            assert!(
                d.shapes
                    .iter()
                    .flat_map(|s| &s.points)
                    .flatten()
                    .all(|&v| (0.0..=1.).contains(&v))
            );
        }
    }
}

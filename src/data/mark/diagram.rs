//! Indexed geometry for coordinate layouts and Cartesian marks.
use crate::color::Color;
use std::f64::consts::{FRAC_PI_2, TAU};

#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub value: f64,
    pub color: Option<Color>,
    pub(crate) quantitative: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct Shape {
    pub points: Vec<[f64; 2]>,
    pub entry: Option<usize>,
    pub closed: bool,
    pub opacity: f32,
    pub lightness: f32,
}

/// Native plot mark shared by hierarchical, flow and polar layouts.
#[derive(Clone, Debug, Default)]
pub struct Diagram {
    pub(crate) entries: Vec<Entry>,
    pub(crate) shapes: Vec<Shape>,
    pub(crate) labels: Vec<([f32; 2], String)>,
    pub(crate) square: bool,
    pub(crate) cartesian: bool,
    pub(crate) smooth_faces: bool,
    pub(crate) range: Option<crate::data::area::Bounds>,
    pub(crate) color_domain: Option<(f64, f64)>,
}

impl Diagram {
    pub(crate) fn cartesian() -> Self {
        Self {
            cartesian: true,
            ..Self::default()
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
        if points.len() >= 2 && points.iter().flatten().all(|v| v.is_finite()) {
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
                points,
                entry: Some(index),
                closed,
                opacity,
                lightness: 1.,
            });
        }
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
            points: points.into_iter().map(|p| p.map(f64::from)).collect(),
            entry: Some(index),
            closed: true,
            opacity,
            lightness: 1.,
        });
    }

    pub(crate) fn guide(&mut self, points: Vec<[f32; 2]>) {
        self.shapes.push(Shape {
            points: points.into_iter().map(|p| p.map(f64::from)).collect(),
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
pub fn funnel<S: AsRef<str>>(stages: &[(S, f64)]) -> Diagram {
    let mut d = Diagram::default();
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
        d.polygon(id, vec![[0.5 - a, y], [0.5 + a, y], [0.5 + b, end], [0.5 - b, end]], 1.);
    }
    d
}

/// One concentric bar per observation. Its swept angle encodes the fraction
/// of a common maximum starting at twelve o'clock.
pub fn radial_bars<S: AsRef<str>>(rows: &[(S, f64)], maximum: f64) -> Diagram {
    let mut d = Diagram {
        square: true,
        ..Diagram::default()
    };
    if !maximum.is_finite() || maximum <= 0. {
        return d;
    }
    let rows: Vec<_> = rows.iter().filter(|(_, v)| v.is_finite() && *v >= 0.).collect();
    let thickness = 0.40 / rows.len().max(1) as f64;
    for (i, (name, v)) in rows.into_iter().enumerate() {
        let id = d.entry(name.as_ref(), *v);
        if *v == 0. {
            continue;
        }
        let inner = 0.08 + i as f64 * thickness;
        d.polygon(
            id,
            sector(inner, inner + 0.9 * thickness, 0., (v / maximum).min(1.)),
            1.,
        );
    }
    d
}

/// Radar axes in input order. All series use the same positive maximum.
/// Incomplete series and nonfinite values are omitted.
pub fn radar<S: AsRef<str>, T: AsRef<str>, V: AsRef<[f64]>>(axes: &[S], series: &[(T, V)], maximum: f64) -> Diagram {
    let mut d = Diagram {
        square: true,
        ..Diagram::default()
    };
    if axes.len() < 3 || !maximum.is_finite() || maximum <= 0. {
        return d;
    }
    let angle = |i| i as f64 / axes.len() as f64;
    for ring in 1..=4 {
        d.guide(
            (0..=axes.len())
                .map(|i| polar(0.37 * ring as f64 / 4., angle(i)))
                .collect(),
        );
    }
    for (i, axis) in axes.iter().enumerate() {
        d.guide(vec![polar(0., 0.), polar(0.37, angle(i))]);
        d.labels.push((polar(0.44, angle(i)), axis.as_ref().to_owned()));
    }
    for (name, values) in series {
        let values = values.as_ref();
        if values.len() != axes.len() || values.iter().any(|v| !v.is_finite()) {
            continue;
        }
        let id = d.series(name.as_ref());
        d.polygon(
            id,
            values
                .iter()
                .enumerate()
                .map(|(i, v)| polar((v / maximum).clamp(0., 1.) * 0.37, angle(i)))
                .collect(),
            0.33,
        );
    }
    d
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

/// Concentric partition. Children divide exactly the angular extent of their parent.
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
            d.polygon(
                id,
                sector(
                    0.035 + thick * depth as f64,
                    0.035 + thick * (depth + 1) as f64,
                    angle,
                    next,
                ),
                1.,
            );
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
    let levels = roots.iter().map(Node::depth).max().unwrap_or(1);
    walk(&mut d, roots, 0, 0.45 / levels as f64, 0., 1.);
    d
}

/// Recursively squarifies children inside their parent, using the same
/// rectangle-packing implementation as the flat treemap. Parent gutters
/// expose the hierarchy. Entry indices follow depth-first order.
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
/// endpoint, invalid value or nonfinite total returns `None`.
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
    for (level, col) in columns.iter().enumerate() {
        let x = 0.01 + level as f64 / columns.len().saturating_sub(1).max(1) as f64 * (0.98 - width);
        let height =
            col.iter().map(|&i| d.entries[i].value * scale).sum::<f64>() + gap * col.len().saturating_sub(1) as f64;
        let mut y = (1. - height) / 2.;
        for &i in col {
            let h = d.entries[i].value * scale;
            pos[i] = (x, y);
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
        d.polygon(f.source, path, 0.36);
        source_offset[f.source] += h;
        target_offset[f.target] += h;
    }
    for (i, shape) in node_shapes {
        d.polygon(i, shape, 1.);
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

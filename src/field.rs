//! Scientific field plots. Axes, clipping and selection use the chart's
//! ordinary Cartesian coordinate transform.
use crate::Diagram;

/// A measured scalar at an x/y position.
#[derive(Clone, Copy, Debug)]
pub struct Sample {
    pub x: f64,
    pub y: f64,
    pub value: f64,
}

impl Sample {
    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.value.is_finite()
    }
    fn position(self) -> [f64; 2] {
        [self.x, self.y]
    }
}

/// Triangles over a scalar field. Invalid topology is rejected at input.
/// Nonfinite measurements mask their adjacent triangles.
#[derive(Debug, Clone)]
pub struct Mesh {
    samples: Vec<Sample>,
    faces: Vec<[usize; 3]>,
}

impl Mesh {
    pub fn new(samples: Vec<Sample>, faces: Vec<[usize; 3]>) -> Option<Self> {
        if faces.iter().any(|&[a, b, c]| {
            a >= samples.len() || b >= samples.len() || c >= samples.len() || a == b || b == c || a == c
        }) {
            return None;
        }
        Some(Self { samples, faces })
    }

    /// Regular rectilinear grid of vertex values in y-major order.
    pub fn grid(x: &[f64], y: &[f64], values: &[f64]) -> Option<Self> {
        if !axis(x) || !axis(y) || x.len().checked_mul(y.len())? != values.len() {
            return None;
        }
        let mut samples = Vec::with_capacity(values.len());
        for (j, &yy) in y.iter().enumerate() {
            for (i, &xx) in x.iter().enumerate() {
                samples.push(Sample {
                    x: xx,
                    y: yy,
                    value: values[j * x.len() + i],
                });
            }
        }
        let mut faces = Vec::new();
        for j in 0..y.len() - 1 {
            for i in 0..x.len() - 1 {
                let a = j * x.len() + i;
                faces.extend([[a, a + 1, a + x.len() + 1], [a, a + x.len() + 1, a + x.len()]]);
            }
        }
        Self::new(samples, faces)
    }

    fn triangles(&self) -> impl Iterator<Item = [Sample; 3]> + '_ {
        self.faces
            .iter()
            .map(|face| face.map(|i| self.samples[i]))
            .filter(|tri| tri.iter().all(|v| v.finite()))
    }

    /// Draw the mesh's unique triangle edges.
    pub fn wireframe(&self) -> Diagram {
        let mut d = Diagram::cartesian();
        let id = d.series("Mesh");
        let mut edges = std::collections::BTreeSet::new();
        for &[a, b, c] in &self.faces {
            if ![a, b, c].iter().all(|&i| self.samples[i].finite()) {
                continue;
            }
            for (p, q) in [(a, b), (b, c), (c, a)] {
                edges.insert((p.min(q), p.max(q)));
            }
        }
        for (a, b) in edges {
            d.path(
                id,
                vec![self.samples[a].position(), self.samples[b].position()],
                false,
                1.,
            );
        }
        d
    }

    /// Interpolated isolines over the piecewise linear triangular surface.
    pub fn contours(&self, levels: &[f64]) -> Diagram {
        let mut d = Diagram::cartesian();
        for &level in levels.iter().filter(|v| v.is_finite()) {
            let id = d.entry(&level.to_string(), level);
            for tri in self.triangles() {
                let mut hits = Vec::with_capacity(2);
                for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
                    if (a.value < level) != (b.value < level) {
                        hits.push(interpolate(a, b, level).position());
                    }
                }
                if hits.len() == 2 && hits[0] != hits[1] {
                    d.path(id, hits, false, 1.);
                }
            }
        }
        d.scalar_colors();
        d
    }

    /// Fill each adjacent pair of level boundaries. Values outside the
    /// specified range remain empty. The polygon boundary is interpolated
    /// with the same rule used for contour lines.
    pub fn contour_bands(&self, levels: &[f64]) -> Option<Diagram> {
        if !axis(levels) {
            return None;
        }
        let mut d = Diagram::cartesian();
        for (i, w) in levels.windows(2).enumerate() {
            let id = d.entry(&format!("{}–{}", w[0], w[1]), w[0].midpoint(w[1]));
            for tri in self.triangles() {
                if tri.iter().all(|p| p.value == w[1]) && i + 2 < levels.len() {
                    continue;
                }
                let above = clip_scalar(&tri, w[0], true);
                let band = clip_scalar(&above, w[1], false);
                if band.len() >= 3 {
                    d.path(id, band.into_iter().map(Sample::position).collect(), true, 1.);
                }
            }
        }
        d.scalar_colors();
        Some(d)
    }
}

fn axis(values: &[f64]) -> bool {
    values.len() >= 2 && values.iter().all(|v| v.is_finite()) && values.windows(2).all(|w| w[0] < w[1])
}

fn interpolate(a: Sample, b: Sample, level: f64) -> Sample {
    let t = (level - a.value) / (b.value - a.value);
    Sample {
        x: a.x + (b.x - a.x) * t,
        y: a.y + (b.y - a.y) * t,
        value: level,
    }
}

fn clip_scalar(points: &[Sample], level: f64, above: bool) -> Vec<Sample> {
    let Some(&mut_prev) = points.last() else {
        return Vec::new();
    };
    let mut prev = mut_prev;
    let mut out = Vec::new();
    let contains = |v: f64| if above { v >= level } else { v <= level };
    for &p in points {
        if contains(prev.value) != contains(p.value) {
            out.push(interpolate(prev, p, level));
        }
        if contains(p.value) {
            out.push(p);
        }
        prev = p;
    }
    out
}

/// Rectangular 2D histogram with independently chosen finite edges.
/// Density divides by included population and bin area.
pub fn histogram2d(x: &[f64], y: &[f64], samples: &[[f64; 2]], density: bool) -> Option<Diagram> {
    if !axis(x) || !axis(y) {
        return None;
    }
    let cols = x.len() - 1;
    let mut counts = vec![0usize; cols.checked_mul(y.len() - 1)?];
    let bin = |v: f64, edges: &[f64]| {
        if v >= edges[0] && v <= edges[edges.len() - 1] {
            Some((edges.partition_point(|&e| e <= v) - 1).min(edges.len() - 2))
        } else {
            None
        }
    };
    let mut n = 0;
    for &[xx, yy] in samples {
        if let (Some(i), Some(j)) = (bin(xx, x), bin(yy, y)) {
            counts[j * cols + i] += 1;
            n += 1;
        }
    }
    let values: Vec<_> = counts
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            if density {
                c as f64 / n.max(1) as f64 / (x[i % cols + 1] - x[i % cols]) / (y[i / cols + 1] - y[i / cols])
            } else {
                c as f64
            }
        })
        .collect();
    tiles(x, y, &values)
}

/// Arbitrarily spaced rectangular cells, colored by their scalar value.
/// Nonfinite measurements are masked. Equal values share one palette slot.
pub fn tiles(x: &[f64], y: &[f64], values: &[f64]) -> Option<Diagram> {
    if !axis(x) || !axis(y) || (x.len() - 1).checked_mul(y.len() - 1)? != values.len() {
        return None;
    }
    let mut levels: Vec<_> = values.iter().copied().filter(|v| v.is_finite()).collect();
    levels.sort_by(f64::total_cmp);
    levels.dedup_by(|a, b| a.total_cmp(b).is_eq());
    let mut d = Diagram::cartesian();
    for &v in &levels {
        d.entry(&v.to_string(), v);
    }
    for (j, ys) in y.windows(2).enumerate() {
        for (i, xs) in x.windows(2).enumerate() {
            let v = values[j * (x.len() - 1) + i];
            if let Ok(id) = levels.binary_search_by(|x| x.total_cmp(&v)) {
                d.path(
                    id,
                    vec![[xs[0], ys[0]], [xs[1], ys[0]], [xs[1], ys[1]], [xs[0], ys[1]]],
                    true,
                    1.,
                );
            }
        }
    }
    d.scalar_colors();
    Some(d)
}

/// Flat-topped hexagonal binning. Radius uses data units on both axes.
/// The origin is a cell center. Cube-coordinate rounding assigns boundary
/// points deterministically, including negative coordinates.
pub fn hexbin(points: &[[f64; 2]], radius: f64) -> Option<Diagram> {
    if !radius.is_finite() || radius <= 0. {
        return None;
    }
    let root3 = 3_f64.sqrt();
    let mut cells = std::collections::BTreeMap::new();
    for &[x, y] in points.iter().filter(|p| p.iter().all(|v| v.is_finite())) {
        let q = 2. / 3. * x / radius;
        let r = (-x / 3. + y / root3) / radius;
        let s = -q - r;
        if [q, r, s].iter().any(|v| v.abs() >= i64::MAX as f64) {
            continue;
        }
        let (mut rq, mut rr, rs) = (q.round(), r.round(), s.round());
        let (dq, dr, ds) = ((rq - q).abs(), (rr - r).abs(), (rs - s).abs());
        if dq > dr && dq > ds {
            rq = -rr - rs;
        } else if dr > ds {
            rr = -rq - rs;
        }
        *cells.entry((rq as i64, rr as i64)).or_insert(0usize) += 1;
    }
    let mut levels: Vec<_> = cells.values().copied().collect();
    levels.sort_unstable();
    levels.dedup();
    let mut d = Diagram::cartesian();
    for &n in &levels {
        d.entry(&n.to_string(), n as f64);
    }
    for ((q, r), n) in cells {
        let cx = radius * 1.5 * q as f64;
        let cy = radius * root3 * (r as f64 + 0.5 * q as f64);
        if let Ok(id) = levels.binary_search(&n) {
            d.path(
                id,
                (0..6)
                    .map(|i| {
                        let theta = i as f64 * std::f64::consts::TAU / 6.;
                        [cx + radius * theta.cos(), cy + radius * theta.sin()]
                    })
                    .collect(),
                true,
                1.,
            );
        }
    }
    d.scalar_colors();
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contour_intersections_match_plane() {
        let m = Mesh::grid(&[0., 1.], &[0., 1.], &[0., 1., 1., 2.]).expect("valid plane");
        let d = m.contours(&[0.75]);
        assert_eq!(d.shapes.len(), 2);
        for p in d.shapes.iter().flat_map(|s| &s.points) {
            assert!((p[0] + p[1] - 0.75).abs() < 1e-10);
        }
        assert!(m.contour_bands(&[0., 1., 2.]).is_some());
    }
    #[test]
    fn hexbin_and_histogram_include_extreme_edges() {
        let d = hexbin(&[[0., 0.], [0., 0.], [-5., 0.]], 1.).expect("valid radius");
        assert_eq!(d.entries().iter().map(|e| e.value).collect::<Vec<_>>(), [1., 2.]);
        let d = histogram2d(&[0., 1.], &[0., 1., 3.], &[[0., 1.], [1., 3.], [2., 1.]], true).expect("valid axes");
        assert_eq!(d.entries().iter().map(|e| e.value).collect::<Vec<_>>(), [0., 0.5]);
    }
}

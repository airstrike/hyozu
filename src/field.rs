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
            for band in self.band(w, i + 2 == levels.len()) {
                d.path(id, band.into_iter().map(Sample::position).collect(), true, 1.);
            }
        }
        d.scalar_colors();
        Some(d)
    }
    fn band<'a>(&'a self, w: &'a [f64], last: bool) -> impl Iterator<Item = Vec<Sample>> + 'a {
        self.triangles()
            .filter(move |tri| last || !tri.iter().all(|p| p.value == w[1]))
            .filter_map(move |tri| {
                let above = clip_scalar(&tri, w[0], true);
                let band = clip_scalar(&above, w[1], false);
                (band.len() >= 3).then_some(band)
            })
    }

    /// A true 3D surface colored by value intervals. Triangles crossing a
    /// boundary are split exactly where they cross its z coordinate.
    pub fn banded_surface(&self, levels: &[f64]) -> Option<crate::spatial::Scene> {
        if !axis(levels) {
            return None;
        }
        let mut scene = crate::spatial::Scene::default();
        for (i, w) in levels.windows(2).enumerate() {
            let id = scene.series(format!("{}–{}", w[0], w[1]));
            for band in self.band(w, i + 2 == levels.len()) {
                scene.polygon(id, &band.into_iter().map(|s| [s.x, s.y, s.value]).collect::<Vec<_>>());
            }
        }
        Some(scene)
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
    hex_cells(
        points.iter().map(|&[x, y]| [x, y, 1.]),
        radius,
        |cell: &mut usize, _| *cell += 1,
        |n| n as f64,
    )
}

/// Reduce finite scalar observations inside each occupied hexagon. For
/// example a caller can supply a mean, sum, median or maximum.
pub fn hexbin_values(points: &[[f64; 3]], radius: f64, reduce: impl Fn(&[f64]) -> f64) -> Option<Diagram> {
    hex_cells(
        points.iter().copied(),
        radius,
        |cell: &mut Vec<f64>, z| cell.push(z),
        |cell| reduce(&cell),
    )
}

fn hex_cells<T: Default>(
    points: impl Iterator<Item = [f64; 3]>,
    radius: f64,
    mut insert: impl FnMut(&mut T, f64),
    reduce: impl Fn(T) -> f64,
) -> Option<Diagram> {
    if !radius.is_finite() || radius <= 0. {
        return None;
    }
    let root3 = 3_f64.sqrt();
    let mut cells = std::collections::BTreeMap::new();
    for [x, y, z] in points.filter(|p| p.iter().all(|v| v.is_finite())) {
        let q = 2. / 3. * x / radius;
        let r = (-x / 3. + y / root3) / radius;
        let s = -q - r;
        if [q, r, s].iter().any(|v| !v.is_finite() || v.abs() >= i64::MAX as f64) {
            continue;
        }
        let (mut rq, mut rr, rs) = (q.round(), r.round(), s.round());
        let (dq, dr, ds) = ((rq - q).abs(), (rr - r).abs(), (rs - s).abs());
        if dq > dr && dq > ds {
            rq = -rr - rs;
        } else if dr > ds {
            rr = -rq - rs;
        }
        insert(cells.entry((rq as i64, rr as i64)).or_default(), z);
    }
    let faces = cells
        .into_iter()
        .map(|((q, r), cell)| {
            let cx = radius * 1.5 * q as f64;
            let cy = radius * root3 * (r as f64 + 0.5 * q as f64);
            let corners = (0..6)
                .map(|i| {
                    let angle = f64::from(i) * std::f64::consts::TAU / 6.;
                    [cx + radius * angle.cos(), cy + radius * angle.sin()]
                })
                .collect();
            (reduce(cell), corners)
        })
        .collect();
    Some(scalar_faces(faces))
}

/// Color arbitrary quadrilateral mesh cells by measured scalar. Vertices
/// are ordered by y row, then x column. Any nonfinite corner masks its cell.
pub fn quad_mesh(columns: usize, rows: usize, vertices: &[[f64; 2]], values: &[f64]) -> Option<Diagram> {
    if rows < 2
        || columns < 2
        || rows.checked_mul(columns)? != vertices.len()
        || (rows - 1).checked_mul(columns - 1)? != values.len()
    {
        return None;
    }
    let faces: Vec<_> = values
        .iter()
        .enumerate()
        .map(|(i, &value)| {
            let a = i / (columns - 1) * columns + i % (columns - 1);
            (
                value,
                [
                    vertices[a],
                    vertices[a + 1],
                    vertices[a + columns + 1],
                    vertices[a + columns],
                ]
                .to_vec(),
            )
        })
        .collect();
    Some(scalar_faces(faces))
}

fn scalar_faces(mut faces: Vec<(f64, Vec<[f64; 2]>)>) -> Diagram {
    faces.retain(|(value, vertices)| value.is_finite() && vertices.iter().flatten().all(|v| v.is_finite()));
    let mut levels: Vec<_> = faces.iter().map(|p| p.0).filter(|v| v.is_finite()).collect();
    levels.sort_by(f64::total_cmp);
    levels.dedup_by(|a, b| a.total_cmp(b).is_eq());
    let mut d = Diagram::cartesian();
    for &v in &levels {
        d.entry(&v.to_string(), v);
    }
    for (v, vertices) in faces {
        if let Ok(slot) = levels.binary_search_by(|x| x.total_cmp(&v)) {
            d.path(slot, vertices, true, 1.);
        }
    }
    d.scalar_colors();
    d
}

impl Mesh {
    /// Flat triangle coloring by the mean of the three vertex values.
    pub fn face_colors(&self) -> Diagram {
        scalar_faces(
            self.triangles()
                .map(|tri| {
                    (
                        tri.iter().map(|p| p.value / 3.).sum(),
                        tri.into_iter().map(Sample::position).collect(),
                    )
                })
                .collect(),
        )
    }

    /// Product-kernel bivariate Gaussian density at each grid vertex.
    /// The two bandwidths use the x and y coordinate units.
    pub fn density(x: &[f64], y: &[f64], points: &[[f64; 2]], bandwidth: [f64; 2]) -> Option<Self> {
        if !axis(x) || !axis(y) || bandwidth.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return None;
        }
        let points: Vec<_> = points.iter().filter(|p| p.iter().all(|v| v.is_finite())).collect();
        if points.is_empty() {
            return None;
        }
        let denom = points.len() as f64 * std::f64::consts::TAU * bandwidth[0] * bandwidth[1];
        let mut z = Vec::with_capacity(x.len().checked_mul(y.len())?);
        for &yy in y {
            for &xx in x {
                z.push(
                    points
                        .iter()
                        .map(|p| {
                            let dx = (xx - p[0]) / bandwidth[0];
                            let dy = (yy - p[1]) / bandwidth[1];
                            (-0.5 * (dx * dx + dy * dy)).exp()
                        })
                        .sum::<f64>()
                        / denom,
                );
            }
        }
        Self::grid(x, y, &z)
    }
}

/// An RGBA8 image in Cartesian coordinates, row zero at the top. The
/// renderer uploads a single texture and uses the requested interpolation.
#[cfg(feature = "raster")]
pub fn image(
    width: u32,
    height: u32,
    rgba: Vec<u8>,
    x: [f64; 2],
    y: [f64; 2],
    filter: crate::core::image::FilterMethod,
) -> Option<Diagram> {
    if width == 0
        || height == 0
        || (width as usize).checked_mul(height as usize)?.checked_mul(4)? != rgba.len()
        || !axis(&x)
        || !axis(&y)
    {
        return None;
    }
    let mut d = Diagram::cartesian();
    d.image = Some(
        crate::core::image::Image::new(crate::core::image::Handle::from_rgba(width, height, rgba))
            .filter_method(filter),
    );
    d.range = Some(crate::data::area::Bounds {
        x_min: x[0],
        x_max: x[1],
        y_min: y[0],
        y_max: y[1],
    });
    Some(d)
}

/// Color a scalar image through a caller-supplied normalized color map.
/// Nonfinite samples are transparent. Row zero is the upper image edge.
#[cfg(feature = "raster")]
pub fn scalar_image(
    dimensions: [u32; 2],
    values: &[f64],
    extent: [[f64; 2]; 2],
    limits: [f64; 2],
    filter: crate::core::image::FilterMethod,
    color: impl Fn(f64) -> [u8; 4],
) -> Option<Diagram> {
    let [w, h] = dimensions;
    let [lo, hi] = limits;
    if (w as usize).checked_mul(h as usize)? != values.len() || !axis(&limits) {
        return None;
    }
    let rgba = values
        .iter()
        .flat_map(|&v| {
            if v.is_finite() {
                color(((v - lo) / (hi - lo)).clamp(0., 1.))
            } else {
                [0; 4]
            }
        })
        .collect();
    image(w, h, rgba, extent[0], extent[1], filter)
}

#[cfg(test)]
mod mesh_tests {
    use super::*;
    #[test]
    fn hexagons_reduce_measurements_separately_and_skip_missing_values() {
        let d = hexbin_values(
            &[[0., 0., 1.], [0., 0., 5.], [3., 0., 12.], [0., 0., f64::NAN]],
            0.5,
            |v| v.iter().sum::<f64>() / v.len() as f64,
        )
        .expect("hex means");
        assert_eq!(d.entries().iter().map(|e| e.value).collect::<Vec<_>>(), [3., 12.]);
        assert_eq!(d.shapes.len(), 2);
    }
    #[test]
    fn density_matches_an_analytic_gaussian_and_repeated_samples() {
        let m = Mesh::density(&[0., 1.], &[0., 1.], &[[0., 0.], [0., 0.], [f64::NAN, 2.]], [1., 2.]).expect("Gaussian");
        let normal = 1. / (std::f64::consts::TAU * 2.);
        assert!((m.samples[0].value - normal).abs() < 1e-12);
        assert!((m.samples[1].value - normal * (-0.5_f64).exp()).abs() < 1e-12);
        assert!(Mesh::density(&[0., 1.], &[0., 1.], &[], [1., 2.]).is_none());
    }
    #[test]
    fn warped_corners_and_flat_triangle_means_are_preserved() {
        let p = [[0., 0.], [2., 0.], [-1., 1.], [3., 1.]];
        let q = quad_mesh(2, 2, &p, &[7.]).expect("one quad");
        assert_eq!(q.shapes[0].points, [p[0], p[1], p[3], p[2]]);
        assert_eq!(q.entries()[0].value, 7.);
        assert!(quad_mesh(3, 2, &p, &[7.]).is_none());
        let m = Mesh::grid(&[0., 1.], &[0., 1.], &[0., 0., 0., 6.]).expect("two faces");
        let c = m.face_colors();
        assert_eq!(c.shapes.len(), 2);
        assert!(c.entries().iter().all(|e| e.value == 2. || e.value == 0.));
    }
    #[cfg(feature = "raster")]
    #[test]
    fn rgba_validates_row_bytes_and_keeps_real_extent() {
        use crate::core::image::FilterMethod;
        assert!(image(2, 1, vec![0; 7], [0., 1.], [0., 1.], FilterMethod::Nearest).is_none());
        let i = image(2, 1, vec![0; 8], [-2., 1.], [4., 8.], FilterMethod::Linear).expect("RGBA image");
        let b = i.bounds().expect("coordinate extent");
        assert_eq!((b.x_min, b.x_max, b.y_min, b.y_max), (-2., 1., 4., 8.));
        assert!(i.image.is_some());
        assert!(i.shapes.is_empty());
    }
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

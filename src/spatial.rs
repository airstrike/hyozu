//! Three-dimensional charts projected into the native chart scene.
//! Geometry stays in world coordinates until a camera is selected.
use crate::Diagram;
use crate::field::Sample;
use std::collections::BTreeSet;

pub type Point = [f64; 3];

/// Camera angles in degrees. `distance` is measured in normalized plot radii.
/// A missing distance selects an orthographic projection.
#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub azimuth: f64,
    pub elevation: f64,
    pub distance: Option<f64>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            azimuth: -52.,
            elevation: 27.,
            distance: None,
        }
    }
}

#[derive(Clone)]
enum Geometry {
    Line(Vec<Point>),
    Face(Vec<Point>),
    Dot(Point),
}

impl Geometry {
    fn points(&self) -> &[Point] {
        match self {
            Self::Line(p) | Self::Face(p) => p,
            Self::Dot(p) => std::slice::from_ref(p),
        }
    }
}

#[derive(Default, Clone)]
pub struct Scene {
    names: Vec<String>,
    aspect: Option<[f64; 3]>,
    hide_axes: bool,
    shapes: Vec<(usize, Geometry)>,
}

impl Scene {
    pub fn series(&mut self, name: impl Into<String>) -> usize {
        self.names.push(name.into());
        self.names.len() - 1
    }

    /// Splits at nonfinite points to preserve missing-data gaps.
    pub fn line(&mut self, series: usize, points: &[Point]) {
        if series >= self.names.len() {
            return;
        }
        for run in points
            .split(|p| p.iter().any(|v| !v.is_finite()))
            .filter(|r| r.len() > 1)
        {
            self.shapes.push((series, Geometry::Line(run.to_vec())));
        }
    }

    pub fn scatter(&mut self, series: usize, points: &[Point]) {
        if series >= self.names.len() {
            return;
        }
        self.shapes.extend(
            points
                .iter()
                .filter(|p| p.iter().all(|v| v.is_finite()))
                .map(|&p| (series, Geometry::Dot(p))),
        );
    }

    pub fn polygon(&mut self, series: usize, points: &[Point]) {
        if series < self.names.len() && points.len() >= 3 && points.iter().flatten().all(|v| v.is_finite()) {
            self.shapes.push((series, Geometry::Face(points.to_vec())));
        }
    }

    /// A rectangular prism between two corners, suitable for bars or voxels.
    pub fn cuboid(&mut self, series: usize, a: Point, b: Point) {
        let corners = [
            [a[0], a[1], a[2]],
            [b[0], a[1], a[2]],
            [b[0], b[1], a[2]],
            [a[0], b[1], a[2]],
            [a[0], a[1], b[2]],
            [b[0], a[1], b[2]],
            [b[0], b[1], b[2]],
            [a[0], b[1], b[2]],
        ];
        for face in [[0, 1, 2, 3], [4, 5, 6, 7], [0, 1, 5, 4], [1, 2, 6, 5], [2, 3, 7, 6], [
            3, 0, 4, 7,
        ]] {
            self.polygon(series, &face.map(|i| corners[i]));
        }
    }

    /// Build a triangular surface or its edge mesh from arbitrary 3D samples.
    /// A triangle touching a nonfinite vertex is masked.
    pub fn mesh(&mut self, series: usize, vertices: &[Sample], triangles: &[[usize; 3]], wireframe: bool) -> bool {
        if series >= self.names.len() || triangles.iter().flatten().any(|&i| i >= vertices.len()) {
            return false;
        }
        let mut drawn = BTreeSet::new();
        for face in triangles {
            let p = face.map(|i| [vertices[i].x, vertices[i].y, vertices[i].value]);
            if p.iter().flatten().any(|v| !v.is_finite()) {
                continue;
            }
            if wireframe {
                for [a, b] in [[0, 1], [1, 2], [2, 0]] {
                    let (m, n) = (face[a], face[b]);
                    if drawn.insert((m.min(n), m.max(n))) {
                        self.line(series, &[p[a], p[b]]);
                    }
                }
            } else {
                self.polygon(series, &p);
            }
        }
        true
    }

    /// A gridded surface triangulated across each cell.
    pub fn surface(&mut self, series: usize, x: &[f64], y: &[f64], z: &[f64], wireframe: bool) -> bool {
        if x.len() < 2 || y.len() < 2 || x.len().checked_mul(y.len()) != Some(z.len()) {
            return false;
        }
        let vertices: Vec<_> = z
            .iter()
            .enumerate()
            .map(|(i, &value)| Sample {
                x: x[i % x.len()],
                y: y[i / x.len()],
                value,
            })
            .collect();
        let mut faces = Vec::new();
        for j in 0..y.len() - 1 {
            for i in 0..x.len() - 1 {
                let a = j * x.len() + i;
                faces.extend([[a, a + 1, a + x.len() + 1], [a, a + x.len() + 1, a + x.len()]]);
            }
        }
        self.mesh(series, &vertices, &faces, wireframe)
    }

    /// Fit the scene to the plot, then rotate and project it. Painter ordering
    /// uses camera-space depth per primitive. Empty scenes render empty.
    pub fn project(&self, camera: Camera) -> Option<Diagram> {
        if !camera.azimuth.is_finite()
            || !camera.elevation.is_finite()
            || camera.distance.is_some_and(|v| !v.is_finite() || v <= 2.)
        {
            return None;
        }
        let mut d = Diagram {
            square: true,
            smooth_faces: true,
            ..Diagram::default()
        };
        for name in &self.names {
            d.series(name);
        }
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for &p in self.shapes.iter().flat_map(|(_, s)| s.points()) {
            for i in 0..3 {
                min[i] = min[i].min(p[i]);
                max[i] = max[i].max(p[i]);
            }
        }
        if !min[0].is_finite() {
            return Some(d);
        }
        let (sa, ca) = camera.azimuth.to_radians().sin_cos();
        let (se, ce) = camera.elevation.to_radians().sin_cos();
        let transform = |p: Point| {
            let mut coordinates: Point = std::array::from_fn(|i| {
                if max[i] > min[i] {
                    (p[i] - min[i]) / (max[i] - min[i]) * 2. - 1.
                } else {
                    0.
                }
            });
            let aspect = self.aspect.unwrap_or([1.; 3]);
            for i in 0..3 {
                coordinates[i] *= aspect[i];
            }
            let [x, y, z] = coordinates;
            let u = x * ca + y * sa;
            let depth = (-x * sa + y * ca) * ce + z * se;
            let v = z * ce - (-x * sa + y * ca) * se;
            let perspective = camera.distance.map_or(1., |dist| dist / (dist - depth));
            [u * perspective, v * perspective, depth]
        };
        let mut corners = Vec::new();
        for x in [min[0], max[0]] {
            for y in [min[1], max[1]] {
                for z in [min[2], max[2]] {
                    corners.push(transform([x, y, z]));
                }
            }
        }
        let extent = if self.hide_axes {
            self.shapes
                .iter()
                .flat_map(|(_, shape)| shape.points())
                .copied()
                .map(transform)
                .map(|p| p[0].abs().max(p[1].abs()))
                .fold(1e-12, f64::max)
        } else {
            corners.iter().map(|p| p[0].abs().max(p[1].abs())).fold(1e-12, f64::max)
        };
        let screen = |p: Point| [0.5 + 0.43 * p[0] / extent, 0.50 - 0.43 * p[1] / extent];
        // Axes originate at the lower corner in world coordinates.
        if !self.hide_axes {
            for i in 0..3 {
                let mut end = min;
                end[i] = max[i];
                let a = screen(transform(min));
                let b = screen(transform(end));
                d.guide(vec![a.map(|v| v as f32), b.map(|v| v as f32)]);
                d.labels.push((b.map(|v| v as f32), ["X", "Y", "Z"][i].to_owned()));
            }
        }
        let mut order: Vec<_> = self
            .shapes
            .iter()
            .map(|(id, s)| {
                let p: Vec<_> = s.points().iter().copied().map(transform).collect();
                let depth = p.iter().map(|v| v[2]).sum::<f64>() / p.len() as f64;
                (depth, *id, s, p)
            })
            .collect();
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, id, s, p) in order {
            match s {
                Geometry::Dot(_) => {
                    let [x, y] = screen(p[0]);
                    d.path(
                        id,
                        (0..16)
                            .map(|i| {
                                let a = f64::from(i) * std::f64::consts::TAU / 16.;
                                [x + 0.006 * a.cos(), y + 0.006 * a.sin()]
                            })
                            .collect(),
                        true,
                        1.,
                    );
                }
                Geometry::Line(_) => d.path(id, p.into_iter().map(screen).collect(), false, 1.),
                Geometry::Face(_) => {
                    let u = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
                    let v = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
                    let normal = [
                        u[1] * v[2] - u[2] * v[1],
                        u[2] * v[0] - u[0] * v[2],
                        u[0] * v[1] - u[1] * v[0],
                    ];
                    let length = normal[0].hypot(normal[1]).hypot(normal[2]);
                    let shade = if length > 0. {
                        ((0.3 * normal[0] + 0.4 * normal[1] + 0.86 * normal[2]) / length).abs()
                    } else {
                        0.
                    };
                    d.path(id, p.into_iter().map(screen).collect(), true, 1.);
                    if let Some(shape) = d.shapes.last_mut() {
                        shape.lightness = (0.55 + 0.45 * shade) as f32;
                    }
                }
            }
        }
        Some(d)
    }
}

impl Scene {
    /// Set the projected box's relative axis lengths.
    pub fn aspect(&mut self, aspect: [f64; 3]) -> bool {
        if aspect.iter().any(|v| !v.is_finite() || *v <= 0.) {
            return false;
        }
        let max = aspect.into_iter().fold(0., f64::max);
        self.aspect = Some(aspect.map(|v| v / max));
        true
    }

    pub fn show_axes(&mut self, show: bool) {
        self.hide_axes = !show;
    }

    /// One quadrilateral surface strip between successive pairs of traces.
    pub fn fill_between(&mut self, id: usize, a: &[Point], b: &[Point]) -> bool {
        if a.len() != b.len() || id >= self.names.len() {
            return false;
        }
        for (x, y) in a.windows(2).zip(b.windows(2)) {
            self.polygon(id, &[x[0], x[1], y[1], y[0]]);
        }
        true
    }

    /// Stems parallel to the z axis, with a point at each measured endpoint.
    pub fn stems(&mut self, id: usize, points: &[Point], baseline: f64) {
        if !baseline.is_finite() {
            return;
        }
        for &p in points {
            self.line(id, &[[p[0], p[1], baseline], p]);
        }
        self.scatter(id, points);
    }

    /// Oriented 3D arrow with a cone head defined in world coordinates.
    pub fn arrow(&mut self, id: usize, origin: Point, delta: Point) {
        let length = delta[0].hypot(delta[1]).hypot(delta[2]);
        if !length.is_finite() || length == 0. {
            return;
        }
        let dir = delta.map(|x| x / length);
        let cross = |a: Point, b: Point| {
            [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ]
        };
        let perp = cross(
            dir,
            if dir[2].abs() < 0.85 {
                [0., 0., 1.]
            } else {
                [1., 0., 0.]
            },
        );
        let norm = perp[0].hypot(perp[1]).hypot(perp[2]);
        let u = perp.map(|v| v / norm);
        let v = cross(dir, u);
        let tip = std::array::from_fn(|i| origin[i] + delta[i]);
        let base: Point = std::array::from_fn(|i| origin[i] + 0.76 * delta[i]);
        self.line(id, &[origin, base]);
        let ring = |n: usize| {
            let a = n as f64 * std::f64::consts::TAU / 8.;
            std::array::from_fn(|i| base[i] + 0.065 * length * (a.cos() * u[i] + a.sin() * v[i]))
        };
        for i in 0..8 {
            self.polygon(id, &[tip, ring(i), ring(i + 1)]);
        }
    }
}

/// Extruded pie segments; each input supplies the name, weight and radial
/// explosion distance as a fraction of radius. Thickness is in radius units.
pub fn pie<S: AsRef<str>>(slices: &[(S, f64, f64)], thickness: f64) -> Option<Scene> {
    if !thickness.is_finite()
        || thickness <= 0.
        || slices
            .iter()
            .any(|(_, v, e)| !v.is_finite() || *v < 0. || !e.is_finite() || *e < 0.)
    {
        return None;
    }
    let total: f64 = slices.iter().map(|s| s.1).sum();
    if !total.is_finite() || total <= 0. {
        return None;
    }
    let mut scene = Scene::default();
    scene.show_axes(false);
    scene.aspect([1., 1., thickness / 2.]);
    let mut angle = 0.;
    for (name, value, offset) in slices {
        let id = scene.series(name.as_ref());
        if *value == 0. {
            continue;
        }
        let arc = value / total * std::f64::consts::TAU;
        let mid = angle + arc / 2.;
        let (cx, cy) = (offset * mid.cos(), offset * mid.sin());
        let n = (arc * 24.).ceil().max(2.) as usize;
        let point = |i: usize, z| {
            let a = angle + arc * i as f64 / n as f64;
            [cx + a.cos(), cy + a.sin(), z]
        };
        for i in 0..n {
            scene.polygon(id, &[[cx, cy, thickness], point(i, thickness), point(i + 1, thickness)]);
            scene.polygon(id, &[
                point(i, 0.),
                point(i + 1, 0.),
                point(i + 1, thickness),
                point(i, thickness),
            ]);
            scene.polygon(id, &[[cx, cy, 0.], point(i + 1, 0.), point(i, 0.)]);
        }
        for edge in [0, n] {
            scene.polygon(id, &[[cx, cy, 0.], point(edge, 0.), point(edge, thickness), [
                cx, cy, thickness,
            ]]);
        }
        angle += arc;
    }
    Some(scene)
}

#[derive(Clone, Copy, Debug)]
pub enum CategoryMark {
    Column,
    Bar,
    Line,
    Area,
}
#[derive(Clone, Copy, Debug)]
pub enum Stack {
    Grouped,
    Sum,
    Percent,
}

/// Office-style category charts with group depth, cumulative stacks or
/// normalized stacks. Positive and negative contributions stack separately.
pub fn categories<S: AsRef<str>, V: AsRef<[f64]>>(
    series: &[(S, V)],
    mark: CategoryMark,
    stack: Stack,
) -> Option<Scene> {
    let n = series.first()?.1.as_ref().len();
    if n == 0
        || series
            .iter()
            .any(|s| s.1.as_ref().len() != n || s.1.as_ref().iter().any(|v| !v.is_finite()))
    {
        return None;
    }
    let mut pos = vec![0.; n];
    let mut neg = vec![0.; n];
    let totals: Vec<f64> = (0..n)
        .map(|i| series.iter().map(|s| s.1.as_ref()[i].abs()).sum())
        .collect();
    if totals.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mut scene = Scene::default();
    if !matches!(stack, Stack::Grouped) {
        scene.aspect([1., 0.25, 1.]);
    }
    for (j, (name, values)) in series.iter().enumerate() {
        let id = scene.series(name.as_ref());
        let y = if matches!(stack, Stack::Grouped) { j as f64 } else { 0. };
        let mut lower = Vec::new();
        let mut upper = Vec::new();
        for (i, &value) in values.as_ref().iter().enumerate() {
            let x = i as f64;
            let value = if matches!(stack, Stack::Percent) {
                if totals[i] > 0. { value / totals[i] } else { 0. }
            } else {
                value
            };
            let base = if matches!(stack, Stack::Grouped) {
                0.
            } else if value >= 0. {
                pos[i]
            } else {
                neg[i]
            };
            let end = base + value;
            if !end.is_finite() {
                return None;
            }
            if value >= 0. {
                pos[i] = end;
            } else {
                neg[i] = end;
            }
            let (lo, hi) = (base.min(end), base.max(end));
            match mark {
                CategoryMark::Column => scene.cuboid(id, [x - 0.32, y - 0.3, lo], [x + 0.32, y + 0.3, hi]),
                CategoryMark::Bar => scene.cuboid(id, [lo, y - 0.3, x - 0.32], [hi, y + 0.3, x + 0.32]),
                CategoryMark::Line => {
                    lower.push([x, y - 0.09, end]);
                    upper.push([x, y + 0.09, end]);
                }
                CategoryMark::Area => {
                    lower.push([x, y, base]);
                    upper.push([x, y, end]);
                }
            }
        }
        scene.fill_between(id, &lower, &upper);
    }
    Some(scene)
}

#[cfg(test)]
mod category_tests {
    use super::*;
    #[test]
    fn world_space_geometry_and_percent_stacks_have_correct_extent() {
        let p = pie(&[("a", 3., 0.), ("b", 1., 0.2)], 0.25).expect("pie");
        assert_eq!(p.names.len(), 2);
        assert!(p.hide_axes);
        assert!(
            p.shapes
                .iter()
                .flat_map(|(_, g)| g.points())
                .all(|p| p[2] == 0. || p[2] == 0.25)
        );
        let bars = categories(
            &[("a", [1., 0.]), ("b", [3., 0.])],
            CategoryMark::Column,
            Stack::Percent,
        )
        .expect("stacks");
        let extent = |id| {
            bars.shapes
                .iter()
                .filter(|(i, _)| *i == id)
                .flat_map(|(_, g)| g.points())
                .map(|p| p[2])
                .fold(0., f64::max)
        };
        assert_eq!(extent(0), 0.25);
        assert_eq!(extent(1), 1.);
        assert!(
            categories(
                &[("a", vec![2.]), ("b", vec![3., 1.])],
                CategoryMark::Bar,
                Stack::Grouped
            )
            .is_none()
        );
    }
    #[test]
    fn arrows_ribbons_and_stems_keep_their_endpoints() {
        let mut s = Scene::default();
        let id = s.series("v");
        s.arrow(id, [1., 2., 3.], [2., -1., 4.]);
        assert!(
            s.shapes
                .iter()
                .flat_map(|(_, g)| g.points())
                .any(|&p| p == [3., 1., 7.])
        );
        s.stems(id, &[[0., 1., 4.]], -3.);
        assert!(
            s.shapes
                .iter()
                .flat_map(|(_, g)| g.points())
                .any(|&p| p == [0., 1., -3.])
        );
        assert!(!s.fill_between(id, &[[0.; 3]], &[]));
        assert!(s.project(Camera::default()).is_some());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rotating_changes_projected_geometry_and_surfaces_fit() {
        let mut s = Scene::default();
        let id = s.series("Test");
        assert!(s.surface(id, &[0., 1.], &[0., 1.], &[1., 2., 4., 1.], false));
        let first = s.project(Camera::default()).expect("valid camera");
        let second = s
            .project(Camera {
                azimuth: 45.,
                ..Camera::default()
            })
            .expect("valid rotation");
        assert_ne!(first.shapes[3].points, second.shapes[3].points);
        assert!(
            second
                .shapes
                .iter()
                .flat_map(|s| &s.points)
                .flatten()
                .all(|v| (0.0..=1.).contains(v))
        );
        assert!(
            s.project(Camera {
                distance: Some(1.),
                ..Camera::default()
            })
            .is_none()
        );
    }
}

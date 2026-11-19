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
            let [x, y, z] = std::array::from_fn(|i| {
                if max[i] > min[i] {
                    (p[i] - min[i]) / (max[i] - min[i]) * 2. - 1.
                } else {
                    0.
                }
            });
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
        let extent = corners.iter().map(|p| p[0].abs().max(p[1].abs())).fold(1e-12, f64::max);
        let screen = |p: Point| [0.5 + 0.43 * p[0] / extent, 0.50 - 0.43 * p[1] / extent];
        // Axes originate at the lower corner in world coordinates.
        for i in 0..3 {
            let mut end = min;
            end[i] = max[i];
            let a = screen(transform(min));
            let b = screen(transform(end));
            d.guide(vec![a.map(|v| v as f32), b.map(|v| v as f32)]);
            d.labels.push((b.map(|v| v as f32), ["X", "Y", "Z"][i].to_owned()));
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

//! Two-dimensional vector fields. Vector components use the same data units as their axes.
use crate::Diagram;

#[derive(Clone, Copy, Debug)]
pub struct Vector {
    pub x: f64,
    pub y: f64,
    pub u: f64,
    pub v: f64,
}

impl Vector {
    fn valid(self) -> bool {
        [self.x, self.y, self.u, self.v].iter().all(|v| v.is_finite())
    }
}

/// Arrows from each sample origin to origin + scale × vector.
pub fn quiver(vectors: &[Vector], scale: f64) -> Diagram {
    let mut d = Diagram::cartesian();
    let id = d.series("Vector field");
    if !scale.is_finite() {
        return d;
    }
    for &v in vectors.iter().filter(|v| v.valid()) {
        let (dx, dy) = (v.u * scale, v.v * scale);
        let tip = [v.x + dx, v.y + dy];
        d.path(id, vec![[v.x, v.y], tip], false, 1.);
        d.path(
            id,
            vec![tip, [tip[0] - 0.28 * dx - 0.13 * dy, tip[1] - 0.28 * dy + 0.13 * dx], [
                tip[0] - 0.28 * dx + 0.13 * dy,
                tip[1] - 0.28 * dy - 0.13 * dx,
            ]],
            true,
            0.9,
        );
    }
    d
}

/// Wind barbs using pennants for 50, full feathers for 10 and half feathers
/// for 5 units. Magnitudes round to the nearest 5. Shafts point upwind.
pub fn wind_barbs(vectors: &[Vector], shaft: f64) -> Diagram {
    let mut d = Diagram::cartesian();
    let id = d.series("Wind");
    if !shaft.is_finite() || shaft <= 0. {
        return d;
    }
    for &v in vectors.iter().filter(|v| v.valid()) {
        let speed = v.u.hypot(v.v);
        let units = (speed / 5.).round();
        if !units.is_finite() {
            continue;
        }
        if units < 1. {
            let circle = (0..=20)
                .map(|i| {
                    let a = f64::from(i) * std::f64::consts::TAU / 20.;
                    [v.x + shaft * 0.1 * a.cos(), v.y + shaft * 0.1 * a.sin()]
                })
                .collect();
            d.path(id, circle, false, 1.);
            continue;
        }
        let (dx, dy) = (-v.u / speed * shaft, -v.v / speed * shaft);
        let pos = |a: f64, p: f64| [v.x + a * dx - p * dy, v.y + a * dy + p * dx];
        d.path(id, vec![pos(0., 0.), pos(1., 0.)], false, 1.);
        let flags = (units / 10.).floor() as usize;
        let full = ((units % 10.) / 2.).floor() as usize;
        let half = (units % 2.) >= 1.;
        // Compress spacing for dense barbs. The finite visual resolution
        // bounds the number of distinct flags that can be represented.
        let flags = flags.min(64);
        let spacing = 0.72 / (flags + full + usize::from(half)).max(5) as f64;
        let mut a = 1.;
        for _ in 0..flags {
            d.path(
                id,
                vec![pos(a, 0.), pos(a + spacing, 0.32), pos(a - spacing, 0.)],
                true,
                1.,
            );
            a -= spacing;
        }
        for _ in 0..full {
            d.path(id, vec![pos(a, 0.), pos(a + spacing, 0.32)], false, 1.);
            a -= spacing;
        }
        if half {
            d.path(id, vec![pos(a, 0.), pos(a + spacing / 2., 0.16)], false, 1.);
        }
    }
    d
}

/// Rectilinear grid of vectors with bilinear interpolation. Missing or nonfinite
/// components mask their cells. Components are supplied in row-major order.
pub struct Grid {
    x: Vec<f64>,
    y: Vec<f64>,
    uv: Vec<[f64; 2]>,
}

impl Grid {
    pub fn new(x: Vec<f64>, y: Vec<f64>, uv: Vec<[f64; 2]>) -> Option<Self> {
        let valid = |axis: &[f64]| {
            axis.len() >= 2 && axis.iter().all(|v| v.is_finite()) && axis.windows(2).all(|w| w[0] < w[1])
        };
        (valid(&x) && valid(&y) && x.len().checked_mul(y.len())? == uv.len()).then_some(Self { x, y, uv })
    }

    pub fn at(&self, [x, y]: [f64; 2]) -> Option<[f64; 2]> {
        let cell = |axis: &[f64], v: f64| {
            if !(v >= axis[0] && v <= axis[axis.len() - 1]) {
                return None;
            }
            let i = (axis.partition_point(|&s| s <= v) - 1).min(axis.len() - 2);
            Some((i, (v - axis[i]) / (axis[i + 1] - axis[i])))
        };
        let (i, t) = cell(&self.x, x)?;
        let (j, s) = cell(&self.y, y)?;
        let k = j * self.x.len() + i;
        let [a, b, c, d] = [
            self.uv[k],
            self.uv[k + 1],
            self.uv[k + self.x.len()],
            self.uv[k + self.x.len() + 1],
        ];
        let result =
            std::array::from_fn(|n| (a[n] * (1. - t) + b[n] * t) * (1. - s) + (c[n] * (1. - t) + d[n] * t) * s);
        result.iter().all(|x| x.is_finite()).then_some(result)
    }

    /// Trace both directions from each seed with a fourth-order Runge–Kutta
    /// step of constant arc length. A trajectory stops at a boundary, masked
    /// cell, stationary point, closed orbit or its specified step budget.
    pub fn streamlines(&self, seeds: &[[f64; 2]], step: f64, max_steps: usize) -> Diagram {
        let mut d = Diagram::cartesian();
        let id = d.series("Streamlines");
        if !step.is_finite() || step <= 0. {
            return d;
        }
        for &seed in seeds {
            let mut back = self.trace(seed, -step, max_steps);
            back.reverse();
            back.push(seed);
            back.extend(self.trace(seed, step, max_steps));
            d.path(id, back, false, 1.);
        }
        d
    }

    fn trace(&self, seed: [f64; 2], h: f64, budget: usize) -> Vec<[f64; 2]> {
        let direction = |p| {
            let [u, v] = self.at(p)?;
            let magnitude = u.hypot(v);
            (magnitude > 0. && magnitude.is_finite()).then(|| [u / magnitude, v / magnitude])
        };
        let advance = |p: [f64; 2], k: [f64; 2], s: f64| [p[0] + s * k[0], p[1] + s * k[1]];
        let mut p = seed;
        let mut points = Vec::new();
        for i in 0..budget {
            let Some(k1) = direction(p) else {
                break;
            };
            let Some(k2) = direction(advance(p, k1, h * 0.5)) else {
                break;
            };
            let Some(k3) = direction(advance(p, k2, h * 0.5)) else {
                break;
            };
            let Some(k4) = direction(advance(p, k3, h)) else {
                break;
            };
            let k = std::array::from_fn(|i| (k1[i] + 2. * k2[i] + 2. * k3[i] + k4[i]) / 6.);
            let next = advance(p, k, h);
            if self.at(next).is_none() || next == p {
                break;
            }
            points.push(next);
            p = next;
            if i > 4 && (p[0] - seed[0]).hypot(p[1] - seed[1]) < h.abs() * 0.7 {
                break;
            }
        }
        points
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uniform_flow_has_straight_bounded_streamlines() {
        let field = Grid::new(vec![0., 2.], vec![0., 1.], vec![[1., 0.]; 4]).expect("rectilinear field");
        assert_eq!(field.at([2., 1.]), Some([1., 0.]));
        let d = field.streamlines(&[[1., 0.5]], 0.25, 40);
        let points = &d.shapes[0].points;
        assert_eq!((points.first(), points.last()), (Some(&[0., 0.5]), Some(&[2., 0.5])));
        assert!(points.iter().all(|p| p[1] == 0.5));
    }
}

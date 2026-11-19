//! Lines in angular and radial coordinates.
use crate::Diagram;
use std::f64::consts::TAU;

fn project(angle: f64, radius: f64) -> [f64; 2] {
    [0.5 + radius * angle.cos(), 0.5 - radius * angle.sin()]
}

const RADIUS: f64 = 0.35;

fn grid(limit: f64) -> Diagram {
    let mut d = Diagram {
        square: true,
        centered_labels: true,
        ..Diagram::default()
    };
    let guide_point = |angle, radius| project(angle, radius).map(|v| v as f32);
    for ring in 1..=4 {
        let r = RADIUS * f64::from(ring) / 4.;
        d.guide((0..=96).map(|i| guide_point(f64::from(i) * TAU / 96., r)).collect());
    }
    for i in 0..8 {
        let angle = f64::from(i) * TAU / 8.;
        d.guide(vec![[0.5, 0.5], guide_point(angle, RADIUS)]);
        if i % 2 == 0 {
            d.labels.push((guide_point(angle, 0.44), format!("{}°", i * 45)));
        }
    }
    let outer_label = if !(0.01..1e5).contains(&limit) {
        format!("{limit:.2e}")
    } else {
        let rounded = format!("{limit:.3}");
        rounded.trim_end_matches('0').trim_end_matches('.').to_owned()
    };
    d.labels.push((guide_point(TAU / 16., RADIUS + 0.035), outer_label));
    d
}

/// Plot named series of [angle in radians, nonnegative radius] pairs.
/// Zero points east and angles grow counterclockwise. All series share the
/// same linear radial scale. Missing samples break the line.
pub fn lines<S: AsRef<str>, V: AsRef<[[f64; 2]]>>(series: &[(S, V)]) -> Diagram {
    let valid = |p: &[f64; 2]| p[0].is_finite() && p[1].is_finite() && p[1] >= 0.;
    let maximum = series
        .iter()
        .flat_map(|(_, v)| v.as_ref())
        .filter(|p| valid(p))
        .map(|p| p[1])
        .fold(0., f64::max);
    let limit = if maximum > 0. { maximum } else { 1. };
    let mut d = grid(limit);
    for (name, values) in series {
        let id = d.series(name.as_ref());
        for run in values.as_ref().split(|p| !valid(p)).filter(|run| run.len() >= 2) {
            d.path(
                id,
                run.iter().map(|&[a, r]| project(a, (r / limit) * RADIUS)).collect(),
                false,
                1.,
            );
        }
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unequal_angles_preserve_radius_and_missing_samples_break_the_line() {
        let points = [[0., 2.], [TAU / 8., 1.], [0., f64::NAN], [TAU / 4., 2.], [TAU / 2., 2.]];
        let d = lines(&[("Sensor", points)]);
        let paths: Vec<_> = d.shapes.iter().filter(|s| s.entry.is_some()).collect();
        assert_eq!(paths.len(), 2);
        let diagonal = 0.175 / 2f64.sqrt();
        for (got, want) in
            paths
                .iter()
                .flat_map(|s| &s.points)
                .zip([[0.85, 0.5], [0.5 + diagonal, 0.5 - diagonal], [0.5, 0.15], [0.15, 0.5]])
        {
            assert!((got[0] - want[0]).abs() < 1e-9 && (got[1] - want[1]).abs() < 1e-9);
        }
    }
}

/// An annular sector centered on an angle (radians). Angular width is also in radians.
#[derive(Clone, Copy, Debug)]
pub struct Bar {
    pub angle: f64,
    pub width: f64,
    pub inner: f64,
    pub outer: f64,
}
impl Bar {
    fn valid(self) -> bool {
        self.angle.is_finite()
            && self.width.is_finite()
            && self.width > 0.
            && self.width <= TAU
            && self.inner.is_finite()
            && self.inner >= 0.
            && self.outer.is_finite()
            && self.outer > self.inner
    }
    fn polygon(self, scale: f64) -> Vec<[f64; 2]> {
        let n = (96. * self.width / TAU).ceil().max(2.) as usize;
        let angle = |i| self.angle + self.width * (i as f64 / n as f64 - 0.5);
        let mut points: Vec<_> = (0..=n).map(|i| project(angle(i), self.outer * scale)).collect();
        points.extend((0..=n).rev().map(|i| project(angle(i), self.inner * scale)));
        points
    }
}
/// Angular bars with independently supplied angles, widths and radial intervals.
pub fn bars<S: AsRef<str>>(bars: &[(S, Bar)]) -> Option<Diagram> {
    if bars.is_empty() || bars.iter().any(|(_, b)| !b.valid()) {
        return None;
    }
    let maximum = bars.iter().map(|(_, b)| b.outer).fold(0., f64::max);
    let mut d = grid(maximum);
    for (name, b) in bars {
        let id = d.entry(name.as_ref(), b.outer - b.inner);
        let normalized = Bar {
            inner: b.inner / maximum,
            outer: b.outer / maximum,
            ..*b
        };
        d.path(id, normalized.polygon(RADIUS), true, 1.);
    }
    Some(d)
}

/// Scatter observations in radians and radius, sharing one scale across named groups.
pub fn scatter<S: AsRef<str>, V: AsRef<[[f64; 2]]>>(groups: &[(S, V)]) -> Diagram {
    let valid = |p: &[f64; 2]| p[0].is_finite() && p[1].is_finite() && p[1] >= 0.;
    let maximum = groups
        .iter()
        .flat_map(|(_, v)| v.as_ref())
        .filter(|p| valid(p))
        .map(|p| p[1])
        .fold(0., f64::max);
    let limit = if maximum > 0. { maximum } else { 1. };
    let mut d = grid(limit);
    for (name, values) in groups {
        let id = d.series(name.as_ref());
        for &[angle, radius] in values.as_ref().iter().filter(|p| valid(p)) {
            let [x, y] = project(angle, (radius / limit) * RADIUS);
            let points = (0..20)
                .map(|i| {
                    let t = f64::from(i) * TAU / 20.;
                    [x + 0.008 * t.cos(), y + 0.008 * t.sin()]
                })
                .collect();
            d.path(id, points, true, 1.);
        }
    }
    d
}

/// Fixed 17-segment left-ventricle layout. Values control color independently
/// of wedge areas. Segments 1, 7 and 13 start at the top, increasing
/// counterclockwise within each ring, and segment 17 occupies the center.
pub fn bullseye(values: &[f64; 17]) -> Option<Diagram> {
    if values.iter().any(|v| !v.is_finite()) {
        return None;
    }
    let mut d = Diagram {
        square: true,
        ..Diagram::default()
    };
    for (offset, n, inner, outer) in [
        (0, 6, 11. / 15., 1.),
        (6, 6, 7. / 15., 11. / 15.),
        (12, 4, 0.2, 7. / 15.),
        (16, 1, 0., 0.2),
    ] {
        for i in 0..n {
            let idx = offset + i;
            let id = d.entry(&format!("Segment {}", idx + 1), values[idx]);
            let width = TAU / n as f64;
            let b = Bar {
                angle: TAU / 4. + width * i as f64,
                width,
                inner,
                outer,
            };
            d.path(id, b.polygon(0.45), true, 1.);
        }
    }
    d.scalar_colors();
    Some(d)
}

/// Angular and radial uncertainties around an observation. Both half-widths
/// are nonnegative. Angles are radians, and lower radial bounds stop at zero.
#[derive(Clone, Copy, Debug)]
pub struct Uncertainty {
    pub angle: f64,
    pub radius: f64,
    pub angular_error: f64,
    pub radial_error: f64,
}
pub fn error_bars(samples: &[Uncertainty]) -> Option<Diagram> {
    if samples.is_empty()
        || samples.iter().any(|p| {
            !p.angle.is_finite()
                || !p.radius.is_finite()
                || p.radius < 0.
                || !(0. ..=TAU / 2.).contains(&p.angular_error)
                || !p.radial_error.is_finite()
                || p.radial_error < 0.
                || !(p.radius + p.radial_error).is_finite()
        })
    {
        return None;
    }
    let max = samples.iter().map(|p| p.radius + p.radial_error).fold(0., f64::max);
    let limit = if max > 0. { max } else { 1. };
    let mut d = grid(limit);
    for (i, p) in samples.iter().enumerate() {
        let id = d.entry(&format!("Observation {}", i + 1), p.radius);
        let point = |a, r| project(a, (r / limit) * RADIUS);
        if p.radial_error > 0. {
            d.path(
                id,
                vec![
                    point(p.angle, (p.radius - p.radial_error).max(0.)),
                    point(p.angle, p.radius + p.radial_error),
                ],
                false,
                1.,
            );
        }
        if p.angular_error > 0. {
            let n = (128. * p.angular_error / TAU).ceil().max(2.) as usize;
            d.path(
                id,
                (0..=n)
                    .map(|i| point(p.angle + p.angular_error * (2. * i as f64 / n as f64 - 1.), p.radius))
                    .collect(),
                false,
                1.,
            );
        }
        let [x, y] = point(p.angle, p.radius);
        d.path(
            id,
            (0..16)
                .map(|j| {
                    let a = TAU * f64::from(j) / 16.;
                    [x + 0.006 * a.cos(), y + 0.006 * a.sin()]
                })
                .collect(),
            true,
            1.,
        );
    }
    Some(d)
}

#[cfg(test)]
mod layouts {
    use super::*;
    #[test]
    fn fixed_ring_sector_geometry_is_independent_of_its_value() {
        let d = bullseye(&std::array::from_fn(|i| i as f64)).expect("seventeen segments");
        assert_eq!(d.color_domain, Some((0., 16.)));
        assert_eq!(d.shapes.len(), 17);
        let span = |i: usize| {
            d.shapes[i]
                .points
                .iter()
                .map(|p| (p[0] - 0.5).hypot(p[1] - 0.5))
                .fold(0., f64::max)
        };
        assert!((span(0) - 0.45).abs() < 1e-10);
        assert!((span(16) - 0.09).abs() < 1e-10);
        assert_eq!(d.entries[0].name, "Segment 1");
        let top = &d.shapes[0].points[8];
        assert!((top[0] - 0.5).abs() < 1e-10 && top[1] < 0.1);
    }
    #[test]
    fn polar_observations_have_independent_angle_width_radius_and_uncertainty() {
        let b = Bar {
            angle: TAU / 8.,
            width: TAU / 8.,
            inner: 2.,
            outer: 4.,
        };
        let d = bars(&[("sample", b)]).expect("valid bar");
        assert_eq!(d.entries[0].value, 2.);
        let s = scatter(&[("sample", [[0., 2.], [TAU / 4., 1.]])]);
        assert_eq!(s.shapes.iter().filter(|s| s.closed).count(), 2);
        let e = error_bars(&[Uncertainty {
            angle: 0.,
            radius: 2.,
            angular_error: TAU / 8.,
            radial_error: 1.,
        }])
        .expect("uncertainties");
        let shapes: Vec<_> = e.shapes.iter().filter(|s| s.entry.is_some()).collect();
        assert_eq!(shapes.len(), 3);
        assert_eq!(shapes[0].points.len(), 2);
        assert!(shapes[1].points.len() > 2 && shapes[2].closed);
    }
}

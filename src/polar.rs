//! Lines in angular and radial coordinates.
use crate::Diagram;
use std::f64::consts::TAU;

fn project(angle: f64, radius: f64) -> [f64; 2] {
    [0.5 + radius * angle.cos(), 0.5 - radius * angle.sin()]
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
    let mut d = Diagram {
        square: true,
        centered_labels: true,
        ..Diagram::default()
    };
    const RADIUS: f64 = 0.35;
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

//! Chart layouts for signed matrices, causal diagrams and atmospheric soundings.
use crate::{Data, Diagram, Mark, data};

/// Draw a signed matrix with a square per nonzero element. Square area is
/// proportional to weight magnitude and color distinguishes sign.
pub fn hinton(rows: usize, columns: usize, weights: &[f64]) -> Option<Diagram> {
    if rows == 0 || columns == 0 || weights.len() != rows.checked_mul(columns)? {
        return None;
    }
    let max = weights
        .iter()
        .copied()
        .filter(|x| x.is_finite())
        .map(f64::abs)
        .fold(0., f64::max);
    let mut d = Diagram {
        square: true,

        ..Diagram::default()
    };
    let side = rows.max(columns) as f64;
    let origin = [(1. - columns as f64 / side) / 2., (1. - rows as f64 / side) / 2.];
    let right = origin[0] + columns as f64 / side;
    let bottom = origin[1] + rows as f64 / side;
    d.guide(
        [origin, [right, origin[1]], [right, bottom], [origin[0], bottom], origin]
            .map(|p| p.map(|v| v as f32))
            .to_vec(),
    );
    for (i, &w) in weights.iter().enumerate().filter(|(_, v)| v.is_finite() && **v != 0.) {
        let row = i / columns;
        let col = i % columns;
        let x = origin[0] + (col as f64 + 0.5) / side;
        let y = origin[1] + (row as f64 + 0.5) / side;
        let r = (w.abs() / max).sqrt() / side / 2.;
        let id = d.entry(&format!("Row {}, column {}", row + 1, col + 1), w);
        d.set_color(id, if w > 0. { 0x287c72 } else { 0xaa5947 });
        d.path(
            id,
            vec![[x - r, y - r], [x + r, y - r], [x + r, y + r], [x - r, y + r]],
            true,
            1.,
        );
    }
    Some(d)
}

/// Each cause group branches into the shared effect. Groups alternate above
/// and below the spine, with a separate rib for each contributing cause.
pub fn fishbone<S: AsRef<str>, T: AsRef<str>, V: AsRef<[T]>>(effect: &str, groups: &[(S, V)]) -> Diagram {
    let mut d = Diagram {
        centered_labels: true,
        ..Diagram::default()
    };
    let effect_id = d.series(effect);
    d.path(effect_id, vec![[0.045, 0.50], [0.88, 0.50]], false, 1.);
    d.path(effect_id, vec![[0.862, 0.478], [0.88, 0.50], [0.862, 0.522]], false, 1.);
    d.labels.push(([0.90, 0.57], effect.into()));
    let pairs = groups.len().div_ceil(2).max(1) as f64;
    let slot = 0.77 / pairs;
    for (i, (name, causes)) in groups.iter().enumerate() {
        let x = 0.055 + slot * ((i / 2 + 1) as f64);
        let dir = if i % 2 == 0 { -1. } else { 1. };
        let end = [x - slot * 0.65, 0.50 + dir * 0.33];
        let id = d.series(name.as_ref());
        d.path(id, vec![[x, 0.5], end], false, 1.);
        d.labels
            .push(([end[0] as f32, (end[1] + dir * 0.055) as f32], name.as_ref().into()));
        let causes = causes.as_ref();
        for (j, cause) in causes.iter().enumerate() {
            let t = (j + 1) as f64 / (causes.len() + 1) as f64;
            let a = [x - slot * 0.65 * t, 0.5 + dir * 0.33 * t];
            let b = [a[0] - slot * 0.28, a[1]];
            d.path(id, vec![a, b], false, 1.);
            d.labels.push((
                [(b[0] - slot * 0.04) as f32, (b[1] - 0.03) as f32],
                cause.as_ref().into(),
            ));
        }
    }
    d
}

/// Atmospheric temperature profiles at pressure levels. The x domain is
/// measured at the bottom of the plot. Isotherms shift horizontally by
/// `skew` degrees Celsius between the bottom and top pressure boundaries.
/// Supply pressure bounds as [surface, upper] in hectopascals.
pub fn skew_t<S: AsRef<str>, V: AsRef<[[f64; 2]]>>(
    profiles: &[(S, V)],
    temperature: [f64; 2],
    pressure: [f64; 2],
    skew: f64,
) -> Option<Data> {
    let [left, right] = temperature;
    let [bottom, top] = pressure;
    if ![left, right, bottom, top, skew].iter().all(|v| v.is_finite()) || left >= right || top <= 0. || bottom <= top {
        return None;
    }
    let (y0, y1) = (-bottom.ln(), -top.ln());
    let span = right - left;
    if !span.is_finite() || y0 >= y1 || !(left - skew.max(0.)).is_finite() || !(right - skew.min(0.)).is_finite() {
        return None;
    }
    let at = |t, y| [t + skew * ((y - y0) / (y1 - y0)), y];
    let mut d = Diagram::cartesian();
    let step = 10_f64.powf((right - left).log10().floor() - 1.).max(f64::MIN_POSITIVE);
    let step = if (right - left) / step > 20. { step * 5. } else { step };
    let first = ((left - (skew.max(0.))) / step).ceil();
    let last = ((right - (skew.min(0.))) / step).floor();
    // Limit guide density independently of unusually large user-supplied skew.
    let stride = ((last - first + 1.) / 128.).ceil().max(1.);
    let guides = ((last - first) / stride).floor().clamp(0., 127.) as usize;
    for i in 0..=guides {
        let temp = (first + i as f64 * stride) * step;
        let segment = [at(temp, y0), at(temp, y1)];
        d.guide(segment.map(|p| p.map(|v| v as f32)).to_vec());
    }
    for (name, measurements) in profiles {
        let id = d.series(name.as_ref());
        for run in measurements
            .as_ref()
            .split(|p| !p[0].is_finite() || !p[1].is_finite() || p[1] <= 0.)
            .filter(|run| run.len() > 1)
        {
            d.path(id, run.iter().map(|&[temp, p]| at(temp, -p.ln())).collect(), false, 1.);
        }
    }
    let pressure_ticks: Vec<_> = [
        1050., 1000., 925., 850., 700., 600., 500., 400., 300., 250., 200., 150., 100., 70., 50., 30., 20., 10.,
    ]
    .into_iter()
    .filter(|&p| p >= top && p <= bottom)
    .map(|p| -p.ln())
    .collect();
    Some(
        data(vec![Mark::Diagram(d)])
            .x_axis(|a| a.with_bounds(left, right).title("Temperature (°C)"))
            .y_axis(|a| {
                a.with_bounds(y0, y1)
                    .with_ticks(pressure_ticks)
                    .with_label_format(|y| format!("{:.0}", (-y).exp()))
                    .title("Pressure (hPa)")
            }),
    )
}

/// Choice of streamgraph baseline. Wiggle reduces the aggregate layer slopes,
/// while weighted wiggle weights each contribution by its share of total flow.
#[derive(Clone, Copy, Debug)]
pub enum Baseline {
    Centered,
    Wiggle,
    WeightedWiggle,
}

/// Stack nonnegative series sampled at common, strictly increasing positions.
/// The baseline translates each column without changing any layer's thickness.
pub fn streamgraph<S: AsRef<str>, V: AsRef<[f64]>>(
    x: &[f64],
    series: &[(S, V)],
    baseline: Baseline,
) -> Option<Diagram> {
    if x.len() < 2
        || series.is_empty()
        || x.iter().any(|v| !v.is_finite())
        || x.windows(2).any(|p| p[0] >= p[1])
        || series
            .iter()
            .any(|(_, s)| s.as_ref().len() != x.len() || s.as_ref().iter().any(|v| !v.is_finite() || *v < 0.))
    {
        return None;
    }
    let mut bottom = Vec::with_capacity(x.len());
    let mut center = 0.;
    for j in 0..x.len() {
        let total: f64 = series.iter().map(|(_, s)| s.as_ref()[j]).sum();
        if !total.is_finite() {
            return None;
        }
        let base = match baseline {
            Baseline::Centered => -0.5 * total,
            Baseline::Wiggle => -series
                .iter()
                .enumerate()
                .map(|(i, (_, s))| s.as_ref()[j] * ((series.len() - i) as f64 - 0.5) / series.len() as f64)
                .sum::<f64>(),
            Baseline::WeightedWiggle => {
                if j > 0 {
                    let mut above = total;
                    for (_, s) in series {
                        let s = s.as_ref();
                        let fraction = if total > 0. { (above - 0.5 * s[j]) / total } else { 0. };
                        center += (fraction - 0.5) * (s[j] - s[j - 1]);
                        above -= s[j];
                    }
                }
                center - 0.5 * total
            }
        };
        if !base.is_finite() {
            return None;
        }
        bottom.push(base);
    }
    let mut d = Diagram::cartesian();
    for (label, s) in series {
        let id = d.series(label.as_ref());
        let values = s.as_ref();
        let mut path: Vec<_> = x.iter().zip(&bottom).map(|(&x, &y)| [x, y]).collect();
        for (y, v) in bottom.iter_mut().zip(values) {
            *y += v;
        }
        path.extend(x.iter().zip(&bottom).rev().map(|(&x, &y)| [x, y]));
        d.path(id, path, true, 1.);
    }
    Some(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_matrix_encodes_magnitude_as_area_and_skips_zero() {
        let d = hinton(2, 2, &[4., -1., 0., f64::NAN]).expect("signed matrix");
        let squares: Vec<_> = d.shapes.iter().filter(|s| s.entry.is_some()).collect();
        assert_eq!(squares.len(), 2);
        let width = |i: usize| squares[i].points[1][0] - squares[i].points[0][0];
        assert_eq!((width(0) / width(1)).powi(2), 4.);
        assert_ne!(d.entries[0].color, d.entries[1].color);
    }
    #[test]
    fn causal_branches_connect_each_cause_to_its_category_and_effect() {
        let d = fishbone("effect", &[("upper", ["one", "two"]), ("lower", ["three", "four"])]);
        assert_eq!(d.entries.len(), 3);
        for i in 1..=2 {
            assert_eq!(d.shapes.iter().filter(|s| s.entry == Some(i)).count(), 3);
        }
        assert_eq!(d.labels.len(), 7);
    }
    #[test]
    fn weighted_baseline_conserves_thickness_and_accounts_for_center_movement() {
        let d = streamgraph(
            &[0., 1., 2.],
            &[("one", [1., 2., 0.]), ("two", [3., 1., 0.])],
            Baseline::WeightedWiggle,
        )
        .expect("flows");
        let baseline: Vec<_> = d.shapes[0].points[..3].iter().map(|p| p[1]).collect();
        assert_eq!(baseline[0], -2.);
        assert!((baseline[1] + 2. / 3.).abs() < 1e-12);
        for (shape, expected) in d.shapes.iter().zip([[1., 2., 0.], [3., 1., 0.]]) {
            for (i, h) in expected.iter().enumerate() {
                assert!((shape.points[5 - i][1] - shape.points[i][1] - h).abs() < 1e-10);
            }
        }
        assert!(streamgraph(&[0., 0.], &[("one", [1., 1.])], Baseline::Centered).is_none());
    }
    #[test]
    fn skew_t_uses_log_pressure_and_tilts_constant_temperature_right() {
        let d = skew_t(
            &[("dry", [[0., 1000.], [0., 100.], [1., 0.], [2., 20.]])],
            [-50., 50.],
            [1000., 100.],
            40.,
        )
        .expect("valid domain");
        let Mark::Diagram(d) = &d.marks()[0] else {
            panic!("diagram")
        };
        let profile = d.shapes.iter().find(|s| s.entry == Some(0)).expect("profile");
        assert_eq!(profile.points.len(), 2);
        assert_eq!(profile.points[0], [0., -1000f64.ln()]);
        assert_eq!(profile.points[1], [40., -100f64.ln()]);
    }
}

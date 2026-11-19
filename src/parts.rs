//! Split a small group of categories out of a primary pie into a second chart.
use crate::Diagram;
use std::f64::consts::TAU;

#[derive(Clone, Copy, Debug)]
pub enum Breakdown {
    Pie,
    Bar,
}

/// The first `split_at` entries get individual main slices. The remaining
/// nonnegative entries form the main pie's combined slice and the breakdown.
/// Both halves must have positive weight. Category order is preserved.
pub fn split_pie<S: AsRef<str>>(items: &[(S, f64)], split_at: usize, view: Breakdown) -> Option<Diagram> {
    if split_at == 0 || split_at >= items.len() || items.iter().any(|(_, v)| !v.is_finite() || *v < 0.) {
        return None;
    }
    let total: f64 = items.iter().map(|i| i.1).sum();
    let secondary: f64 = items[split_at..].iter().map(|i| i.1).sum();
    if !total.is_finite() || secondary <= 0. || secondary >= total {
        return None;
    }
    let mut d = Diagram {
        square: true,
        ..Diagram::default()
    };
    for (name, value) in items {
        d.entry(name.as_ref(), *value);
    }
    let grouped = d.entry("Other", secondary);
    let fraction = secondary / total;
    let circ = |cx: f64, r: f64, t: f64| [cx + r * (TAU * t).cos(), 0.5 + r * (TAU * t).sin()];
    let sector = |cx: f64, r: f64, a: f64, b: f64| {
        let steps = ((b - a) * 144.).ceil().max(1.) as usize;
        let mut p = vec![[cx, 0.5]];
        p.extend((0..=steps).map(|i| circ(cx, r, a + (b - a) * i as f64 / steps as f64)));
        p
    };
    let left = 0.29;
    let start = -fraction / 2.;
    let upper = circ(left, 0.28, start);
    let lower = circ(left, 0.28, start + fraction);
    d.guide(vec![upper.map(|x| x as f32), [0.78, 0.33]]);
    d.guide(vec![lower.map(|x| x as f32), [0.78, 0.67]]);
    d.path(grouped, sector(left, 0.28, start, start + fraction), true, 1.);
    let mut t = start + fraction;
    for (i, (_, v)) in items[..split_at].iter().enumerate() {
        if *v > 0. {
            let next = t + v / total;
            d.path(i, sector(left, 0.28, t, next), true, 1.);
            t = next;
        }
    }
    let mut t = 0.;
    for (i, (_, v)) in items[split_at..].iter().enumerate() {
        if *v == 0. {
            continue;
        }
        let next = t + v / secondary;
        let vertices = match view {
            Breakdown::Pie => sector(0.79, 0.17, t - 0.25, next - 0.25),
            Breakdown::Bar => vec![
                [0.72, 0.33 + 0.34 * t],
                [0.89, 0.33 + 0.34 * t],
                [0.89, 0.33 + 0.34 * next],
                [0.72, 0.33 + 0.34 * next],
            ],
        };
        d.path(i + split_at, vertices, true, 1.);
        t = next;
    }
    Some(d)
}

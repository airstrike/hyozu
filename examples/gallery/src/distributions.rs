use hyozu::stats::{self, Bins};
use hyozu::{Data, axis, data};

const SAMPLE: &[f64] = &[
    1.2, 1.9, 2.0, 2.3, 2.3, 2.5, 2.5, 2.5, 2.7, 2.8, 3.1, 3.3, 3.4, 3.7, 4.1, 4.8,
];
const EDGES: &[f64] = &[1., 1.5, 2., 2.5, 3., 3.5, 4., 4.5, 5.];

fn bins() -> Bins {
    Bins::new(EDGES, SAMPLE).expect("gallery bin edges increase")
}

pub fn histogram() -> Data {
    bins().histogram(false).title("Sample frequency")
}

pub fn frequency_polygon() -> Data {
    data(bins().frequency_polygon(false))
        .x_axis(|a| a.with_kind(axis::Kind::Scalar))
        .title("Frequency polygon")
}

pub fn density() -> Data {
    stats::density(SAMPLE).title("Gaussian density estimate")
}

pub fn ecdf() -> Data {
    stats::ecdf(SAMPLE).title("Empirical cumulative distribution")
}

pub fn qq() -> Data {
    data(stats::qq(SAMPLE, |p| 1. + p * 4.)).title("Sample vs. uniform quantiles")
}

pub fn pareto() -> Data {
    stats::pareto(&[("Payment", 22), ("Transit", 11), ("Quality", 45), ("Other", 5)]).title("Returns by reason")
}

pub fn dotplot() -> Data {
    data(stats::dotplot(&bins())).title("Observations by interval")
}
pub fn survival() -> Data {
    stats::survival(SAMPLE).title("Empirical survival")
}
pub fn fit() -> Data {
    let observations = [[1., 2.], [2., 2.4], [3., 4.2], [4., 4.], [5., 6.5], [6., 6.2], [7., 8.]];
    stats::fit::linear(&observations, Some(2.571))
        .expect("varying x positions")
        .title("Trend with 95% mean confidence band")
}
pub fn smooth() -> Data {
    let points: Vec<_> = (0..50)
        .map(|i| {
            let x = f64::from(i) * 0.2;
            [x, x.sin() + 0.4 * (x * 7.).sin()]
        })
        .collect();
    stats::fit::loess(&points, 0.32)
        .expect("valid span")
        .title("Locally weighted trend")
}
pub fn jitter() -> Data {
    let points: Vec<_> = (0..100).map(|i| [f64::from(i % 4), f64::from((i / 4) % 5)]).collect();
    data(stats::fit::jitter(&points, [0.22, 0.22], 18)).title("Repeated observations")
}
pub fn percent_bars() -> Data {
    let groups = [
        ("Direct", [4., 8., 6., 12.]),
        ("Partner", [3., 12., 5., 7.]),
        ("Organic", [3., 8., 9., 8.]),
    ];
    data(stats::percent_bars(&groups).expect("nonnegative aligned groups"))
        .y_axis_bounds(0., 1.)
        .y_axis_labels(|v: f64| format!("{:.0}%", v * 100.))
        .title("Channel share")
}
pub fn percent_areas() -> Data {
    let groups = [
        ("Direct", [4., 8., 6., 12.]),
        ("Partner", [3., 12., 5., 7.]),
        ("Organic", [3., 8., 9., 8.]),
    ];
    data(stats::percent_areas(&groups).expect("nonnegative aligned groups"))
        .y_axis_bounds(0., 1.)
        .y_axis_labels(|v: f64| format!("{:.0}%", v * 100.))
        .title("Channel share through time")
}

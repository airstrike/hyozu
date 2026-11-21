use hyozu::cartesian::{self, Interval, Ohlc};
use hyozu::{Data, data};

const RANGES: [Interval; 5] = [
    Interval {
        x: 1.,
        low: 2.,
        high: 4.,
    },
    Interval {
        x: 2.,
        low: 3.,
        high: 7.,
    },
    Interval {
        x: 3.,
        low: 3.,
        high: 5.,
    },
    Interval {
        x: 4.,
        low: 4.,
        high: 9.,
    },
    Interval {
        x: 5.,
        low: 5.,
        high: 7.,
    },
];

fn quotes() -> Vec<Ohlc> {
    [
        (4., 8., 3., 7.),
        (7., 9., 5., 6.),
        (6., 8., 4., 5.),
        (5., 9., 4., 8.),
        (8., 11., 7., 10.),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (open, high, low, close))| Ohlc {
        x: i as f64,
        open,
        high,
        low,
        close,
    })
    .collect()
}

pub fn candlesticks() -> Data {
    data(cartesian::candlesticks(&quotes(), 0.52)).title("Daily trading range")
}
pub fn ohlc() -> Data {
    data(cartesian::ohlc(&quotes(), 0.52)).title("Opening and closing prices")
}
pub fn ribbon() -> Data {
    data(cartesian::ribbon(&RANGES)).title("Forecast interval")
}
pub fn error_bars() -> Data {
    data(cartesian::error_bars(&RANGES, 0.35)).title("Measurement intervals")
}
pub fn step() -> Data {
    data(cartesian::steps(
        &[[0., 2.], [1., 4.], [2., 3.], [3., 6.], [4., 4.]],
        hyozu::line::Step::After,
    ))
    .title("Inventory on hand")
}
pub fn stem() -> Data {
    cartesian::stem(&[[1., 3.], [2., -2.], [3., 1.], [4., 5.], [5., -3.]], 0.).title("Signal impulses")
}
pub fn events() -> Data {
    data(cartesian::events(
        &[&[0.2, 0.4, 1., 1.2, 3.8], &[0.1, 1.4, 2.4, 2.5, 3.7], &[
            0.75, 0.9, 1.2, 3.3,
        ]],
        0.65,
    ))
    .title("Events by sensor")
}

pub fn stock_volume() -> Data {
    let quotes = quotes();
    let v = [[0., 1400.], [1., 1100.], [2., 1750.], [3., 950.], [4., 2000.]];
    cartesian::with_volume(cartesian::ohlc(&quotes, 0.5), &v, 0.6).title("Price and trading volume")
}
pub fn high_low_close() -> Data {
    let q: Vec<_> = quotes()
        .into_iter()
        .map(|q| cartesian::Hlc {
            x: q.x,
            high: q.high,
            low: q.low,
            close: q.close,
        })
        .collect();
    data(cartesian::hlc(&q, 0.6)).title("High, low and closing price")
}
pub fn horizontal_ranges() -> Data {
    data(cartesian::horizontal_ranges(&RANGES, 0.35)).title("Confidence intervals")
}
pub fn crossbars() -> Data {
    data(cartesian::crossbars(&RANGES.map(|r| (r, r.low.midpoint(r.high))), 0.58)).title("Estimated treatment effects")
}
pub fn points() -> Data {
    cartesian::point_ranges(&RANGES.map(|r| (r, r.low.midpoint(r.high))), 0.).title("Point estimates and ranges")
}
pub fn stairs() -> Data {
    data(cartesian::stairs(&[0., 1., 2.5, 3., 5., 6.], &[2., 4., 3., 6., 2.], Some(0.)).expect("six ordered edges"))
        .title("Unequal observation intervals")
}
pub fn function() -> Data {
    data(cartesian::function([-4., 4.], 160, |x| (-0.2 * x * x).exp() * (3. * x).cos()).expect("finite domain"))
        .title("Damped oscillation")
}
pub fn polygons() -> Data {
    data(cartesian::polygons(&[
        ("Northern parcel", vec![[0., 2.], [1., 4.], [4., 5.], [3., 2.]]),
        ("Southern parcel", vec![[0., 1.], [3., 1.8], [4., 0.], [1., -1.]]),
    ]))
    .title("Survey parcels")
}
pub fn curves() -> Data {
    data(cartesian::curves(
        &[[[0., 0.], [1., 5.], [3., -2.], [5., 3.]], [
            [0., 1.],
            [2., 6.],
            [4., 0.],
            [5., 4.],
        ]],
        40,
    ))
    .title("Cubic trajectories")
}
pub fn spokes() -> Data {
    data(cartesian::spokes(
        &(0..16)
            .map(|i| {
                let a = f64::from(i) * std::f64::consts::TAU / 16.;
                ([a.cos(), a.sin()], a + 0.45, 0.5 + 0.2 * (2. * a).cos())
            })
            .collect::<Vec<_>>(),
    ))
    .title("Oriented measurements")
}
pub fn rug() -> Data {
    let values = [0.3, 0.42, 0.8, 1.2, 1.32, 1.41, 1.6, 1.84, 2.2, 2.6, 2.7, 3.2, 3.6];
    let mut marks = hyozu::stats::density(&values).marks().to_vec();
    marks.push(cartesian::rug(&values, 0., 0.035, true).into());
    data(marks).title("Density and observed values")
}

pub fn smooth() -> Data {
    cartesian::smooth_scatter(
        &[[0.5, 2.], [1., 4.], [2.4, 3.6], [3.1, 6.], [5., 4.], [6.8, 7.2]],
        true,
    )
    .title("Interpolated observations")
}
pub fn spherical() -> Data {
    let values = [8., 22., 14., 34., 18., 42.];
    let points = hyozu::xy([(1., 3.), (2., 5.), (3., 2.), (4., 6.), (5., 4.), (6., 7.)])
        .size_by(
            hyozu::encoding::size_by(move |i, _| values[i])
                .domain(0.0..=42.)
                .range(0.0..=43.),
        )
        .spherical(true)
        .size_legend(hyozu::legend::Config::right())
        .size_legend_title("Orders (K)");
    data(points)
        .x_axis_bounds(0., 7.)
        .y_axis_bounds(0., 9.)
        .title("Sales, margin and order size")
}

pub fn boxed_labels() -> Data {
    use hyozu::mark::text::{TextItem, text};
    let points = [(0., 2.), (1., 3.), (2., 2.7), (3., 4.), (4., 5.)];
    let labels = text([
        TextItem::new(hyozu::Datum::new(1., 3.), "Pilot"),
        TextItem::new(hyozu::Datum::new(3., 4.), "Release"),
    ])
    .offset(0., -15.)
    .color(hyozu::core::Color::WHITE)
    .background(hyozu::core::Color::from_rgb8(42, 105, 101), 4.);
    data(vec![hyozu::Mark::Line(hyozu::line(points)), hyozu::Mark::Text(labels)])
        .title("Milestones and observed growth")
}

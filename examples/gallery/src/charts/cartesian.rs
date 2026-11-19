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
        cartesian::Step::After,
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

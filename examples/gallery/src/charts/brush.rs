use hyozu::brush::Selection;
use hyozu::{Data, data, line};
pub struct Example {
    pub chart: Data,
    pub overview: Vec<[f64; 2]>,
    pub selection: Selection,
}
pub fn example() -> Example {
    let overview: Vec<_> = (0..=100)
        .map(|i| {
            let t = f64::from(i);
            [t, 22. + 0.12 * t + (0.24 * t).sin() * 4. + (t * 0.7).cos() * 1.5]
        })
        .collect();
    let mut chart = data(line(overview.iter().map(|&[x, y]| (x, y)).collect::<Vec<_>>()))
        .x_axis(|a| a.with_kind(hyozu::axis::Kind::Scalar))
        .title("Sessions over time");
    let selection = Selection::new([0., 100.], [25., 75.]).expect("ordered range");
    selection.apply(&mut chart);
    Example {
        chart,
        overview,
        selection,
    }
}

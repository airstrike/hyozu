use hyozu::{Data, data, field, stats};
fn observations() -> Vec<[f64; 2]> {
    (0..96)
        .map(|i| {
            let x = f64::from(i % 8);
            [x, 3. + 0.6 * x + (f64::from(i) * 2.3).sin() * (1. + x * 0.25)]
        })
        .collect()
}
pub fn point_range() -> Data {
    stats::summary::mean_se(&observations(), 1.)
        .expect("finite range")
        .title("Mean and standard error")
}
pub fn binned() -> Data {
    let points = stats::summary::bins(&observations(), &[-0.5, 1.5, 4.5, 7.5], |v| {
        v.iter().sum::<f64>() / v.len() as f64
    })
    .expect("ordered bins");
    data(hyozu::line(points)).title("Mean by observation interval")
}
pub fn grid() -> Data {
    let samples: Vec<_> = (0..500)
        .map(|i| {
            let t = f64::from(i) * 2.39996;
            let r = (f64::from(i) / 500.).sqrt();
            let (x, y) = (r * t.cos(), r * t.sin());
            [x, y, 2. + 1.4 * x - 0.6 * y]
        })
        .collect();
    let edges: Vec<_> = (-5..=5).map(|i| f64::from(i) / 5.).collect();
    data(
        field::summary2d(&edges, &edges, &samples, |values| {
            values.iter().sum::<f64>() / values.len() as f64
        })
        .expect("grid cells"),
    )
    .palette(hyozu::palette::gradient([0xb6d7c0, 0x226a72]))
    .title("Mean measurement per cell")
}
pub fn connector() -> Data {
    data(
        hyozu::cartesian::connect(&[[1., 2.], [2., 3.], [3., 1.], [4., 4.], [5., 3.]], &[
            [0.4, 0.75],
            [0.6, 0.25],
            [1., 1.],
        ])
        .expect("connection ending at one"),
    )
    .title("Custom transitions")
}
pub fn grouped() -> Data {
    let rows = observations();
    let marks = stats::summary::groups(
        rows,
        |p| p[0] as i32 % 2,
        |key, v| {
            let ordered = stats::summary::at(v, |y| y.iter().sum::<f64>() / y.len() as f64);
            hyozu::Mark::Line(hyozu::line(ordered).with_name(format!("Group {}", key + 1)))
        },
    )
    .into_iter()
    .map(|(_, mark)| mark)
    .collect::<Vec<_>>();
    data(marks).title("Per-group mean trajectories")
}
pub fn robust() -> Data {
    let mut pts: Vec<_> = (0..80)
        .map(|i| {
            let a = f64::from(i) * 2.39996;
            let r = (f64::from(i) / 80.).sqrt();
            let x = r * a.cos();
            [x, 0.55 * x + 0.45 * r * a.sin()]
        })
        .collect();
    pts.push([2.4, -1.8]);
    let fit = stats::student_ellipse(&pts, 5., 2.).expect("identifiable scatter matrix");
    data(vec![
        hyozu::Mark::Xy(hyozu::xy(pts.into_iter().map(|p| (p[0], p[1])).collect::<Vec<_>>())),
        hyozu::Mark::Line(fit),
    ])
    .title("Robust bivariate scatter")
}

pub fn aligned() -> Data {
    data(
        stats::aligned_areas(&[
            ("North", vec![[0., 2.], [2., 4.], [5., 3.], [7., 5.]]),
            ("South", vec![[1., 1.], [3., 4.], [4., 3.], [7., 2.]]),
        ])
        .expect("increasing sample positions"),
    )
    .title("Nonuniform observation periods")
}

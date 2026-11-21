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

pub fn trendlines() -> Data {
    use hyozu::stats::trend;
    let bookings: Vec<_> = [2.1, 2.9, 3.6, 5.2, 6.1, 8.3, 10.2, 13.9, 17.1, 23.]
        .iter()
        .enumerate()
        .map(|(i, &y)| [i as f64 + 1., y])
        .collect();
    let forecast = trend::Forecast::new(0.5, 2.).expect("nonnegative periods");
    let fits = [
        (trend::linear(&bookings, None), 0x2563eb),
        (trend::exponential(&bookings, None), 0xdc2626),
        (trend::logarithmic(&bookings), 0x059669),
        (trend::polynomial(&bookings, trend::Order::Three, None), 0xd97706),
        (trend::power(&bookings), 0x7c3aed),
    ];
    let mut marks = vec![hyozu::Mark::Xy(
        hyozu::xy(bookings.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>())
            .color(0x1f2937)
            .with_name("Bookings"),
    )];
    for (fit, color) in fits {
        let fit = fit.expect("positive, varying observations");
        let line = fit.line(forecast).expect("finite forecast");
        let name = format!(
            "{}: {}, R² = {:.4}",
            line.name().unwrap_or_default(),
            fit.equation(),
            fit.r_squared()
        );
        marks.push(hyozu::Mark::Line(
            line.with_name(name).color(color).style(hyozu::LineStyle::Dashed),
        ));
    }
    let period = trend::Period::new(3).expect("at least two points");
    marks.push(hyozu::Mark::Line(
        trend::moving_average(&bookings, period)
            .expect("more points than the period")
            .color(0x6b7280),
    ));
    data(marks)
        .x_axis(|a| a.with_kind(hyozu::axis::Kind::Scalar))
        .y_axis_bounds(-10., 40.)
        .legend(hyozu::legend::Anchor::Bottom)
        .title("Trendlines with a two-period forecast")
}

pub fn error_amounts() -> Data {
    use hyozu::cartesian::error;
    let amount = |v| error::Magnitude::new(v).expect("nonnegative amount");
    let custom = error::Custom::new(&[0.3, 0.6, 0.4, 0.9, 0.5, 1.2], &[0.2, 0.3, 0.6, 0.4, 0.8, 0.5])
        .expect("one pair per point");
    let kinds = [
        ("Fixed ±0.5", error::Amount::Fixed(amount(0.5)), 0x2563eb),
        ("Percent ±10%", error::Amount::Percent(amount(10.)), 0xd97706),
        ("1 standard deviation", error::Amount::StdDev(amount(1.)), 0x059669),
        ("Standard error", error::Amount::StdErr, 0xdc2626),
        ("Custom", error::Amount::Custom(custom), 0x7c3aed),
    ];
    let values = [4.2, 5.1, 4.6, 6.3, 5.8, 7.];
    let mut marks = Vec::new();
    for (k, (name, amount, color)) in kinds.into_iter().enumerate() {
        let offset = (k as f64 - 2.) * 0.14;
        let series: Vec<_> = values
            .iter()
            .enumerate()
            .map(|(i, &y)| [i as f64 + 1. + offset, y])
            .collect();
        let intervals = amount.vertical(&series, error::Direction::Both).expect("finite series");
        let mut bars = error::bars(name, &intervals, 0.08);
        bars.set_color(0, color);
        marks.push(hyozu::Mark::Diagram(bars));
        marks.push(hyozu::Mark::Xy(
            hyozu::xy(series.iter().map(|p| (p[0], p[1])).collect::<Vec<_>>()).color(color),
        ));
    }
    data(marks)
        .x_axis(|a| a.with_kind(hyozu::axis::Kind::Scalar))
        .legend(hyozu::legend::Anchor::Bottom)
        .title("Error amounts on one series")
}

use hyozu::{bar, bars, line, xy};

/// Kleiber's law: resting metabolic rate grows as body mass to the ¾
/// power, a straight line once both axes are logarithmic.
pub fn log_log() -> hyozu::Data {
    let species: [f64; 10] = [0.02, 0.3, 1.2, 3.0, 12.0, 30.0, 70.0, 250.0, 600.0, 4000.0];
    let observed: Vec<(f64, f64)> = species
        .iter()
        .zip([1.05, 0.92, 1.1, 0.97, 1.04, 0.95, 1.02, 0.9, 1.08, 0.97])
        .map(|(&mass, noise)| (mass, 3.4 * mass.powf(0.75) * noise))
        .collect();
    let fit: Vec<(f64, f64)> = [0.01, 10_000.0].map(|mass: f64| (mass, 3.4 * mass.powf(0.75))).to_vec();
    hyozu::data(vec![
        xy(observed).with_name("Species").into(),
        line(fit).with_name("M^0.75 fit").into(),
    ])
    .x_log()
    .y_log()
    .x_axis(|axis| axis.show_minor_grid(true))
    .y_axis(|axis| axis.show_minor_grid(true))
    .title("Log–log")
}

/// An ocean temperature profile: depth grows downward on a reversed axis.
pub fn reversed_depth() -> hyozu::Data {
    let profile = [
        (28.1, 0.0),
        (27.9, 25.0),
        (27.2, 50.0),
        (24.0, 75.0),
        (19.5, 100.0),
        (15.2, 150.0),
        (12.4, 200.0),
        (9.1, 300.0),
        (6.8, 500.0),
        (5.0, 700.0),
        (4.2, 1000.0),
        (3.1, 1500.0),
        (2.4, 2000.0),
    ];
    hyozu::data(line(profile).with_name("Temperature"))
        .x_axis_labels(|v: f64| format!("{v:.0} °C"))
        .y_axis(|axis| axis.reverse().labels(|v: f64| format!("{v:.0} m")))
        .title("Reversed depth")
}

/// Long category names stay readable when the labels turn.
pub fn rotated_labels() -> hyozu::Data {
    hyozu::data(bars([
        bar([48.2, 36.5, 29.8, 22.1, 17.4, 12.9, 8.3]).with_name("Revenue")
    ]))
    .x_axis_labels([
        "Enterprise licenses",
        "Professional services",
        "Cloud subscriptions",
        "Support renewals",
        "Training & certification",
        "Marketplace apps",
        "Hardware appliances",
    ])
    .x_axis(|axis| axis.label_angle(45.0))
    .y_axis_labels(|v: f64| format!("${v:.0}M"))
    .title("Rotated labels")
}

/// Axes that cross at the origin, the way a maths textbook draws them.
pub fn origin_axes() -> hyozu::Data {
    let wave: Vec<(f64, f64)> = (0..=120)
        .map(|i| {
            let x = -6.0 + 12.0 * f64::from(i) / 120.0;
            (x, 2.0 * x.sin() + 0.4 * x)
        })
        .collect();
    hyozu::data(line(wave).with_name("2 sin x + 0.4x"))
        .x_axis(|axis| axis.crosses_at(0.0).show_grid(false))
        .y_axis(|axis| axis.crosses_at(0.0).show_grid(false))
        .title("Origin axes")
}

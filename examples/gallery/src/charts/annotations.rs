use hyozu::annotation::{Connector, End, Subject};
use hyozu::{Color, Data, TextAlign, annotation, bar, bars, data, line, xy};

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// A light plate on light themes, a dark one on dark themes.
fn plate() -> Color {
    Color::themed(
        hyozu::core::Color::from_rgba8(255, 255, 255, 0.92),
        hyozu::core::Color::from_rgba8(28, 30, 36, 0.92),
    )
}

/// Callouts on a revenue chart: entry targets follow the bars and the
/// forecast line; coordinate targets mark a threshold and a shortage.
pub fn callouts() -> Data {
    let revenue = [31.0, 34.0, 22.0, 29.0, 35.0, 38.0, 52.0, 41.0, 39.0, 43.0, 45.0, 48.0];
    let forecast = [30.0, 33.0, 35.0, 33.0, 36.0, 39.0, 41.0, 42.0, 41.0, 44.0, 47.0, 50.0];
    data(vec![
        bars([bar(revenue).with_name("Revenue")]).into(),
        line(forecast).with_name("Forecast").into(),
        annotation::entry(0, 0, 6)
            .connector(Connector::Elbow)
            .end(End::Arrow)
            .title("Record July")
            .note("Promotion and v2 launch")
            .background(plate())
            .offset(-90.0, -18.0)
            .color(Color::Danger)
            .into(),
        annotation(2.0, 22.0)
            .subject(Subject::rect(60.0, 44.0))
            .end(End::Dot)
            .note("Supply shortage cut shipments for three weeks")
            .wrap(120.0)
            .align(TextAlign::Left)
            .offset(-10.0, -150.0)
            .into(),
        annotation::entry(1, 0, 3)
            .connector(Connector::Curve)
            .end(End::Dot)
            .note("Forecast dip")
            .offset(30.0, -45.0)
            .into(),
        annotation(10.0, 50.0)
            .subject(Subject::threshold(34.0))
            .connector(Connector::None)
            .note("Q4 target")
            .offset(0.0, -6.0)
            .color(0x2E8B57)
            .into(),
        annotation(30.0, 20.0)
            .subject(Subject::circle(8.0))
            .note("Outside the axes: hidden")
            .into(),
    ])
    .x_axis_labels(MONTHS)
    .y_axis_labels(|v: f64| format!("${v:.0}k"))
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Callouts")
}

/// Callouts on log axes land where the log mapping puts their data.
pub fn log_callouts() -> Data {
    let species: [(f64, &str); 8] = [
        (0.02, "Mouse"),
        (0.3, "Rat"),
        (3.0, "Cat"),
        (30.0, "Dog"),
        (70.0, "Human"),
        (600.0, "Horse"),
        (4000.0, "Elephant"),
        (12.0, "Beaver"),
    ];
    let observed: Vec<(f64, f64)> = species
        .iter()
        .zip([1.05, 0.92, 0.97, 0.95, 1.02, 1.08, 0.97, 1.04])
        .map(|(&(mass, _), noise)| (mass, 3.4 * mass.powf(0.75) * noise))
        .collect();
    let elephant = observed[6];
    let fit: Vec<(f64, f64)> = [0.01, 10_000.0].map(|mass: f64| (mass, 3.4 * mass.powf(0.75))).to_vec();
    data(vec![
        xy(observed).with_name("Species").into(),
        line(fit).with_name("M^0.75 fit").into(),
        annotation::entry(0, 0, 0)
            .subject(Subject::circle(8.0))
            .end(End::Arrow)
            .note("Mouse, 20 g")
            .offset(30.0, -40.0)
            .into(),
        annotation(elephant.0, elephant.1)
            .subject(Subject::Crosshair)
            .connector(Connector::Elbow)
            .title("Elephant")
            .note(format!("{:.0} kg, {:.0} W", elephant.0, elephant.1))
            .background(plate())
            .offset(-90.0, 110.0)
            .color(Color::Primary)
            .into(),
        annotation(10.0, 3.4 * 10f64.powf(0.75))
            .subject(Subject::circle(5.0))
            .connector(Connector::Curve)
            .end(End::Dot)
            .note("10 kg on the fit")
            .offset(50.0, 45.0)
            .into(),
    ])
    .x_log()
    .y_log()
    .x_axis(|axis| axis.show_minor_grid(true))
    .y_axis(|axis| axis.show_minor_grid(true))
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Log-axis callouts")
}

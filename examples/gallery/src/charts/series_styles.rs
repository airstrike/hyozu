use hyozu::bullet::{self, Ranges};
use hyozu::line::{Connector, Interpolation, Nulls, Step, UpDown, marker};
use hyozu::pattern::{Hatch, Pattern};
use hyozu::{Data, LineStyle, Mark, area, areas, bar, bars, data, line, pie};

const MONTHS: [&str; 8] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug"];

pub fn interpolation() -> Data {
    let shape = [2.0, 2.0, 6.5, 3.0, 3.0, 8.0, 7.6, 4.0];
    let modes: [(&str, Interpolation); 6] = [
        ("Linear", Interpolation::Linear),
        ("Monotone", Interpolation::Monotone),
        ("Natural", Interpolation::Natural),
        ("Step before", Step::Before.into()),
        ("Step after", Step::After.into()),
        ("Step mid", Step::Mid.into()),
    ];
    let marks: Vec<Mark> = modes
        .into_iter()
        .enumerate()
        .map(|(k, (name, mode))| {
            let lift = (5 - k) as f64 * 7.0;
            line(shape.map(|y| y + lift))
                .interpolation(mode)
                .markers(marker::Shape::Circle)
                .with_name(name)
                .into()
        })
        .collect();
    data(marks)
        .x_axis_labels(MONTHS)
        .y_axis(|a| a.show_labels(false))
        .legend(hyozu::legend::Anchor::Right)
        .title("Line interpolation")
}

pub fn empty_cells() -> Data {
    let gap = f64::NAN;
    let values = [4.0, 5.5, gap, 6.0, 7.5, gap, gap, 6.5];
    let policies = [("Gap", Nulls::Gap), ("Zero", Nulls::Zero), ("Connect", Nulls::Connect)];
    let marks: Vec<Mark> = policies
        .into_iter()
        .enumerate()
        .map(|(k, (name, nulls))| {
            let lift = (2 - k) as f64 * 9.0;
            line(values.map(|y| y + lift))
                .nulls(nulls)
                .markers(marker::Shape::Circle)
                .with_name(name)
                .into()
        })
        .collect();
    data(marks)
        .x_axis_labels(MONTHS)
        .legend(hyozu::legend::Anchor::Bottom)
        .title("Empty cells: gap, zero and connect")
}

pub fn hatched_bars() -> Data {
    data(
        bars!(
            bar([42, 55, 48, 61]).with_name("Plan").pattern(Hatch::Upward),
            bar([38, 59, 51, 57]).with_name("Actual"),
            bar([12, 18, 15, 21])
                .with_name("Forecast")
                .pattern(Pattern::new(Hatch::Crosshatch).spacing(5.0)),
        )
        .data_labels(None),
    )
    .x_axis_labels(["Q1", "Q2", "Q3", "Q4"])
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Hatched bar fills")
}

pub fn hatched_area() -> Data {
    data(
        areas([
            area([12, 15, 14, 18, 22, 21, 25, 27]).with_name("Committed"),
            area([4, 6, 5, 7, 6, 9, 8, 10])
                .with_name("Best case")
                .pattern(Pattern::new(Hatch::Downward).spacing(5.0)),
            area([2, 3, 4, 3, 5, 4, 6, 5])
                .with_name("Upside")
                .pattern(Pattern::new(Hatch::Vertical).spacing(4.0)),
        ])
        .stacked()
        .interpolation(Interpolation::Monotone),
    )
    .x_axis_labels(MONTHS)
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Hatched stacked areas, monotone")
}

pub fn hatched_pie() -> Data {
    data(pie([
        hyozu::mark::pie::slice(38.0).name("Hardware"),
        hyozu::mark::pie::slice(27.0).name("Services").pattern(Hatch::Upward),
        hyozu::mark::pie::slice(21.0).name("Licenses").pattern(Hatch::Grid),
        hyozu::mark::pie::slice(14.0)
            .name("Other")
            .pattern(Pattern::new(Hatch::Horizontal).spacing(4.0)),
    ]))
    .legend(hyozu::legend::Anchor::Right)
    .title("Hatched slices")
}

pub fn drop_and_high_low() -> Data {
    data(
        line::group([
            line([62.0, 66.0, 61.0, 70.0, 74.0, 69.0, 77.0, 80.0]).with_name("North"),
            line([54.0, 59.0, 63.0, 60.0, 65.0, 71.0, 68.0, 73.0]).with_name("South"),
            line([48.0, 51.0, 55.0, 57.0, 56.0, 60.0, 63.0, 61.0]).with_name("West"),
        ])
        .high_low_lines(Connector::new().width(1.5))
        .drop_lines(Connector::new().style(LineStyle::Dotted)),
    )
    .x_axis_labels(MONTHS)
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Drop lines and high-low lines")
}

pub fn up_down_bars() -> Data {
    data(
        line::group([
            line([20.0, 23.5, 22.0, 25.0, 24.0, 21.5, 26.0, 28.0]).with_name("Open"),
            line([24.5, 26.0, 26.5, 27.5, 25.0, 25.5, 29.0, 31.0]).with_name("High"),
            line([18.5, 21.0, 20.5, 22.0, 20.0, 19.5, 23.0, 26.0]).with_name("Low"),
            line([23.0, 22.5, 25.5, 24.5, 21.0, 25.0, 28.5, 27.0]).with_name("Close"),
        ])
        .high_low_lines(Connector::new())
        .up_down_bars(UpDown::new().gap(100)),
    )
    .x_axis_labels(MONTHS)
    .legend(hyozu::legend::Anchor::Bottom)
    .title("Up/down bars between open and close")
}

pub fn bullet_chart() -> Data {
    let ranges = |bounds: &[f64]| Ranges::new(bounds).expect("ascending positive bounds");
    data(bullet::bullet([
        bullet::row("Revenue", 270.0)
            .target(250.0)
            .ranges(ranges(&[150.0, 225.0, 300.0])),
        bullet::row("Profit", 210.0)
            .target(260.0)
            .ranges(ranges(&[140.0, 200.0, 300.0])),
        bullet::row("New customers", 160.0)
            .target(190.0)
            .ranges(ranges(&[120.0, 180.0, 240.0])),
        bullet::row("Satisfaction", 225.0)
            .target(230.0)
            .ranges(ranges(&[175.0, 210.0, 250.0])),
    ]))
    .title("Bullet chart")
}

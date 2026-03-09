use hyozu::bars;
use hyozu::mark::bar::label::Position;

pub fn data() -> hyozu::Data {
    hyozu::data(bars([85, 72, 64, 53, 41]).data_labels(Position::End + (|v: f64| format!("{v:.0}%"))))
        .x_axis_labels(["Chrome", "Safari", "Firefox", "Edge", "Other"])
        .title("Browser Market Share")
}

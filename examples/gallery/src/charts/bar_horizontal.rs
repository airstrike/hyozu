use hyozu::bars;

pub fn data() -> hyozu::Data {
    hyozu::data(bars([120, 190, 150, 80, 210]).horizontal())
        .title("Sales by Region")
        .y_axis_labels(["North", "South", "East", "West", "Central"])
}

/// Market caps that span four decades read on a log value axis: `y_log`
/// logs the values, which horizontal bars lay along the bottom.
pub fn log_values() -> hyozu::Data {
    hyozu::data(bars([2_950.0, 410.0, 63.0, 8.2, 0.9]).horizontal())
        .y_log()
        .y_axis_labels(["Mega cap", "Large cap", "Mid cap", "Small cap", "Micro cap"])
        .x_axis_labels(|v: f64| format!("${v}B"))
        .title("Log value axis")
}

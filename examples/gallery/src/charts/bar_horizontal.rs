use hyozu::bars;

pub fn data() -> hyozu::Data {
    hyozu::data(bars([120, 190, 150, 80, 210]).horizontal())
        .title("Sales by Region")
        .y_axis_labels(["North", "South", "East", "West", "Central"])
}

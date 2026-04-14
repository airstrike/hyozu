use hyozu::bars;

pub fn data() -> hyozu::Data {
    hyozu::data(bars([1200, 1900, 1500, 2200, 1800, 2400]))
        .x_axis_labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
        .title("Monthly Sales")
}

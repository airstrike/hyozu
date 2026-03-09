use hyozu::bars;

pub fn data() -> hyozu::Data {
    hyozu::data(
        bars!([320, 410, 390, 480, 520, 610], [220, 280, 310, 350, 290, 370], [
            150, 190, 170, 210, 230, 260
        ],)
        .stacked(),
    )
    .x_axis_labels(["Q1", "Q2", "Q3", "Q4", "Q5", "Q6"])
    .title("Revenue by Segment")
}

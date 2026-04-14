use hyozu::{area, areas};

pub fn data() -> hyozu::Data {
    hyozu::data(
        areas([
            area([2, 4, 3, 5, 4, 6, 5]).with_name("Revenue"),
            area([1, 2, 2, 3, 2, 4, 3]).with_name("Expenses"),
        ])
        .stacked(),
    )
    .title("Revenue vs Expenses")
}

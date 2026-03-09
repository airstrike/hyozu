use hyozu::{boxplot, entry_from_data};

pub fn data() -> hyozu::Data {
    hyozu::data(boxplot(vec![
        entry_from_data(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 20.0]).with_name("Sample A"),
        entry_from_data(&[3.0, 5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 8.0, 9.0, 12.0]).with_name("Sample B"),
    ]))
    .title("Distribution Comparison")
    .x_axis_labels(["A", "B"])
}

use hyozu::heatmap;

pub fn data() -> hyozu::Data {
    #[rustfmt::skip]
    let values: Vec<f64> = vec![
        22.0, 24.0, 28.0, 32.0, 30.0, 26.0,
        20.0, 22.0, 26.0, 30.0, 28.0, 24.0,
        18.0, 20.0, 24.0, 28.0, 26.0, 22.0,
        16.0, 18.0, 22.0, 26.0, 24.0, 20.0,
    ];

    hyozu::data(
        heatmap(values, 4, 6)
            .col_names(["Jan", "Mar", "May", "Jul", "Sep", "Nov"])
            .row_names(["North", "South", "East", "West"])
            .show_labels(true)
            .label_format(|v| format!("{v:.0}")),
    )
    .title("Regional Temperature")
}

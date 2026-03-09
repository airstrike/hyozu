use hyozu::heatmap;

pub fn data() -> hyozu::Data {
    #[rustfmt::skip]
    let values: Vec<f64> = vec![
        0.0, 0.0, 3.0, 1.0, 0.0, 2.0, 0.0,
        1.0, 0.0, 5.0, 8.0, 2.0, 0.0, 0.0,
        0.0, 4.0, 7.0,12.0, 6.0, 1.0, 0.0,
        2.0, 6.0,11.0,15.0, 9.0, 3.0, 0.0,
        0.0, 3.0, 8.0,14.0,10.0, 5.0, 1.0,
        1.0, 2.0, 6.0, 9.0, 7.0, 4.0, 0.0,
    ];

    hyozu::data(
        heatmap(values, 6, 7)
            .col_names(["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"])
            .row_names(["W1", "W2", "W3", "W4", "W5", "W6"])
            .show_labels(true)
            .label_format(|v| if v < 0.5 { String::new() } else { format!("{v:.0}") })
            .warm(),
    )
    .title("Weekly Activity")
}

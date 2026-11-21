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
            .label_format(|v| format!("{v:.0}"))
            .scale_legend(hyozu::legend::Config::below())
            .legend_title("Mean temperature (°C)"),
    )
    .title("Regional Temperature")
}

/// Request latency spans three orders of magnitude, so the color scale is
/// logarithmic and the color bar reads in decades.
pub fn log_scale() -> hyozu::Data {
    #[rustfmt::skip]
    let values: Vec<f64> = vec![
          4.0,   6.0,  12.0,   35.0,  18.0,   7.0,
          5.0,   9.0,  28.0,   90.0,  41.0,  11.0,
          8.0,  22.0, 140.0,  620.0, 210.0,  30.0,
          6.0,  15.0,  75.0, 2400.0, 380.0,  19.0,
    ];

    hyozu::data(
        heatmap(values, 4, 6)
            .col_names(["00h", "04h", "08h", "12h", "16h", "20h"])
            .row_names(["Edge", "API", "Search", "Billing"])
            .log()
            .warm()
            .scale_legend(hyozu::legend::Config::right())
            .legend_title("p99 latency (ms)"),
    )
    .title("Service Latency")
}

use hyozu::gauge;

pub fn data() -> hyozu::Data {
    hyozu::data(
        gauge(72.0)
            .range(0.0, 100.0)
            .format(|v| format!("{v:.0}%"))
            .unit("complete"),
    )
    .title("Project Progress")
}

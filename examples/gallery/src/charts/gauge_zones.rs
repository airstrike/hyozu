use hyozu::gauge;

pub fn data() -> hyozu::Data {
    hyozu::data(
        gauge(65.0)
            .range(0.0, 100.0)
            .zones([
                (30.0, 0xCC3333),  // red
                (70.0, 0xE6B20A),  // yellow
                (100.0, 0x33B24D), // green
            ])
            .format(|v| format!("{v:.0}%"))
            .unit("performance")
            .ticks(hyozu::axis::Ticks::default())
            .gradient(true),
    )
    .title("Performance Monitor")
}

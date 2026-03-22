use hyozu::gauge;

pub fn data() -> hyozu::Data {
    let red = iced::Color::from_rgb(0.8, 0.2, 0.2);
    let yellow = iced::Color::from_rgb(0.9, 0.7, 0.1);
    let green = iced::Color::from_rgb(0.2, 0.7, 0.3);

    hyozu::data(
        gauge(65.0)
            .range(0.0, 100.0)
            .zone(30.0, red)
            .zone(70.0, yellow)
            .zone(100.0, green)
            .format(|v| format!("{v:.0}%"))
            .unit("performance")
            .ticks(hyozu::axis::tick::Ticks::default())
            .gradient(true),
    )
    .title("Performance Monitor")
}

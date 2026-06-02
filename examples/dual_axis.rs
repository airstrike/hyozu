use iced::widget::center;
use iced::{Task, Theme};

use hyozu::mark::bar::label::Position as BarLabelPos;
use hyozu::mark::line::label::Position as LineLabelPos;
use hyozu::{LineStyle, Mark, bars, data, line};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([800.0, 500.0])
        .title("hyozu • dual axis")
        .theme(App::theme)
        .run()
}

struct App {
    chart: hyozu::Data,
}

pub type Message = ();

impl App {
    fn new() -> Self {
        let revenue = bars([1200, 1900, 1500, 2200, 1800, 2400]).data_labels(BarLabelPos::Above + currency);
        let conversion_rate = line([0.12, 0.18, 0.15, 0.22, 0.19, 0.25])
            .style(LineStyle::Dashed)
            .data_labels(LineLabelPos::Auto + percent);

        Self {
            chart: data(revenue)
                .x_axis_labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
                .y_axis_labels(currency)
                .secondary(vec![Mark::Line(conversion_rate)])
                .right_axis(|a| a.title("Conversion").labels(percent).with_bounds(0.0, 1.0))
                .title("Revenue with Conversion Rate"),
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(
            hyozu::chart(&self.chart)
                .design(&Theme::SolarizedLight)
                .padding(20)
                .style(|d| hyozu::chart::Style {
                    corners: 4.0.into(),
                    ..hyozu::chart::default(d)
                }),
        )
        .padding(20)
        .into()
    }

    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }

    fn theme(&self) -> Theme {
        Theme::SolarizedLight
    }
}

/// Formats a number as `$#,##0` — thousands-separated integer dollars.
///
/// Mirrors the helper used in `examples/interactive.rs` and
/// `examples/btc_stream`. Negative values become `-$1,234`.
fn currency(value: f64) -> String {
    let whole = value as i64;
    let s = whole.abs().to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, ch);
    }
    if whole < 0 {
        format!("-${result}")
    } else {
        format!("${result}")
    }
}

/// Formats a decimal fraction (0.0–1.0) as a whole-number percentage.
/// Shared between the right-axis label formatter and the line's data
/// labels so they always agree.
fn percent(v: f64) -> String {
    format!("{:.0}%", v * 100.0)
}

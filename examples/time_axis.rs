//! Test example for time axis at various scales
//!
//! Tests hierarchical time intervals from minutes to years.
//! Each time range uses appropriate data granularity.

use iced::widget::{button, column, container, row, text};
use iced::{Fill, Task};

use hyozu::axis::Alignment;
use hyozu::{chart, data, line};

fn format_price(value: f64) -> String {
    format!("${:.2}", value)
}

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu - time axis test")
        .theme(hyozu::theme::hyozu())
        .window_size([800.0, 500.0])
        .centered()
        .run()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TimeRange {
    #[default]
    OneHour,
    SixHours,
    OneDay,
    OneWeek,
    OneMonth,
    SixMonths,
    OneYear,
    FiveYears,
}

impl TimeRange {
    fn label(&self) -> &'static str {
        match self {
            TimeRange::OneHour => "1 Hour",
            TimeRange::SixHours => "6 Hours",
            TimeRange::OneDay => "1 Day",
            TimeRange::OneWeek => "1 Week",
            TimeRange::OneMonth => "1 Month",
            TimeRange::SixMonths => "6 Months",
            TimeRange::OneYear => "1 Year",
            TimeRange::FiveYears => "5 Years",
        }
    }

    /// Returns (duration_secs, point_interval_secs, description)
    fn config(&self) -> (i64, i64, &'static str) {
        match self {
            // 1 hour: 1 point per minute (60 points)
            TimeRange::OneHour => (3600, 60, "minute"),
            // 6 hours: 1 point per 5 minutes (72 points)
            TimeRange::SixHours => (6 * 3600, 5 * 60, "5 minutes"),
            // 1 day: 1 point per 30 minutes (48 points)
            TimeRange::OneDay => (24 * 3600, 30 * 60, "30 minutes"),
            // 1 week: 1 point per 4 hours (42 points)
            TimeRange::OneWeek => (7 * 24 * 3600, 4 * 3600, "4 hours"),
            // 1 month: 1 point per day (30 points)
            TimeRange::OneMonth => (30 * 24 * 3600, 24 * 3600, "day"),
            // 6 months: 1 point per week (~26 points)
            TimeRange::SixMonths => (180 * 24 * 3600, 7 * 24 * 3600, "week"),
            // 1 year: 1 point per week (~52 points)
            TimeRange::OneYear => (365 * 24 * 3600, 7 * 24 * 3600, "week"),
            // 5 years: 1 point per week (~260 points)
            TimeRange::FiveYears => (5 * 365 * 24 * 3600, 7 * 24 * 3600, "week"),
        }
    }

    fn all() -> &'static [TimeRange] {
        &[
            TimeRange::OneHour,
            TimeRange::SixHours,
            TimeRange::OneDay,
            TimeRange::OneWeek,
            TimeRange::OneMonth,
            TimeRange::SixMonths,
            TimeRange::OneYear,
            TimeRange::FiveYears,
        ]
    }
}

struct App {
    range: TimeRange,
    alignment: Alignment,
    show: line::label::Show,
    data: data::Data,
}

#[derive(Debug, Clone)]
enum Message {
    Range(TimeRange),
    Alignment(Alignment),
    Show(line::label::Show),
}

impl App {
    fn new() -> Self {
        let range = TimeRange::default();
        let alignment = Alignment::SnapToEnd;
        let show = line::label::Show::LastOnly;
        Self {
            data: Self::build_data(range, alignment, show),
            range,
            alignment,
            show,
        }
    }

    fn build_data(range: TimeRange, alignment: Alignment, show: line::label::Show) -> data::Data {
        // End timestamp: 2024-06-15 12:00:00 UTC
        let end_ts: i64 = 1718452800;
        let (duration, interval, _desc) = range.config();
        let start_ts = end_ts - duration;

        // Generate geometric brownian motion from a fixed origin
        // so all time ranges share the same underlying price path
        let points = generate_gbm(start_ts, end_ts, interval);

        data(line(points).data_labels(show + line::label::Position::Right + format_price))
            .x_axis(|_| line::Line::time_axis().with_ticks(alignment))
            .y_axis_labels(format_price)
    }

    fn rebuild_data(&mut self) {
        self.data = Self::build_data(self.range, self.alignment, self.show);
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Range(range) => {
                self.range = range;
                self.rebuild_data();
            }
            Message::Alignment(alignment) => {
                self.alignment = alignment;
                if let Some(axis) = self.data.x_axis_mut() {
                    axis.ticks_mut().alignment = alignment;
                }
            }
            Message::Show(show) => {
                self.show = show;
                if let Some(label) = self.data.line_mut(0).and_then(|l| l.label_mut()) {
                    label.show = show;
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> iced::Element<'_, Message> {
        use line::label::Show;

        let range_buttons = row(TimeRange::all()
            .iter()
            .map(|&r| btn(r.label(), self.range == r, Message::Range(r))))
        .spacing(4);

        let alignments = [
            (Alignment::Auto, "Auto"),
            (Alignment::SnapToStart, "Start"),
            (Alignment::SnapToEnd, "End"),
        ];
        let alignment_buttons = row(alignments
            .iter()
            .map(|&(a, label)| btn(label, self.alignment == a, Message::Alignment(a))))
        .spacing(4);

        let shows = [
            (Show::Any, "Any"),
            (Show::FirstOnly, "First"),
            (Show::LastOnly, "Last"),
            (Show::FirstAndLast, "First+Last"),
            (Show::MinMaxFirst, "Min/Max"),
        ];
        let show_buttons = row(shows
            .iter()
            .map(|&(s, label)| btn(label, self.show == s, Message::Show(s))))
        .spacing(4);

        let controls = column![
            row![text("Range:").size(12).width(50), range_buttons]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            row![
                row![text("Ticks:").size(12).width(50), alignment_buttons]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
                row![text("Show:").size(12).width(50), show_buttons]
                    .spacing(8)
                    .align_y(iced::Alignment::Center),
            ]
            .spacing(16),
        ]
        .spacing(8);

        let content = column![
            controls,
            chart(&self.data).padding(iced::padding::all(10).right(10 + 50))
        ]
        .spacing(20);

        container(content).width(Fill).height(Fill).padding(20).into()
    }
}

fn btn<'a, Message: Clone + 'a>(label: &'a str, selected: bool, msg: Message) -> iced::Element<'a, Message> {
    button(text(label).size(12))
        .padding([6, 12])
        .style(move |theme, status| {
            if selected {
                button::primary(theme, button::Status::Active)
            } else {
                button::subtle(theme, status)
            }
        })
        .on_press(msg)
        .into()
}

/// Simple deterministic RNG for reproducible price simulation.
/// Uses a linear congruential generator with Box-Muller for Gaussian.
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        // LCG parameters from Numerical Recipes
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() as f64) / (u64::MAX as f64)
    }

    /// Box-Muller transform for Gaussian distribution
    fn next_gaussian(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-10); // Avoid log(0)
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

/// Generate geometric brownian motion (stock-like) price series.
///
/// Uses a fixed origin timestamp so all time ranges see consistent
/// prices (zooming out shows the same data at lower resolution).
fn generate_gbm(start_ts: i64, end_ts: i64, interval: i64) -> Vec<(i64, f64)> {
    // Fixed origin: 2019-01-01 00:00:00 UTC (gives us 5+ years of history)
    const ORIGIN_TS: i64 = 1546300800;
    const BASE_PRICE: f64 = 50.0;

    // Use minute-level steps for the underlying simulation
    // This ensures consistency across different interval views
    const STEP: i64 = 60;

    // Parameters for geometric brownian motion (per-minute)
    // Annual drift ~8%, volatility ~20% (typical stock)
    // Per-minute: drift/525600, vol/sqrt(525600)
    let drift = 0.08 / 525600.0;
    let volatility = 0.20 / 725.0; // sqrt(525600) ≈ 725

    let mut price = BASE_PRICE;
    let mut rng = SimpleRng::new(42); // Fixed seed for reproducibility

    // Simulate from origin to end_ts
    let mut ts = ORIGIN_TS;
    let mut result = Vec::new();

    while ts <= end_ts {
        // Store point if it's in our range and aligned to end_ts
        // (so all time ranges end at the same price)
        if ts >= start_ts && (end_ts - ts) % interval == 0 {
            result.push((ts, price));
        }

        // Geometric brownian motion step
        let random = rng.next_gaussian();
        let return_pct = drift + volatility * random;
        price *= 1.0 + return_pct;

        ts += STEP;
    }

    result
}

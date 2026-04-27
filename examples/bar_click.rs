//! Minimal smoke test for bar-click → `Action::Clicked(Target::Entry)`.
//!
//! Renders a grouped bar chart (two series × six months), wires
//! `on_action`, and shows the most recent click target in a status
//! label below the chart. Confirms end-to-end that:
//!
//!   - clicking a bar emits `Target::Entry { mark, series, index }`,
//!     where `series` discriminates Revenue vs Expenses and `index` is
//!     the month position;
//!   - clicking a bar's data label emits `Target::EntryLabel { … }` with
//!     the same triple;
//!   - clicking the chart background emits `Target::Mark(usize::MAX)`.
//!
//! Use as a copy-from reference when wiring drill-on-bar in a
//! tatami-backed dashboard: map the reported `index` back to your
//! `series_result.x()[index]` `MemberRef` to pin into the slicer.

use iced::widget::{column, container, text};
use iced::{Center, Element, Length, Task};

use hyozu::bar::label;
use hyozu::data::Action;
use hyozu::target::Target;
use hyozu::{Palette, bar, bars, chart, data};

const MONTHS: [&str; 6] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([720.0, 520.0])
        .title("hyozu • bar click")
        .run()
}

struct App {
    sales: hyozu::Data,
    last_target: Option<Target>,
}

#[derive(Debug, Clone)]
enum Message {
    ChartAction(Action),
}

impl App {
    fn new() -> Self {
        Self {
            sales: data(
                bars([
                    bar([1200, 1900, 1500, 2200, 1800, 2400]).with_name("Revenue"),
                    bar([800, 1100, 950, 1400, 1050, 1500]).with_name("Expenses"),
                ])
                .corner_radius(4.0)
                .data_labels(label::Position::Above),
            )
            .palette(Palette::Categorical)
            .x_axis_labels(MONTHS)
            .y_axis(|a| a.show_grid(true)),
            last_target: None,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChartAction(action) => {
                self.sales.perform(action.clone());
                if let Action::Clicked(target) = action {
                    self.last_target = Some(target);
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let status = match &self.last_target {
            None => "click any bar or label".to_string(),
            Some(Target::Entry { mark, series, index }) => {
                let name = series_name(*series);
                format!(
                    "Bar:    mark={mark}, series={series} ({name}), index={index} ({})",
                    MONTHS[*index]
                )
            }
            Some(Target::EntryLabel { mark, series, index }) => {
                let name = series_name(*series);
                format!(
                    "Label:  mark={mark}, series={series} ({name}), index={index} ({})",
                    MONTHS[*index]
                )
            }
            Some(Target::Mark(m)) if *m == usize::MAX => "Background click (no element hit)".to_string(),
            Some(other) => format!("Other: {other:?}"),
        };

        let chart = chart(&self.sales).on_action(Message::ChartAction).padding(20);

        container(
            column![chart, text(status).size(13)]
                .spacing(12)
                .align_x(Center)
                .width(Length::Fill),
        )
        .padding(16)
        .into()
    }
}

fn series_name(idx: usize) -> &'static str {
    match idx {
        0 => "Revenue",
        1 => "Expenses",
        _ => "?",
    }
}

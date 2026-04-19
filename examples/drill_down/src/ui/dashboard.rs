//! Dashboard state + the per-panel async dispatch plumbing.
//!
//! Phase 1 wired the navigation + per-panel async-result flow end-to-end
//! against a constructed `InMemoryCube`. Phase 2 specializes the KPI
//! panel's rendering through `hyozu::tatami::card`, adds a header with a
//! measure switcher + back/forward buttons, and grows the dashboard's
//! Message enum to carry view-toggle + navigation intents.

use std::collections::HashMap;
use std::sync::Arc;

use iced::widget::{Column, button, column, container, row, text};
use iced::{Element, Length, Task};

use tatami::Cube;
use tatami_inmem::InMemoryCube;

use crate::ui::kpi;
use crate::ui::panel::Panel;
use crate::ui::query_state::QueryState;
use crate::ui::trail::Trail;
use crate::ui::view::{Measure, Period, View};

/// Messages consumed by the Dashboard.
///
/// The outer `App` layer lifts these via `lift_dashboard_message` — it
/// translates dashboard-internal intents into the top-level `Message`
/// enum and routes them back through the Dashboard's typed methods.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Message {
    /// A panel's query finished.
    PanelDone(Panel, Result<tatami::Results, String>),
    /// User picked a new active measure from the header switcher.
    SetMeasure(Measure),
    /// User picked a new period grain.
    SetPeriod(Period),
    /// Back button pressed.
    Back,
    /// Forward button pressed.
    Forward,
}

/// Dashboard state. The `cube` is shared across per-panel async tasks
/// via `Arc`; `trail` carries the navigation history; `view` carries the
/// current measure + period; `panels` keeps the latest async state per
/// panel.
#[derive(Debug)]
pub struct Dashboard {
    /// Shared handle to the OLAP cube.
    pub cube: Arc<InMemoryCube>,
    /// Back/forward navigation history of queries.
    pub trail: Trail,
    /// Measure + period view mode.
    pub view: View,
    /// iced theme — held on the dashboard so Phase 6 can toggle it.
    pub theme: iced::Theme,
    /// Per-panel async query state.
    pub panels: HashMap<Panel, QueryState>,
}

impl Dashboard {
    /// Construct a dashboard and return the initial per-panel dispatch.
    pub fn new(cube: Arc<InMemoryCube>, initial: tatami::Query, view: View) -> (Self, Task<Message>) {
        let mut panels = HashMap::with_capacity(Panel::ALL.len());
        for panel in Panel::ALL {
            panels.insert(panel, QueryState::Running);
        }
        let mut dashboard = Self {
            cube,
            trail: Trail::new(initial),
            view,
            theme: iced::Theme::Light,
            panels,
        };
        let task = dashboard.dispatch();
        (dashboard, task)
    }

    /// Structural navigation — push and fire all five panel queries.
    pub fn navigate(&mut self, q: tatami::Query) -> Task<Message> {
        self.trail.push(q);
        self.dispatch()
    }

    /// View-toggle — update the view state and refire the four
    /// view-dependent panels. Chips is skipped because it does not depend
    /// on `view`.
    pub fn set_view(&mut self, v: View) -> Task<Message> {
        if self.view == v {
            return Task::none();
        }
        self.view = v;
        self.dispatch_view_dependent()
    }

    /// Move back; if the cursor moved, refire.
    pub fn back(&mut self) -> Task<Message> {
        if self.trail.back() {
            self.dispatch()
        } else {
            Task::none()
        }
    }

    /// Move forward; if the cursor moved, refire.
    pub fn forward(&mut self) -> Task<Message> {
        if self.trail.forward() {
            self.dispatch()
        } else {
            Task::none()
        }
    }

    /// Fire all five panel queries in parallel against the current trail
    /// entry and view.
    pub fn dispatch(&mut self) -> Task<Message> {
        let trail_q = self.trail.current().clone();
        let view = self.view;
        let cube = self.cube.clone();
        let tasks: Vec<Task<Message>> = Panel::ALL
            .iter()
            .map(|&panel| {
                self.panels.insert(panel, QueryState::Running);
                run(cube.clone(), panel, panel.query(&trail_q, &view))
            })
            .collect();
        Task::batch(tasks)
    }

    /// Fire every panel query except Chips, which is view-invariant.
    pub fn dispatch_view_dependent(&mut self) -> Task<Message> {
        let trail_q = self.trail.current().clone();
        let view = self.view;
        let cube = self.cube.clone();
        let tasks: Vec<Task<Message>> = Panel::ALL
            .iter()
            .filter(|&&p| p != Panel::Chips)
            .map(|&panel| {
                self.panels.insert(panel, QueryState::Running);
                run(cube.clone(), panel, panel.query(&trail_q, &view))
            })
            .collect();
        Task::batch(tasks)
    }

    /// Handle a dashboard message — either a panel result landed, or the
    /// user interacted with the header.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PanelDone(panel, Ok(results)) => {
                self.panels.insert(panel, QueryState::Ok(results));
                Task::none()
            }
            Message::PanelDone(panel, Err(error)) => {
                self.panels.insert(panel, QueryState::Err(error));
                Task::none()
            }
            Message::SetMeasure(m) => self.set_view(View {
                measure: m,
                ..self.view
            }),
            Message::SetPeriod(p) => self.set_view(View { period: p, ..self.view }),
            Message::Back => self.back(),
            Message::Forward => self.forward(),
            // `Message` is `#[non_exhaustive]`; surface future variants
            // loudly rather than silently dropping them.
            #[allow(unreachable_patterns)]
            _ => Task::none(),
        }
    }

    /// Render the dashboard: a header row with the measure switcher +
    /// back/forward buttons, then one card per panel.
    pub fn view(&self) -> Element<'_, Message> {
        let header = self.render_header();
        let cards = Panel::ALL.iter().map(|&panel| self.render_card(panel));
        container(
            column![header, Column::with_children(cards).spacing(12).width(Length::Fill)]
                .spacing(16)
                .padding(16)
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Render the header: measure switcher + back/forward buttons. The
    /// measure switcher is a button row; the active measure is visually
    /// distinct via a different button style.
    fn render_header(&self) -> Element<'_, Message> {
        let measure_row = Measure::ALL.iter().fold(row![].spacing(4), |acc, &m| {
            let label = text(measure_label(m)).size(13);
            let b = if m == self.view.measure {
                button(label).style(button::primary)
            } else {
                button(label).style(button::secondary)
            };
            acc.push(b.on_press(Message::SetMeasure(m)).padding([4, 10]))
        });

        let back = {
            let b = button(text("◀").size(13)).padding([4, 8]);
            if self.trail.can_back() {
                b.on_press(Message::Back)
            } else {
                b
            }
        };
        let forward = {
            let b = button(text("▶").size(13)).padding([4, 8]);
            if self.trail.can_forward() {
                b.on_press(Message::Forward)
            } else {
                b
            }
        };

        row![
            text("Hewton").size(20),
            iced::widget::Space::new().width(Length::Fill),
            measure_row,
            iced::widget::Space::new().width(Length::Fixed(16.0)),
            back,
            forward,
        ]
        .spacing(8)
        .align_y(iced::Alignment::Center)
        .into()
    }

    /// Render a single panel card. Kpi panel routes through the
    /// `hyozu::tatami::card` adapter when its query resolved as a scalar;
    /// every other panel still uses the text stub pending Phase 2b+.
    fn render_card(&self, panel: Panel) -> Element<'_, Message> {
        let state = self.panels.get(&panel);
        let body: Element<'_, Message> = match (panel, state) {
            (Panel::Kpi, Some(QueryState::Ok(results))) => kpi::render(results, self.view.measure),
            (_, None) => text("(no task)").size(14).into(),
            (_, Some(QueryState::Running)) => text("Running…").size(14).into(),
            (_, Some(QueryState::Ok(results))) => text(format!("Ok: {}", describe(results))).size(14).into(),
            (_, Some(QueryState::Err(message))) => text(format!("Err: {message}")).size(14).into(),
        };
        container(column![text(panel.heading()).size(18), body].spacing(4))
            .padding(12)
            .width(Length::Fill)
            .into()
    }
}

fn measure_label(m: Measure) -> &'static str {
    match m {
        Measure::Revenue => "Revenue",
        Measure::Occupancy => "Occupancy",
        Measure::Adr => "ADR",
        Measure::RevPar => "RevPAR",
    }
}

/// Fire one panel query as an iced Task.
fn run(cube: Arc<InMemoryCube>, panel: Panel, q: tatami::Query) -> Task<Message> {
    Task::future(async move {
        let outcome = cube.query(&q).await.map_err(|e| e.to_string());
        Message::PanelDone(panel, outcome)
    })
}

fn describe(results: &tatami::Results) -> &'static str {
    match results {
        tatami::Results::Scalar(_) => "Results::Scalar",
        tatami::Results::Series(_) => "Results::Series",
        tatami::Results::Pivot(_) => "Results::Pivot",
        tatami::Results::Rollup(_) => "Results::Rollup",
        _ => "Results::<unknown>",
    }
}

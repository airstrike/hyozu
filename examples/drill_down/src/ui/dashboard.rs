//! Dashboard state + the per-panel async dispatch plumbing.
//!
//! Phase 1 wires the navigation + per-panel async-result flow end-to-end
//! against a constructed `InMemoryCube`. Rendering is a simple column of
//! per-panel text stubs; real marks land in later phases.

use std::collections::HashMap;
use std::sync::Arc;

use iced::widget::{Column, column, container, text};
use iced::{Element, Length, Task};

use tatami::Cube;
use tatami_inmem::InMemoryCube;

use crate::ui::panel::Panel;
use crate::ui::query_state::QueryState;
use crate::ui::trail::Trail;
use crate::ui::view::View;

/// Messages consumed by the Dashboard.
///
/// The Dashboard itself only emits and handles `PanelDone`; the outer
/// `App` layer maps `SetMeasure`, `SetPeriod`, `Back`, and `Forward`
/// onto the `Dashboard`'s methods.
#[derive(Debug)]
#[non_exhaustive]
pub enum Message {
    /// A panel's query finished.
    PanelDone(Panel, Result<tatami::Results, String>),
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

    /// Handle a `PanelDone` message — update the panel's state. Returns
    /// `Task::none` for now; later phases may chain follow-up dispatches.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PanelDone(panel, Ok(results)) => {
                self.panels.insert(panel, QueryState::Ok(results));
            }
            Message::PanelDone(panel, Err(error)) => {
                self.panels.insert(panel, QueryState::Err(error));
            }
        }
        Task::none()
    }

    /// Render the Phase 1 text-stub view — a column of one card per panel.
    pub fn view(&self) -> Element<'_, Message> {
        let cards = Panel::ALL
            .iter()
            .map(|&panel| render_card(panel, self.panels.get(&panel)));
        container(Column::with_children(cards).spacing(12).padding(16).width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

/// Fire one panel query as an iced Task.
fn run(cube: Arc<InMemoryCube>, panel: Panel, q: tatami::Query) -> Task<Message> {
    Task::future(async move {
        let outcome = cube.query(&q).await.map_err(|e| e.to_string());
        Message::PanelDone(panel, outcome)
    })
}

/// Phase 1 per-panel text stub — heading + state description.
fn render_card<'a>(panel: Panel, state: Option<&'a QueryState>) -> Element<'a, Message> {
    let body = match state {
        None => "(no task)".to_string(),
        Some(QueryState::Running) => "Running…".to_string(),
        Some(QueryState::Ok(results)) => format!("Ok: {}", describe(results)),
        Some(QueryState::Err(message)) => format!("Err: {message}"),
    };
    container(column![text(panel.heading()).size(18), text(body).size(14)].spacing(4))
        .padding(12)
        .width(Length::Fill)
        .into()
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

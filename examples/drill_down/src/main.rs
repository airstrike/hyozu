//! drill_down — Hewton Hotels multi-panel sales dashboard.
//!
//! Phase 1 ships the scaffolding: async asset loading with a 500 ms mock
//! delay, Polars CSV parse on the blocking thread pool, `InMemoryCube`
//! construction from the loaded DataFrame + `hewton_schema`, and the
//! per-panel `Dashboard` dispatch loop. Real rendering lands in later
//! phases.

use std::sync::Arc;
use std::time::Duration;

use iced::widget::{column, container, text};
use iced::{Element, Length, Task};

use polars_core::prelude::DataFrame;
use tatami_inmem::InMemoryCube;

use drill_down::ui::dashboard::{self, Dashboard};
use drill_down::{data, ui};

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Drill Down — Hewton Hotels")
        .window_size((1400.0, 900.0))
        .run()
}

/// Top-level application state.
enum App {
    /// Assets are loading in parallel. Each slot starts `None` and is
    /// populated by its `*Loaded` message.
    Loading {
        facts: Option<DataFrame>,
        /// Phase 1 placeholder — `()` is a stand-in for `hyozu::GeoData`
        /// once the map actually renders in Phase 2.
        geo: Option<()>,
        /// Phase 1 placeholder — `()` is a stand-in for `Centroids` in
        /// Phase 2.
        centroids: Option<()>,
    },
    /// All three assets loaded and the cube was constructed.
    Loaded(Dashboard),
    /// An unrecoverable error occurred during load or cube construction.
    Failed(data::Error),
}

/// Application-level messages.
#[derive(Debug)]
enum Message {
    /// Fact CSV finished loading (success or failure).
    FactsLoaded(Result<DataFrame, data::Error>),
    /// GeoJSON finished loading (placeholder payload in Phase 1).
    GeoLoaded(Result<(), data::Error>),
    /// Centroids finished loading (placeholder payload in Phase 1).
    CentroidsLoaded(Result<(), data::Error>),
    /// A panel's query finished.
    PanelDone(ui::Panel, Result<tatami::Results, String>),
    /// The user clicked a measure toggle.
    #[allow(dead_code)] // wired in Phase 2
    SetMeasure(ui::Measure),
    /// The user clicked a period toggle.
    #[allow(dead_code)] // wired in Phase 3
    SetPeriod(ui::Period),
    /// Back button.
    #[allow(dead_code)] // wired in Phase 2
    Back,
    /// Forward button.
    #[allow(dead_code)] // wired in Phase 2
    Forward,
    /// Retry loading after a failure.
    #[allow(dead_code)] // wired in Phase 6
    Retry,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let load = Task::batch([
            Task::future(load_facts(500)).map(Message::FactsLoaded),
            Task::future(load_geo(500)).map(Message::GeoLoaded),
            Task::future(load_centroids(500)).map(Message::CentroidsLoaded),
        ]);
        (
            App::Loading {
                facts: None,
                geo: None,
                centroids: None,
            },
            load,
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::FactsLoaded(Ok(df)) => {
                if let App::Loading { facts, .. } = self {
                    *facts = Some(df);
                }
                self.try_transition()
            }
            Message::FactsLoaded(Err(e)) => {
                *self = App::Failed(e);
                Task::none()
            }
            Message::GeoLoaded(Ok(())) => {
                if let App::Loading { geo, .. } = self {
                    *geo = Some(());
                }
                self.try_transition()
            }
            Message::GeoLoaded(Err(e)) => {
                *self = App::Failed(e);
                Task::none()
            }
            Message::CentroidsLoaded(Ok(())) => {
                if let App::Loading { centroids, .. } = self {
                    *centroids = Some(());
                }
                self.try_transition()
            }
            Message::CentroidsLoaded(Err(e)) => {
                *self = App::Failed(e);
                Task::none()
            }
            Message::PanelDone(panel, outcome) => {
                if let App::Loaded(dashboard) = self {
                    dashboard
                        .update(dashboard::Message::PanelDone(panel, outcome))
                        .map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::SetMeasure(m) => {
                if let App::Loaded(dashboard) = self {
                    let v = ui::View {
                        measure: m,
                        ..dashboard.view
                    };
                    dashboard.set_view(v).map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::SetPeriod(p) => {
                if let App::Loaded(dashboard) = self {
                    let v = ui::View {
                        period: p,
                        ..dashboard.view
                    };
                    dashboard.set_view(v).map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::Back => {
                if let App::Loaded(dashboard) = self {
                    dashboard.back().map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::Forward => {
                if let App::Loaded(dashboard) = self {
                    dashboard.forward().map(lift_dashboard_message)
                } else {
                    Task::none()
                }
            }
            Message::Retry => {
                let (fresh, task) = App::new();
                *self = fresh;
                task
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match self {
            App::Loading { .. } => container(text("Loading…").size(24))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into(),
            App::Failed(err) => {
                container(column![text("Failed to load").size(24), text(format!("{err}")).size(14),].spacing(8))
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into()
            }
            App::Loaded(dashboard) => dashboard.view().map(lift_dashboard_message),
        }
    }

    /// Check whether all three assets are loaded; if so, build the cube and
    /// transition to `Loaded`. On cube-build failure, transition to
    /// `Failed`.
    ///
    /// Checks presence *before* consuming any slot — a naive destructure via
    /// `facts.take()` / `geo.take()` drains those slots even when the match
    /// fails, which permanently strands the loader if the third asset hasn't
    /// arrived yet.
    fn try_transition(&mut self) -> Task<Message> {
        let App::Loading { facts, geo, centroids } = self else {
            return Task::none();
        };
        if facts.is_none() || geo.is_none() || centroids.is_none() {
            return Task::none();
        }
        let df = facts.take().expect("presence checked");
        let _ = geo.take();
        let _ = centroids.take();

        let schema = match data::hewton_schema() {
            Ok(s) => s,
            Err(e) => {
                *self = App::Failed(e.into());
                return Task::none();
            }
        };
        let cube = match InMemoryCube::new(df, schema) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                *self = App::Failed(e.into());
                return Task::none();
            }
        };

        let initial_query = ui::queries::nation_default();
        let (dashboard, task) = Dashboard::new(cube, initial_query, ui::View::default());
        *self = App::Loaded(dashboard);
        task.map(lift_dashboard_message)
    }
}

/// Lift a `dashboard::Message` into the outer `Message`.
fn lift_dashboard_message(msg: dashboard::Message) -> Message {
    match msg {
        dashboard::Message::PanelDone(p, r) => Message::PanelDone(p, r),
        // `dashboard::Message` is `#[non_exhaustive]` across the crate
        // boundary. Any future variant the dashboard grows needs a
        // matching `Message` variant — this wildcard panics to surface
        // the gap immediately rather than silently dropping it.
        _ => unreachable!("unhandled dashboard::Message variant"),
    }
}

// ── Async load helpers ────────────────────────────────────────────────────

async fn load_bytes(asset: &'static str, delay_ms: u64) -> Result<Vec<u8>, data::Error> {
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    let path = format!("{}/assets/{asset}", env!("CARGO_MANIFEST_DIR"));
    tokio::fs::read(&path)
        .await
        .map_err(|e| data::Error::Io { path, source: e })
}

async fn load_facts(delay_ms: u64) -> Result<DataFrame, data::Error> {
    let bytes = load_bytes("hewton.csv", delay_ms).await?;
    tokio::task::spawn_blocking(move || {
        use polars_io::prelude::{CsvReadOptions, SerReader};
        use std::io::Cursor;
        CsvReadOptions::default()
            .with_has_header(true)
            .into_reader_with_file_handle(Cursor::new(bytes))
            .finish()
            .map_err(|e| data::Error::Parse {
                asset: "hewton.csv",
                detail: e.to_string(),
            })
    })
    .await?
}

/// Phase 1 placeholder — sleeps and returns Ok. Phase 2 replaces this
/// with a real GeoJSON parse into `hyozu::GeoData`.
async fn load_geo(delay_ms: u64) -> Result<(), data::Error> {
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    Ok(())
}

/// Phase 1 placeholder — sleeps and returns Ok. Phase 2 replaces this
/// with a real centroids JSON parse.
async fn load_centroids(delay_ms: u64) -> Result<(), data::Error> {
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
    Ok(())
}

//! Dashboard — owns the cube, trail, per-panel state, and async results.
//!
//! The dashboard is schema-blind: panel bindings are indices into the
//! cube's `Schema` (supplied via [`crate::DashboardSpec`]), not hard-coded
//! names. A trail entry is the slicer; each panel builds its own
//! `tatami::Query` on top of that slicer at dispatch time.
//!
//! Panels live as draggable/resizable tiles in a `sweeten::tile_grid`.
//! Per-panel pickers render in the tile title bar; per-panel bodies render
//! as the tile content.

use std::collections::HashMap;
use std::sync::Arc;

use iced::widget::{button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Fill, Length, Task};

use hyozu::{Action, GeoData};
use sweeten::widget::tile_grid::{self, CellHeight, Configuration, grid_content, title_bar};
use tatami::Cube;
use tatami::query::{Options, Tuple};
use tatami::schema::Schema;
use tatami_inmem::InMemoryCube;

use crate::panel::{Panel, kpi, line, map, rail};
use crate::query_state::QueryState;
use crate::spec::DashboardSpec;
use crate::trail::Trail;
use crate::{axis, data, metric};

/// Grid column count.
pub const COLS: u16 = 12;
/// Fixed cell height in logical pixels.
pub const CELL_H: f32 = 200.0;
/// Spacing between tiles.
pub const GRID_SPACING: f32 = 4.0;

/// A single tile in the dashboard grid.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Tile {
    /// One of the dashboard's panels. Panel state is read from the
    /// [`Dashboard`] — the tile holds only the panel identifier.
    Panel(Panel),
}

/// Messages consumed by the dashboard.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Message {
    /// A panel's query completed.
    PanelDone(Panel, Result<tatami::Results, String>),
    /// User interacted with the KPI panel's chrome.
    Kpi(kpi::Message),
    /// User interacted with the Map panel's chrome.
    Map(map::Message),
    /// User interacted with the Rail panel's chrome.
    Rail(rail::Message),
    /// User interacted with the Line panel's chrome.
    Line(line::Message),
    /// User clicked a chart element — currently used to drill on the map.
    MapClicked(Action),
    /// A tile was dragged, resized, or clicked.
    GridAction(tile_grid::Action),
    /// Back button pressed.
    Back,
    /// Forward button pressed.
    Forward,
}

/// Dashboard state.
#[derive(Debug)]
pub struct Dashboard {
    /// Shared handle to the OLAP cube.
    pub cube: Arc<InMemoryCube>,
    /// Schema snapshot — every picker reads indices against this value.
    pub schema: Schema,
    /// GeoJSON for the map panel.
    pub geo: Arc<GeoData>,
    /// Per-feature centroids derived from `geo`.
    pub centroids: Arc<data::Centroids>,
    /// Window title.
    pub title: String,
    /// Back/forward navigation history of slicer queries.
    pub trail: Trail,
    /// Per-panel async result state.
    pub panels: HashMap<Panel, QueryState>,
    /// Per-panel bindings.
    pub kpi: kpi::State,
    pub map: map::State,
    pub rail: rail::State,
    pub line: line::State,
    /// Cached chart `Data` for the map panel.
    pub map_data: Option<hyozu::Data>,
    /// Cached chart `Data` for the rail panel.
    pub rail_data: Option<hyozu::Data>,
    /// Cached chart `Data` for the line panel.
    pub line_data: Option<hyozu::Data>,
    /// Tile grid layout — one item per panel.
    pub grid: tile_grid::State<Tile>,
    /// Currently focused tile, if any.
    pub focus: Option<tile_grid::ItemId>,
}

impl Dashboard {
    /// Construct a dashboard and kick off the initial per-panel dispatch.
    pub fn new(
        cube: Arc<InMemoryCube>,
        schema: Schema,
        geo: Arc<GeoData>,
        centroids: Arc<data::Centroids>,
        spec: DashboardSpec,
    ) -> (Self, Task<Message>) {
        let DashboardSpec {
            title,
            initial_slicer,
            kpi,
            map,
            rail,
            line,
        } = spec;

        let slicer = Tuple::of(initial_slicer).unwrap_or_else(|_| Tuple::empty());
        let initial = tatami::Query {
            axes: tatami::Axes::Scalar,
            slicer,
            metrics: Vec::new(),
            options: Options::default(),
        };

        let mut panels = HashMap::with_capacity(Panel::ALL.len());
        for panel in Panel::ALL {
            panels.insert(panel, QueryState::Running);
        }

        let config: Configuration<Tile> = Configuration::new(COLS)
            .float(true)
            .with_item(0, 0, 3, 2, Tile::Panel(Panel::Kpi))
            .with_item(3, 0, 6, 2, Tile::Panel(Panel::Map))
            .with_item(9, 0, 3, 2, Tile::Panel(Panel::Rail))
            .with_item(0, 2, 12, 1, Tile::Panel(Panel::Line));
        let grid = tile_grid::State::with_configuration(config);

        let mut dashboard = Self {
            cube,
            schema,
            geo,
            centroids,
            title,
            trail: Trail::new(initial),
            panels,
            kpi,
            map,
            rail,
            line,
            map_data: None,
            rail_data: None,
            line_data: None,
            grid,
            focus: None,
        };
        let task = dashboard.dispatch_all();
        (dashboard, task)
    }

    /// Fire every panel's query in parallel against the current trail
    /// entry's slicer.
    pub fn dispatch_all(&mut self) -> Task<Message> {
        self.map_data = None;
        self.rail_data = None;
        self.line_data = None;
        let tasks: Vec<Task<Message>> = Panel::ALL.iter().filter_map(|&panel| self.dispatch(panel)).collect();
        Task::batch(tasks)
    }

    /// Dispatch a single panel. Leaves the panel in [`QueryState::Running`]
    /// or records an idle placeholder; returns `None` when the panel's
    /// bindings don't produce a runnable query.
    pub fn dispatch(&mut self, panel: Panel) -> Option<Task<Message>> {
        let slicer = self.trail.current().slicer.clone();
        let query = match panel {
            Panel::Kpi => self.kpi.query(&self.schema, slicer),
            Panel::Map => self.map.query(&self.schema, slicer),
            Panel::Rail => self.rail.query(&self.schema, slicer),
            Panel::Line => self.line.query(&self.schema, slicer),
        };
        match query {
            Some(q) => {
                self.panels.insert(panel, QueryState::Running);
                let cube = self.cube.clone();
                Some(Task::future(async move {
                    let outcome = cube.query(&q).await.map_err(|e| e.to_string());
                    Message::PanelDone(panel, outcome)
                }))
            }
            None => {
                self.panels.insert(panel, QueryState::Err("(pick bindings)".into()));
                self.clear_cache(panel);
                None
            }
        }
    }

    fn clear_cache(&mut self, panel: Panel) {
        match panel {
            Panel::Map => self.map_data = None,
            Panel::Rail => self.rail_data = None,
            Panel::Line => self.line_data = None,
            Panel::Kpi => {}
        }
    }

    /// Push a new slicer query onto the trail and re-dispatch every
    /// panel. Adjacent-duplicate pushes are a no-op.
    pub fn navigate(&mut self, q: tatami::Query) -> Task<Message> {
        self.trail.push(q);
        self.dispatch_all()
    }

    /// Move back; re-dispatch when the cursor moves.
    pub fn back(&mut self) -> Task<Message> {
        if self.trail.back() {
            self.dispatch_all()
        } else {
            Task::none()
        }
    }

    /// Move forward; re-dispatch when the cursor moves.
    pub fn forward(&mut self) -> Task<Message> {
        if self.trail.forward() {
            self.dispatch_all()
        } else {
            Task::none()
        }
    }

    /// Handle a dashboard message.
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PanelDone(panel, Ok(results)) => {
                match panel {
                    Panel::Map => {
                        self.map_data = map::build_data(&results, &self.geo, &self.centroids, self.map.scope);
                    }
                    Panel::Rail => self.rail_data = rail::build_data(&results),
                    Panel::Line => self.line_data = line::build_data(&results),
                    Panel::Kpi => {}
                }
                self.panels.insert(panel, QueryState::Ok(results));
                Task::none()
            }
            Message::PanelDone(panel, Err(error)) => {
                eprintln!("panel {panel:?} query failed: {error}");
                self.clear_cache(panel);
                self.panels.insert(panel, QueryState::Err(error));
                Task::none()
            }
            Message::Kpi(msg) => {
                if kpi::apply(&mut self.kpi, msg) {
                    self.dispatch(Panel::Kpi).unwrap_or_else(Task::none)
                } else {
                    Task::none()
                }
            }
            Message::Map(msg) => {
                if map::apply(&mut self.map, &self.schema, msg) {
                    self.dispatch(Panel::Map).unwrap_or_else(Task::none)
                } else {
                    Task::none()
                }
            }
            Message::Rail(msg) => {
                if rail::apply(&mut self.rail, &self.schema, msg) {
                    self.dispatch(Panel::Rail).unwrap_or_else(Task::none)
                } else {
                    Task::none()
                }
            }
            Message::Line(msg) => {
                if line::apply(&mut self.line, &self.schema, msg) {
                    self.dispatch(Panel::Line).unwrap_or_else(Task::none)
                } else {
                    Task::none()
                }
            }
            Message::MapClicked(Action::Clicked(hyozu::Target::Feature { id, .. })) => {
                let feature_id = id.as_str().to_owned();
                let series = match self.panels.get(&Panel::Map) {
                    Some(QueryState::Ok(tatami::Results::Series(s))) => s,
                    _ => {
                        eprintln!("map drill: no series result cached for feature id {feature_id:?}");
                        return Task::none();
                    }
                };
                let Some(member) = self.map.drill_member(series, &feature_id) else {
                    eprintln!("map drill: no member resolved for feature id {feature_id:?}");
                    return Task::none();
                };
                let mut next = self.trail.current().clone();
                let mut members: Vec<_> = next
                    .slicer
                    .members()
                    .iter()
                    .filter(|m| m.dim != member.dim)
                    .cloned()
                    .collect();
                members.push(member);
                let Ok(slicer) = Tuple::of(members) else {
                    eprintln!("map drill: slicer construction failed for feature id {feature_id:?}");
                    return Task::none();
                };
                next.slicer = slicer;
                self.navigate(next)
            }
            Message::MapClicked(_) => Task::none(),
            Message::GridAction(action) => {
                if action.is_click() {
                    self.focus = Some(action.id());
                }
                self.grid.perform(action, |_, _| false);
                Task::none()
            }
            Message::Back => self.back(),
            Message::Forward => self.forward(),
        }
    }

    /// Render the dashboard.
    pub fn view(&self) -> Element<'_, Message> {
        let header = self.render_header();
        let dim_options = axis::dim_choices(&self.schema);
        let metric_options = metric::choices(&self.schema);

        let grid = sweeten::tile_grid(&self.grid, |id, tile| {
            let Tile::Panel(panel) = *tile;
            let is_focused = self.focus == Some(id);

            let body = self.panel_body(panel);
            let controls = self.panel_controls(panel, dim_options.clone(), metric_options.clone());
            let heading = row![text(panel.heading()).size(13)]
                .spacing(6)
                .align_y(Alignment::Center);

            grid_content(body)
                .title_bar(title_bar(heading).controls(controls).padding(10).always_show_controls())
                .style(if is_focused { style::panel_focused } else { style::panel })
        })
        .width(Fill)
        .spacing(GRID_SPACING)
        .cell_height(CellHeight::Fixed(CELL_H))
        .on_action(Message::GridAction);

        container(
            column![header, scrollable(grid).width(Fill).spacing(5)]
                .spacing(8)
                .padding(12)
                .width(Fill),
        )
        .width(Fill)
        .height(Fill)
        .style(style::page)
        .into()
    }

    fn render_header(&self) -> Element<'_, Message> {
        let back = {
            let b = button(text("\u{25c0}").size(13)).padding([4, 8]);
            if self.trail.can_back() {
                b.on_press(Message::Back)
            } else {
                b
            }
        };
        let forward = {
            let b = button(text("\u{25b6}").size(13)).padding([4, 8]);
            if self.trail.can_forward() {
                b.on_press(Message::Forward)
            } else {
                b
            }
        };

        row![
            text(self.title.as_str()).size(20),
            iced::widget::Space::new().width(Length::Fill),
            back,
            forward,
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    /// Render the body of a single panel based on its query state and any
    /// cached chart `Data`.
    fn panel_body(&self, panel: Panel) -> Element<'_, Message> {
        let state = self.panels.get(&panel);
        match (panel, state) {
            (Panel::Kpi, Some(QueryState::Ok(results))) => kpi::render(results, &self.schema, &self.kpi),
            (Panel::Map, Some(QueryState::Ok(_))) => match self.map_data.as_ref() {
                Some(data) => hyozu::chart(data)
                    .height(Length::Fill)
                    .on_action(Message::MapClicked)
                    .into(),
                None => text("(map data unavailable)").size(14).into(),
            },
            (Panel::Rail, Some(QueryState::Ok(results))) => match self.rail_data.as_ref() {
                Some(data) => rail::render(data),
                None => rail::fallback(results),
            },
            (Panel::Line, Some(QueryState::Ok(results))) => match self.line_data.as_ref() {
                Some(data) => line::render(data),
                None => line::fallback(results),
            },
            (_, None) => text("(no task)").size(14).into(),
            (_, Some(QueryState::Running)) => text("Running\u{2026}").size(14).into(),
            (_, Some(QueryState::Err(message))) => render_error(message),
        }
    }

    /// Render the picker controls for a panel. These appear in the tile
    /// title bar.
    fn panel_controls(
        &self,
        panel: Panel,
        dim_options: Vec<axis::DimChoice>,
        metric_options: Vec<metric::Choice>,
    ) -> Element<'_, Message> {
        match panel {
            Panel::Kpi => kpi::chrome(&self.kpi, metric_options),
            Panel::Map => map::chrome(&self.schema, &self.map, dim_options, metric_options),
            Panel::Rail => rail::chrome(&self.schema, &self.rail, dim_options, metric_options),
            Panel::Line => line::chrome(&self.schema, &self.line, dim_options, metric_options),
        }
    }
}

/// Render a panel-level error message. An empty-result error from the
/// resolver (no member at the sliced coordinate) is presented as a neutral
/// "no data" note; other classes pass through as raw text so the shape of
/// the underlying failure stays visible.
fn render_error(message: &str) -> Element<'_, Message> {
    let is_no_data = message.starts_with("resolve: dimension ") && message.contains("unknown member at path ");
    if is_no_data {
        text("No data available for this selection").size(14).into()
    } else {
        text(message).size(14).into()
    }
}

mod style {
    use iced::Theme;
    use iced::widget::container;

    pub fn panel(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.base.color.into()),
            border: iced::Border {
                width: 1.0,
                color: palette.background.strong.color,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }

    pub fn panel_focused(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.base.color.into()),
            border: iced::Border {
                width: 2.0,
                color: palette.primary.base.color,
                radius: 6.0.into(),
            },
            ..Default::default()
        }
    }

    pub fn page(theme: &Theme) -> container::Style {
        let palette = theme.palette();
        container::Style {
            background: Some(palette.background.weak.color.into()),
            ..Default::default()
        }
    }
}

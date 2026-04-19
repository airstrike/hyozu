use iced::Alignment::End;
use iced::widget::{button, column, container, pick_list, progress_bar, row, rule, scrollable, space, table, text};
use iced::{Center, Color, Element, Fill, Font, Theme, color, font};

use sweeten::widget::tile_grid::{self, CellHeight, grid_content, title_bar};

use hyozu::mark::bar::Label;
use hyozu::mark::bar::label::Position as LabelPosition;
use hyozu::mark::treemap::item;
use hyozu::{LegendConfig, Target, bar, bars, chart, data, treemap};

// ── Fonts ────────────────────────────────────────────────────────────

const BOLD: Font = Font {
    weight: font::Weight::Bold,
    ..Font::DEFAULT
};

// ── Layout ───────────────────────────────────────────────────────────

const COLS: u16 = 12;
const CELL_H: f32 = 75.0;
const GRID_SPACING: f32 = 3.0;

// ── Colors — Anaplan light theme ─────────────────────────────────────

const PAGE_BG: Color = color!(0xedf0f5);
const CARD_BG: Color = Color::WHITE;
const HEADER_BG: Color = color!(0x1b365d);
const TITLE_COLOR: Color = color!(0x003b5c);
const TEXT_COLOR: Color = color!(0x333333);
const MUTED_COLOR: Color = color!(0x888888);
const DIVIDER: Color = color!(0xdde1e6);
const ACCENT: Color = color!(0x0078d4);

const STRATEGIC: Color = color!(0x17a589);
const BIG_ACCT: Color = color!(0x2e86de);
const MEDIUM_ACCT: Color = color!(0x8e44ad);
const SMALL_ACCT: Color = color!(0xbb8fce);

// ── Model ────────────────────────────────────────────────────────────

mod model {
    pub struct Dashboard {
        pub total_accounts: u32,
        pub previous_accounts: u32,
        pub filters: Vec<Filter>,
        pub turnover_bands: Vec<Band>,
        pub segment_counts: Vec<SegmentCount>,
        pub top_accounts: Vec<Account>,
        pub owners: Vec<Owner>,
        pub industries: Vec<Industry>,
    }

    pub struct Filter {
        pub name: &'static str,
        pub selected: u32,
        pub total: u32,
    }

    pub struct Band {
        pub label: &'static str,
        pub count: u32,
    }

    pub struct SegmentCount {
        pub label: &'static str,
        pub count: u32,
    }

    #[derive(Clone)]
    pub struct Account {
        pub name: &'static str,
        pub ranking: u32,
        pub revenue: &'static str,
        pub segment: &'static str,
        pub country: &'static str,
        pub industry: &'static str,
    }

    pub struct Owner {
        pub name: &'static str,
        pub strategic: u32,
        pub big: u32,
        pub medium: u32,
        pub small: u32,
    }

    pub struct Industry {
        pub name: &'static str,
        pub avg_revenue: f32,
    }

    impl Dashboard {
        pub fn variance(&self) -> i32 {
            self.total_accounts as i32 - self.previous_accounts as i32
        }

        /// Mock data snapshot — simulates loaded backend state.
        pub fn sample() -> Self {
            Self {
                total_accounts: 616,
                previous_accounts: 514,
                filters: vec![
                    Filter {
                        name: "Segment",
                        selected: 4,
                        total: 4,
                    },
                    Filter {
                        name: "Industry",
                        selected: 45,
                        total: 45,
                    },
                    Filter {
                        name: "SM Country",
                        selected: 16,
                        total: 17,
                    },
                    Filter {
                        name: "Legal Entity",
                        selected: 3,
                        total: 5,
                    },
                ],
                turnover_bands: vec![
                    Band {
                        label: "<500 M$",
                        count: 1,
                    },
                    Band {
                        label: "500 M$ - 1 B$",
                        count: 148,
                    },
                    Band {
                        label: "1 B$ - 10 B$",
                        count: 129,
                    },
                    Band {
                        label: "10 B$ - 50 B$",
                        count: 99,
                    },
                    Band {
                        label: "50 B$ - 100 B$",
                        count: 56,
                    },
                    Band {
                        label: ">100 B$",
                        count: 11,
                    },
                ],
                segment_counts: vec![
                    SegmentCount {
                        label: "Small Account",
                        count: 63,
                    },
                    SegmentCount {
                        label: "Medium Account",
                        count: 284,
                    },
                    SegmentCount {
                        label: "Big Account",
                        count: 223,
                    },
                    SegmentCount {
                        label: "Strategic Account",
                        count: 46,
                    },
                ],
                top_accounts: vec![
                    Account {
                        name: "Account 917",
                        ranking: 1,
                        revenue: "542,292 M$",
                        segment: "Strategic Account",
                        country: "USA",
                        industry: "Chemicals",
                    },
                    Account {
                        name: "Account 1640",
                        ranking: 2,
                        revenue: "481,739 M$",
                        segment: "Medium Account",
                        country: "France",
                        industry: "Entertainment",
                    },
                    Account {
                        name: "Account 210",
                        ranking: 3,
                        revenue: "437,666 M$",
                        segment: "Big Account",
                        country: "Benelux",
                        industry: "Healthcare",
                    },
                    Account {
                        name: "Account 1855",
                        ranking: 4,
                        revenue: "347,380 M$",
                        segment: "Strategic Account",
                        country: "USA",
                        industry: "Manufacturing",
                    },
                    Account {
                        name: "Account 979",
                        ranking: 5,
                        revenue: "321,465 M$",
                        segment: "Strategic Account",
                        country: "UK",
                        industry: "Distribution & Wholesale",
                    },
                    Account {
                        name: "Account 2571",
                        ranking: 6,
                        revenue: "234,687 M$",
                        segment: "Strategic Account",
                        country: "Spain",
                        industry: "Tourism & Hospitality",
                    },
                    Account {
                        name: "Account 2680",
                        ranking: 7,
                        revenue: "218,403 M$",
                        segment: "Big Account",
                        country: "USA",
                        industry: "Education",
                    },
                    Account {
                        name: "Account 1737",
                        ranking: 8,
                        revenue: "217,306 M$",
                        segment: "Strategic Account",
                        country: "Argentina",
                        industry: "Education",
                    },
                    Account {
                        name: "Account 2369",
                        ranking: 9,
                        revenue: "134,712 M$",
                        segment: "Strategic Account",
                        country: "Italy",
                        industry: "Automotive",
                    },
                    Account {
                        name: "Account 2077",
                        ranking: 10,
                        revenue: "127,644 M$",
                        segment: "Strategic Account",
                        country: "Italy",
                        industry: "Professional Services",
                    },
                    Account {
                        name: "Account 960",
                        ranking: 11,
                        revenue: "117,939 M$",
                        segment: "Strategic Account",
                        country: "USA",
                        industry: "Personal Care",
                    },
                    Account {
                        name: "Account 1899",
                        ranking: 12,
                        revenue: "98,859 M$",
                        segment: "Medium Account",
                        country: "USA",
                        industry: "Construction",
                    },
                    Account {
                        name: "Account 304",
                        ranking: 13,
                        revenue: "98,731 M$",
                        segment: "Big Account",
                        country: "UK",
                        industry: "Healthcare",
                    },
                    Account {
                        name: "Account 81",
                        ranking: 14,
                        revenue: "98,204 M$",
                        segment: "Small Account",
                        country: "Benelux",
                        industry: "Education",
                    },
                    Account {
                        name: "Account 2536",
                        ranking: 15,
                        revenue: "96,921 M$",
                        segment: "Strategic Account",
                        country: "Spain",
                        industry: "Entertainment",
                    },
                    Account {
                        name: "Account 436",
                        ranking: 16,
                        revenue: "95,845 M$",
                        segment: "Strategic Account",
                        country: "Portugal",
                        industry: "E-Commerce Retail",
                    },
                ],
                owners: vec![
                    Owner {
                        name: "Kayleigh Patel",
                        strategic: 4,
                        big: 4,
                        medium: 3,
                        small: 4,
                    },
                    Owner {
                        name: "Miah West",
                        strategic: 4,
                        big: 4,
                        medium: 4,
                        small: 4,
                    },
                    Owner {
                        name: "Lane Sanford",
                        strategic: 4,
                        big: 5,
                        medium: 4,
                        small: 4,
                    },
                    Owner {
                        name: "Summer Richmond",
                        strategic: 4,
                        big: 5,
                        medium: 4,
                        small: 5,
                    },
                    Owner {
                        name: "Camila Lynn",
                        strategic: 5,
                        big: 5,
                        medium: 4,
                        small: 5,
                    },
                    Owner {
                        name: "Brenda Krueger",
                        strategic: 5,
                        big: 6,
                        medium: 5,
                        small: 5,
                    },
                    Owner {
                        name: "Memphis Mcgee",
                        strategic: 6,
                        big: 6,
                        medium: 5,
                        small: 5,
                    },
                    Owner {
                        name: "Cash McIntyre",
                        strategic: 6,
                        big: 7,
                        medium: 5,
                        small: 6,
                    },
                    Owner {
                        name: "Cali Shaffer",
                        strategic: 7,
                        big: 7,
                        medium: 6,
                        small: 6,
                    },
                    Owner {
                        name: "Mercedes Mcgrath",
                        strategic: 8,
                        big: 8,
                        medium: 7,
                        small: 7,
                    },
                ],
                industries: vec![
                    Industry {
                        name: "Entertainment",
                        avg_revenue: 320.0,
                    },
                    Industry {
                        name: "Personal Care",
                        avg_revenue: 260.0,
                    },
                    Industry {
                        name: "Tourism & Hospitality",
                        avg_revenue: 210.0,
                    },
                    Industry {
                        name: "Metals",
                        avg_revenue: 175.0,
                    },
                    Industry {
                        name: "Chemicals",
                        avg_revenue: 145.0,
                    },
                    Industry {
                        name: "Utilities",
                        avg_revenue: 135.0,
                    },
                    Industry {
                        name: "Transportation",
                        avg_revenue: 125.0,
                    },
                    Industry {
                        name: "Electrical Goods",
                        avg_revenue: 110.0,
                    },
                    Industry {
                        name: "Holding",
                        avg_revenue: 105.0,
                    },
                    Industry {
                        name: "Education",
                        avg_revenue: 95.0,
                    },
                    Industry {
                        name: "Banking & Finance",
                        avg_revenue: 92.0,
                    },
                    Industry {
                        name: "Telecommu & Media",
                        avg_revenue: 88.0,
                    },
                    Industry {
                        name: "Publishing",
                        avg_revenue: 82.0,
                    },
                    Industry {
                        name: "Insurance",
                        avg_revenue: 76.0,
                    },
                    Industry {
                        name: "Automotive",
                        avg_revenue: 72.0,
                    },
                    Industry {
                        name: "Research",
                        avg_revenue: 62.0,
                    },
                    Industry {
                        name: "Manufacturing",
                        avg_revenue: 58.0,
                    },
                    Industry {
                        name: "Restaurant Chains",
                        avg_revenue: 55.0,
                    },
                    Industry {
                        name: "Housing & Real Estate",
                        avg_revenue: 52.0,
                    },
                    Industry {
                        name: "Pharmaceuticals",
                        avg_revenue: 48.0,
                    },
                    Industry {
                        name: "Healthcare",
                        avg_revenue: 46.0,
                    },
                    Industry {
                        name: "E-Commerce Retail",
                        avg_revenue: 42.0,
                    },
                    Industry {
                        name: "Machinery",
                        avg_revenue: 38.0,
                    },
                    Industry {
                        name: "Distribution & Wholesale",
                        avg_revenue: 36.0,
                    },
                    Industry {
                        name: "Retail",
                        avg_revenue: 35.0,
                    },
                    Industry {
                        name: "Construction",
                        avg_revenue: 34.0,
                    },
                    Industry {
                        name: "Professional Services",
                        avg_revenue: 30.0,
                    },
                    Industry {
                        name: "Oil & Gas",
                        avg_revenue: 28.0,
                    },
                    Industry {
                        name: "Mining",
                        avg_revenue: 26.0,
                    },
                    Industry {
                        name: "Logistics",
                        avg_revenue: 24.0,
                    },
                    Industry {
                        name: "No Profit",
                        avg_revenue: 22.0,
                    },
                    Industry {
                        name: "Instruments & Supplies",
                        avg_revenue: 20.0,
                    },
                    Industry {
                        name: "Government & Public",
                        avg_revenue: 19.0,
                    },
                    Industry {
                        name: "Other Services",
                        avg_revenue: 18.0,
                    },
                    Industry {
                        name: "Food & Beverage",
                        avg_revenue: 16.0,
                    },
                    Industry {
                        name: "Textile",
                        avg_revenue: 13.0,
                    },
                    Industry {
                        name: "Agriculture",
                        avg_revenue: 11.0,
                    },
                ],
            }
        }
    }
}

// ── Tile types ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WidgetKind {
    Overview,
    TurnoverBand,
    TurnoverSegment,
    OwnerSegment,
    Details,
    Revenues,
}

impl WidgetKind {
    const ALL: &[Self] = &[
        Self::Overview,
        Self::TurnoverBand,
        Self::TurnoverSegment,
        Self::OwnerSegment,
        Self::Details,
        Self::Revenues,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Overview => "Account Overview",
            Self::TurnoverBand => "Account By Turnover Band",
            Self::TurnoverSegment => "Account By Turnover Segment",
            Self::OwnerSegment => "Account By Owner & Segment",
            Self::Details => "Account Details",
            Self::Revenues => "Average Revenues",
        }
    }
}

impl std::fmt::Display for WidgetKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Clone)]
struct TileDetail {
    title: String,
    accounts: Vec<model::Account>,
}

#[derive(Clone)]
struct Tile {
    kind: Option<WidgetKind>,
    detail: Option<TileDetail>,
}

// ── Screen ───────────────────────────────────────────────────────────

struct Screen {
    model: model::Dashboard,
    grid: tile_grid::State<Tile>,
    focus: Option<tile_grid::ItemId>,
    turnover_band: hyozu::Data,
    turnover_segment: hyozu::Data,
    owner_segment: hyozu::Data,
    treemap_chart: hyozu::Data,
}

#[derive(Debug, Clone)]
enum Message {
    GridAction(tile_grid::Action),
    AddTile,
    CloseTile(tile_grid::ItemId),
    SetWidget(tile_grid::ItemId, WidgetKind),
    ChartAction(tile_grid::ItemId, hyozu::Action),
    Back(tile_grid::ItemId),
}

impl Screen {
    fn new() -> Self {
        let model = model::Dashboard::sample();

        let turnover_band = Self::build_turnover_band(&model);
        let turnover_segment = Self::build_turnover_segment(&model);
        let owner_segment = Self::build_owner_segment(&model);
        let treemap_chart = Self::build_treemap(&model);

        let tile = |kind| Tile {
            kind: Some(kind),
            detail: None,
        };

        let mut grid = tile_grid::State::new(COLS);
        grid.add(0, 0, 3, 2, tile(WidgetKind::Overview));
        grid.add(0, 2, 3, 4, tile(WidgetKind::TurnoverBand));
        grid.add(0, 6, 3, 3, tile(WidgetKind::TurnoverSegment));
        grid.add(3, 0, 5, 4, tile(WidgetKind::OwnerSegment));
        grid.add(3, 4, 5, 5, tile(WidgetKind::Details));
        grid.add(8, 0, 4, 9, tile(WidgetKind::Revenues));

        Self {
            model,
            grid,
            focus: None,
            turnover_band,
            turnover_segment,
            owner_segment,
            treemap_chart,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::GridAction(action) => {
                if action.is_click() {
                    self.focus = Some(action.id());
                }
                self.grid.perform(action, |_, _| false);
            }
            Message::AddTile => {
                self.grid.add_auto(3, 3, Tile {
                    kind: None,
                    detail: None,
                });
            }
            Message::CloseTile(id) => {
                self.grid.remove(id);
                if self.focus == Some(id) {
                    self.focus = None;
                }
            }
            Message::SetWidget(id, kind) => {
                if let Some(tile) = self.grid.get_mut(id) {
                    tile.kind = Some(kind);
                    tile.detail = None;
                }
            }
            Message::ChartAction(id, hyozu::Action::Clicked(ref target)) => {
                let kind = self.grid.get(id).and_then(|t| t.kind);
                let detail = self.build_detail(kind, target);
                if let Some(tile) = self.grid.get_mut(id) {
                    tile.detail = detail;
                }
            }
            Message::Back(id) => {
                if let Some(tile) = self.grid.get_mut(id) {
                    tile.detail = None;
                }
            }
            _ => {}
        }
    }

    fn build_detail(&self, kind: Option<WidgetKind>, target: &Target) -> Option<TileDetail> {
        let (Target::Entry { index, .. } | Target::EntryLabel { index, .. }) = target else {
            return None;
        };
        let index = *index;

        match kind {
            Some(WidgetKind::TurnoverBand) => {
                let band = self.model.turnover_bands.get(index)?;
                Some(TileDetail {
                    title: format!("Turnover Band: {}", band.label),
                    accounts: self.model.top_accounts.clone(),
                })
            }
            Some(WidgetKind::TurnoverSegment) => {
                let seg = self.model.segment_counts.get(index)?;
                Some(TileDetail {
                    title: format!("Segment: {}", seg.label),
                    accounts: self
                        .model
                        .top_accounts
                        .iter()
                        .filter(|a| a.segment == seg.label)
                        .cloned()
                        .collect(),
                })
            }
            Some(WidgetKind::OwnerSegment) => {
                let owner = self.model.owners.get(index)?;
                Some(TileDetail {
                    title: format!("Owner: {}", owner.name),
                    accounts: self.model.top_accounts.clone(),
                })
            }
            Some(WidgetKind::Revenues) => {
                let ind = self.model.industries.get(index)?;
                Some(TileDetail {
                    title: format!("Industry: {}", ind.name),
                    accounts: self
                        .model
                        .top_accounts
                        .iter()
                        .filter(|a| a.industry == ind.name)
                        .cloned()
                        .collect(),
                })
            }
            _ => None,
        }
    }

    // ── View ─────────────────────────────────────────────────────────

    fn view(&self) -> Element<'_, Message> {
        column![
            self.view_filter_bar(),
            self.view_toolbar(),
            rule::horizontal(1),
            self.view_grid(),
        ]
        .into()
    }

    fn view_toolbar(&self) -> Element<'_, Message> {
        let add = button(text("+ Add Widget").size(13))
            .on_press(Message::AddTile)
            .style(button::primary)
            .padding([6, 16]);

        let info = text!("{}-column grid  |  {} widgets", self.grid.columns(), self.grid.len())
            .size(12)
            .color(MUTED_COLOR);

        container(row![add, info, space::horizontal()].spacing(16).align_y(Center))
            .padding([8, 14])
            .into()
    }

    fn view_grid(&self) -> Element<'_, Message> {
        let focus = self.focus;

        let grid = sweeten::tile_grid(&self.grid, |id, tile| {
            let is_focused = focus == Some(id);

            // Title text
            let title_text = match (&tile.kind, &tile.detail) {
                (_, Some(detail)) => detail.title.as_str(),
                (Some(kind), _) => kind.label(),
                (None, _) => "New Widget",
            };

            // Controls (back + close)
            let mut controls = row![].spacing(4);
            if tile.detail.is_some() {
                controls = controls.push(
                    button(text("\u{2190}").size(14))
                        .on_press(Message::Back(id))
                        .style(button::text)
                        .padding([2, 6]),
                );
            }
            controls = controls.push(
                button(text("\u{00D7}").size(14))
                    .on_press(Message::CloseTile(id))
                    .style(button::text)
                    .padding([2, 6]),
            );

            // Body
            let body: Element<'_, Message> = match (&tile.kind, &tile.detail) {
                (_, Some(detail)) => self.view_tile_detail(detail),
                (Some(kind), None) => self.view_tile_content(id, *kind),
                (None, _) => self.view_tile_picker(id),
            };

            grid_content(body)
                .title_bar(
                    title_bar(text(title_text.to_string()).size(12).color(TITLE_COLOR).font(BOLD))
                        .controls(controls)
                        .padding([6, 10])
                        .style(if is_focused { title_bar_focused } else { title_bar_style }),
                )
                .style(if is_focused { panel_focused } else { panel_style })
        })
        .width(Fill)
        .height(Fill)
        .spacing(GRID_SPACING)
        .cell_height(CellHeight::Fixed(CELL_H))
        .on_action(Message::GridAction);

        container(grid)
            .padding(8)
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(PAGE_BG.into()),
                ..Default::default()
            })
            .into()
    }

    // ── Tile content views ───────────────────────────────────────────

    fn view_tile_content(&self, id: tile_grid::ItemId, kind: WidgetKind) -> Element<'_, Message> {
        match kind {
            WidgetKind::Overview => self.view_overview(),
            WidgetKind::TurnoverBand => chart(&self.turnover_band)
                .style(hyozu::chart::transparent)
                .width(Fill)
                .height(Fill)
                .on_action(move |a| Message::ChartAction(id, a))
                .into(),
            WidgetKind::TurnoverSegment => chart(&self.turnover_segment)
                .style(hyozu::chart::transparent)
                .width(Fill)
                .height(Fill)
                .on_action(move |a| Message::ChartAction(id, a))
                .into(),
            WidgetKind::OwnerSegment => chart(&self.owner_segment)
                .style(hyozu::chart::transparent)
                .width(Fill)
                .height(Fill)
                .on_action(move |a| Message::ChartAction(id, a))
                .into(),
            WidgetKind::Details => self.view_table(),
            WidgetKind::Revenues => self.view_treemap(id),
        }
    }

    fn view_tile_picker(&self, id: tile_grid::ItemId) -> Element<'_, Message> {
        let picker = pick_list(None::<WidgetKind>, WidgetKind::ALL, WidgetKind::to_string)
            .on_select(move |kind| Message::SetWidget(id, kind))
            .placeholder("Choose a widget...");

        container(picker).center(Fill).padding(20).into()
    }

    fn view_tile_detail<'a>(&'a self, detail: &'a TileDetail) -> Element<'a, Message> {
        let n = detail.accounts.len();

        if detail.accounts.is_empty() {
            container(
                text("No matching accounts in the top 16")
                    .size(13)
                    .color(MUTED_COLOR)
                    .center(),
            )
            .center(Fill)
            .into()
        } else {
            let tbl = table(account_columns(), &detail.accounts)
                .padding_x(8.0)
                .padding_y(5.0)
                .separator_x(0.0)
                .separator_y(1.0);

            column![
                text!("{n} accounts").size(11).color(MUTED_COLOR),
                scrollable(Element::from(tbl)).height(Fill),
            ]
            .spacing(4)
            .padding(8)
            .into()
        }
    }

    // ── Widget-specific views ────────────────────────────────────────

    fn view_filter_bar(&self) -> Element<'_, Message> {
        let filters = self.model.filters.iter().map(|f| {
            let frac = f.selected as f32 / f.total as f32;
            row![
                text(f.name).size(12).color(Color::WHITE),
                text!("{}/{}", f.selected, f.total)
                    .size(12)
                    .color(Color::WHITE)
                    .font(BOLD),
                progress_bar(0.0..=1.0, frac).length(60).girth(6),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        });

        let done = button(text("Done").size(12)).style(button::secondary).padding([4, 16]);

        let content: Vec<Element<'_, Message>> = filters
            .chain(std::iter::once(space::horizontal().into()))
            .chain(std::iter::once(done.into()))
            .collect();

        container(row(content).spacing(24).align_y(Center).padding([0, 16]))
            .center_y(Fill)
            .width(Fill)
            .height(50)
            .style(|_| container::Style {
                background: Some(HEADER_BG.into()),
                ..Default::default()
            })
            .into()
    }

    fn view_overview(&self) -> Element<'_, Message> {
        let total = self.model.total_accounts.to_string();
        let previous = self.model.previous_accounts.to_string();
        let variance = self.model.variance().to_string();
        let trend = if self.model.variance() >= 0 {
            "\u{25B2}"
        } else {
            "\u{25BC}"
        };

        let stat = |label: &str, value: &str| -> Element<'_, Message> {
            column![
                text(label.to_string()).size(9).color(MUTED_COLOR).center().font(BOLD),
                text(value.to_string()).size(14).color(ACCENT).center(),
            ]
            .spacing(2)
            .align_x(Center)
            .width(Fill)
            .into()
        };

        column![
            text(total).size(48).color(ACCENT).center().font(BOLD),
            row![
                stat("PREVIOUS", &previous),
                stat("VARIANCE", &variance),
                stat("TREND", trend),
            ]
            .spacing(8),
        ]
        .spacing(4)
        .width(Fill)
        .padding(8)
        .into()
    }

    fn view_table(&self) -> Element<'_, Message> {
        let tbl = table(account_columns(), &self.model.top_accounts)
            .padding_x(8.0)
            .padding_y(5.0)
            .separator_x(0.0)
            .separator_y(1.0);

        scrollable(Element::from(tbl)).height(Fill).into()
    }

    fn view_treemap(&self, id: tile_grid::ItemId) -> Element<'_, Message> {
        let tabs = row![
            text("BY INDUSTRY").size(10).color(TITLE_COLOR).font(BOLD),
            text("INDUSTRY DETAILS").size(10).color(MUTED_COLOR),
            text("BY COUNTRY").size(10).color(MUTED_COLOR),
            text("COUNTRY DETAILS").size(10).color(MUTED_COLOR),
        ]
        .spacing(12);

        column![
            tabs,
            chart(&self.treemap_chart)
                .style(hyozu::chart::transparent)
                .width(Fill)
                .height(Fill)
                .on_action(move |a| Message::ChartAction(id, a)),
        ]
        .spacing(6)
        .padding(4)
        .into()
    }

    // ── Chart data builders ──────────────────────────────────────────

    fn build_turnover_band(model: &model::Dashboard) -> hyozu::Data {
        let values: Vec<i32> = model.turnover_bands.iter().map(|b| b.count as i32).collect();
        let labels: Vec<&str> = model.turnover_bands.iter().map(|b| b.label).collect();
        data(
            bars([bar(values).with_color(ACCENT)])
                .horizontal()
                .with_size(0.65)
                .data_labels(Label::new().with_position(LabelPosition::End)),
        )
        .y_axis_labels(labels)
    }

    fn build_turnover_segment(model: &model::Dashboard) -> hyozu::Data {
        let values: Vec<i32> = model.segment_counts.iter().map(|s| s.count as i32).collect();
        let labels: Vec<&str> = model.segment_counts.iter().map(|s| s.label).collect();
        data(
            bars([bar(values).with_color(ACCENT)])
                .horizontal()
                .with_size(0.6)
                .data_labels(Label::new().with_position(LabelPosition::End)),
        )
        .y_axis_labels(labels)
    }

    fn build_owner_segment(model: &model::Dashboard) -> hyozu::Data {
        let strategic: Vec<i32> = model.owners.iter().map(|o| o.strategic as i32).collect();
        let big: Vec<i32> = model.owners.iter().map(|o| o.big as i32).collect();
        let medium: Vec<i32> = model.owners.iter().map(|o| o.medium as i32).collect();
        let small: Vec<i32> = model.owners.iter().map(|o| o.small as i32).collect();
        let labels: Vec<&str> = model.owners.iter().map(|o| o.name).collect();

        data(
            bars([
                bar(strategic).with_name("Strategic Account").with_color(STRATEGIC),
                bar(big).with_name("Big Account").with_color(BIG_ACCT),
                bar(medium).with_name("Medium Account").with_color(MEDIUM_ACCT),
                bar(small).with_name("Small Account").with_color(SMALL_ACCT),
            ])
            .stacked()
            .horizontal()
            .with_size(0.7)
            .data_labels(None),
        )
        .y_axis_labels(labels)
        .legend(LegendConfig::right())
    }

    fn build_treemap(model: &model::Dashboard) -> hyozu::Data {
        let items: Vec<_> = model.industries.iter().map(|i| item(i.name, i.avg_revenue)).collect();
        data(treemap(items))
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn tbl_header<'a>(label: &str) -> Element<'a, Message> {
    container(
        text(label.to_string())
            .font(BOLD)
            .size(10)
            .color(MUTED_COLOR)
            .align_x(Center),
    )
    .padding([2, 5])
    .center_y(Fill)
    .into()
}

fn tbl_cell<'a>(content: impl ToString) -> Element<'a, Message> {
    container(text(content.to_string()).size(11).color(TEXT_COLOR))
        .center_y(Fill)
        .padding([2, 5])
        .into()
}

fn account_columns<'a, 'b>() -> [table::Column<'a, 'b, &'a model::Account, Message>; 6] {
    [
        table::column(tbl_header(""), |a: &model::Account| tbl_cell(a.name)),
        table::column(tbl_header("RANKING"), |a: &model::Account| tbl_cell(a.ranking)).align_x(End),
        table::column(tbl_header("REVENUE"), |a: &model::Account| tbl_cell(a.revenue)),
        table::column(tbl_header("SEGMENT"), |a: &model::Account| tbl_cell(a.segment)),
        table::column(tbl_header("COUNTRY"), |a: &model::Account| tbl_cell(a.country)),
        table::column(tbl_header("INDUSTRY"), |a: &model::Account| tbl_cell(a.industry)),
    ]
}

// ── Styles ───────────────────────────────────────────────────────────

fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(CARD_BG.into()),
        border: iced::Border {
            width: 1.0,
            color: DIVIDER,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn panel_focused(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(CARD_BG.into()),
        border: iced::Border {
            width: 2.0,
            color: ACCENT,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn title_bar_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TITLE_COLOR),
        background: Some(CARD_BG.into()),
        ..Default::default()
    }
}

fn title_bar_focused(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(TITLE_COLOR),
        background: Some(CARD_BG.into()),
        ..Default::default()
    }
}

// ── Theme ────────────────────────────────────────────────────────────

fn app_theme() -> Theme {
    use iced::theme::palette::Seed;

    Theme::custom("Anaplan", Seed {
        background: PAGE_BG,
        text: TEXT_COLOR,
        primary: ACCENT,
        success: color!(0x17a589),
        warning: color!(0xf39c12),
        danger: color!(0xe74c3c),
    })
}

// ── Entry point ──────────────────────────────────────────────────────

fn main() -> iced::Result {
    iced::application(Screen::new, Screen::update, Screen::view)
        .theme(app_theme())
        .window_size((1500.0, 850.0))
        .title("Account Dashboard")
        .run()
}

use iced::Alignment::End;
use iced::widget::{button, column, container, progress_bar, row, rule, scrollable, space, table, text};
use iced::{Center, Color, Element, Fill, Font, Theme, color, font};

use sweeten::widget::tile_grid::{self, CellHeight, grid_content};

use hyozu::mark::treemap::item;
use hyozu::{LegendPosition, Target, bar, bars, chart, data, treemap};

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

// ── Screen (UI state) ────────────────────────────────────────────────

#[derive(Clone)]
enum Panel {
    FilterBar,
    Overview,
    TurnoverBand,
    TurnoverSegment,
    OwnerSegment,
    Details,
    Revenues,
}

enum View {
    Index,
    Detail {
        title: String,
        accounts: Vec<model::Account>,
    },
}

struct Screen {
    model: model::Dashboard,
    view: View,
    grid: tile_grid::State<Panel>,
    turnover_band: hyozu::Data,
    turnover_segment: hyozu::Data,
    owner_segment: hyozu::Data,
    treemap_chart: hyozu::Data,
}

#[derive(Debug, Clone)]
enum Message {
    BandClicked(hyozu::Action),
    SegmentClicked(hyozu::Action),
    OwnerClicked(hyozu::Action),
    TreemapClicked(hyozu::Action),
    Back,
}

impl Screen {
    fn new() -> Self {
        let model = model::Dashboard::sample();

        let turnover_band = Self::build_turnover_band(&model);
        let turnover_segment = Self::build_turnover_segment(&model);
        let owner_segment = Self::build_owner_segment(&model);
        let treemap_chart = Self::build_treemap(&model);

        let mut grid = tile_grid::State::new(COLS);
        grid.set_float(true);
        grid.add(0, 0, 12, 1, Panel::FilterBar);
        grid.add(0, 1, 3, 2, Panel::Overview);
        grid.add(0, 3, 3, 4, Panel::TurnoverBand);
        grid.add(0, 7, 3, 3, Panel::TurnoverSegment);
        grid.add(3, 1, 5, 4, Panel::OwnerSegment);
        grid.add(3, 5, 5, 5, Panel::Details);
        grid.add(8, 1, 4, 9, Panel::Revenues);

        Self {
            model,
            view: View::Index,
            grid,
            turnover_band,
            turnover_segment,
            owner_segment,
            treemap_chart,
        }
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::BandClicked(hyozu::Action::Clicked(Target::Entry { index, .. }))
            | Message::BandClicked(hyozu::Action::Clicked(Target::EntryLabel { index, .. })) => {
                if let Some(band) = self.model.turnover_bands.get(index) {
                    self.view = View::Detail {
                        title: format!("Turnover Band: {}", band.label),
                        accounts: self.model.top_accounts.clone(),
                    };
                }
            }
            Message::SegmentClicked(hyozu::Action::Clicked(Target::Entry { index, .. }))
            | Message::SegmentClicked(hyozu::Action::Clicked(Target::EntryLabel { index, .. })) => {
                if let Some(seg) = self.model.segment_counts.get(index) {
                    self.view = View::Detail {
                        title: format!("Segment: {}", seg.label),
                        accounts: self
                            .model
                            .top_accounts
                            .iter()
                            .filter(|a| a.segment == seg.label)
                            .cloned()
                            .collect(),
                    };
                }
            }
            Message::OwnerClicked(hyozu::Action::Clicked(Target::Entry { index, .. }))
            | Message::OwnerClicked(hyozu::Action::Clicked(Target::EntryLabel { index, .. })) => {
                if let Some(owner) = self.model.owners.get(index) {
                    self.view = View::Detail {
                        title: format!("Owner: {}", owner.name),
                        accounts: self.model.top_accounts.clone(),
                    };
                }
            }
            Message::TreemapClicked(hyozu::Action::Clicked(Target::Entry { index, .. })) => {
                if let Some(ind) = self.model.industries.get(index) {
                    self.view = View::Detail {
                        title: format!("Industry: {}", ind.name),
                        accounts: self
                            .model
                            .top_accounts
                            .iter()
                            .filter(|a| a.industry == ind.name)
                            .cloned()
                            .collect(),
                    };
                }
            }
            Message::Back => {
                self.view = View::Index;
            }
            _ => {}
        }
    }

    fn view(&self) -> Element<'_, Message> {
        match &self.view {
            View::Index => self.view_index(),
            View::Detail { title, accounts } => self.view_detail(title, accounts),
        }
    }

    fn view_index(&self) -> Element<'_, Message> {
        let grid = sweeten::tile_grid(&self.grid, |_id, panel| {
            let content: Element<'_, Message> = match panel {
                Panel::FilterBar => self.view_filter_bar(),
                Panel::Overview => self.view_overview(),
                Panel::TurnoverBand => {
                    self.view_chart_interactive("Account By Turnover Band", &self.turnover_band, Message::BandClicked)
                }
                Panel::TurnoverSegment => self.view_chart_interactive(
                    "Account By Turnover Segment",
                    &self.turnover_segment,
                    Message::SegmentClicked,
                ),
                Panel::OwnerSegment => self.view_chart_interactive(
                    "Account By Owner & Segment",
                    &self.owner_segment,
                    Message::OwnerClicked,
                ),
                Panel::Details => self.view_details(),
                Panel::Revenues => self.view_revenues(),
            };
            grid_content(content)
                .draggable(false)
                .resizable(false)
                .style(panel_style)
        })
        .width(Fill)
        .height(Fill)
        .spacing(GRID_SPACING)
        .cell_height(CellHeight::Fixed(CELL_H))
        .locked(true);

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

    fn view_detail<'a>(&'a self, title: &str, accounts: &'a [model::Account]) -> Element<'a, Message> {
        let back = button(text("\u{2190} Back").size(13))
            .on_press(Message::Back)
            .style(button::text)
            .padding([6, 12]);

        let header = row![back, text(title.to_string()).size(18).color(TITLE_COLOR).font(BOLD),]
            .spacing(16)
            .align_y(Center);

        let count = text!("{} accounts", accounts.len()).size(13).color(MUTED_COLOR);

        let body: Element<'_, Message> = if accounts.is_empty() {
            container(
                text("No matching accounts in the top 16")
                    .size(14)
                    .color(MUTED_COLOR)
                    .center(),
            )
            .center(Fill)
            .into()
        } else {
            let tbl = table(account_columns(), accounts)
                .padding_x(8.0)
                .padding_y(5.0)
                .separator_x(0.0)
                .separator_y(1.0);

            scrollable(Element::from(tbl)).height(Fill).into()
        };

        container(column![header, count, rule::horizontal(1), body].spacing(8).padding(16))
            .width(Fill)
            .height(Fill)
            .style(|_| container::Style {
                background: Some(CARD_BG.into()),
                border: iced::Border {
                    width: 1.0,
                    color: DIVIDER,
                    radius: 4.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    // ── Chart data builders ──────────────────────────────────────────

    fn build_turnover_band(model: &model::Dashboard) -> hyozu::Data {
        let values: Vec<i32> = model.turnover_bands.iter().map(|b| b.count as i32).collect();
        let labels: Vec<&str> = model.turnover_bands.iter().map(|b| b.label).collect();

        data(bars([bar(values).with_color(ACCENT)]).horizontal().with_size(0.65)).y_axis_labels(labels)
    }

    fn build_turnover_segment(model: &model::Dashboard) -> hyozu::Data {
        let values: Vec<i32> = model.segment_counts.iter().map(|s| s.count as i32).collect();
        let labels: Vec<&str> = model.segment_counts.iter().map(|s| s.label).collect();

        data(bars([bar(values).with_color(ACCENT)]).horizontal().with_size(0.6)).y_axis_labels(labels)
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
        .legend(LegendPosition::Right)
    }

    fn build_treemap(model: &model::Dashboard) -> hyozu::Data {
        let items: Vec<_> = model.industries.iter().map(|i| item(i.name, i.avg_revenue)).collect();

        data(treemap(items))
    }

    // ── Panel views ──────────────────────────────────────────────────

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
            .height(Fill)
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

        section(
            "Account Overview",
            column![
                text(total).size(52).color(ACCENT).center().font(BOLD),
                row![
                    stat("PREVIOUS", &previous),
                    stat("VARIANCE", &variance),
                    stat("TREND", trend),
                ]
                .spacing(8),
            ]
            .spacing(4)
            .width(Fill),
        )
    }

    fn view_chart_interactive<'a>(
        &'a self,
        title: &str,
        chart_data: &'a hyozu::Data,
        on_action: fn(hyozu::Action) -> Message,
    ) -> Element<'a, Message> {
        section(
            title,
            chart(chart_data)
                .style(hyozu::chart::transparent)
                .width(Fill)
                .height(Fill)
                .on_action(on_action),
        )
    }

    fn view_details(&self) -> Element<'_, Message> {
        let tbl = table(account_columns(), &self.model.top_accounts)
            .padding_x(8.0)
            .padding_y(5.0)
            .separator_x(0.0)
            .separator_y(1.0);

        section("Account Details", scrollable(Element::from(tbl)).height(Fill))
    }

    fn view_revenues(&self) -> Element<'_, Message> {
        let tabs = row![
            text("BY INDUSTRY").size(10).color(TITLE_COLOR).font(BOLD),
            text("INDUSTRY DETAILS").size(10).color(MUTED_COLOR),
            text("BY COUNTRY").size(10).color(MUTED_COLOR),
            text("COUNTRY DETAILS").size(10).color(MUTED_COLOR),
        ]
        .spacing(12);

        section(
            "Average Revenues",
            column![
                tabs,
                chart(&self.treemap_chart)
                    .style(hyozu::chart::transparent)
                    .width(Fill)
                    .height(Fill)
                    .on_action(Message::TreemapClicked),
            ]
            .spacing(6),
        )
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

fn section<'a>(title: &str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![
        text(title.to_string()).size(13).color(TITLE_COLOR).font(BOLD),
        rule::horizontal(1),
        content.into(),
    ]
    .spacing(6)
    .padding(12)
    .into()
}

fn panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(CARD_BG.into()),
        border: iced::Border {
            width: 1.0,
            color: DIVIDER,
            radius: 4.0.into(),
        },
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

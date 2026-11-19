mod appearance;
mod category;
mod charts;
mod detail;
mod distributions;
mod icon;

use category::Category;
use charts::{ChartId, Variant};
use hyozu::Data;
use iced::widget::{button, column, container, row, scrollable, text, text_input, themer};
use iced::{Alignment, Fill, Font, Subscription, Task, Widget};
use icedcn::Theme;
use icedcn::widget::button as control;
use icedcn::widget::button::{Ghost, Outline};

type Element<'a, Message> = iced::Element<'a, Message, Theme>;

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([1200.0, 820.0])
        .title("hyozu gallery")
        .settings(iced::Settings {
            text_size: 14.into(),
            ..Default::default()
        })
        .fonts([icon::FONT])
        .theme(|state: &App| state.theme.clone())
        .subscription(App::subscription)
        .run()
}

#[derive(Debug, Clone)]
enum Message {
    ToggleDark,
    ToggleBrowse,
    Orbit(f64),
    Resize(f32),
    Filter(Option<Category>),
    Search(String),
    Select(ChartId),
    Back,
    Source(bool),
    Copy,
    Copied(bool),
    Chart(hyozu::Action),
}

struct App {
    theme: Theme,
    dark: bool,
    browse: bool,
    width: f32,
    filter: Option<Category>,
    search: String,
    variants: Vec<Variant>,
    data: Vec<Data>,
    selected: Option<ChartId>,
    camera: hyozu::spatial::Camera,
    source: bool,
    copied: bool,
}

fn heading<'a>(s: impl iced::widget::text::IntoFragment<'a>, size: u16) -> iced::widget::Text<'a, Theme> {
    text(s).size(f32::from(size)).font(Font {
        weight: iced::font::Weight::Semibold,
        ..Font::DEFAULT
    })
}

fn spring<'a, M: 'a>() -> Element<'a, M> {
    iced::widget::space().width(Fill).boxed()
}

fn chart_colors(category: Category) -> hyozu::Palette {
    let colors = match category {
        Category::Bar | Category::Waterfall => [0x367b85, 0x77b3a6, 0xe5a772, 0xb07664, 0x7781ac],
        Category::Statistical | Category::BoxPlot | Category::Violin => {
            [0x7752a5, 0xa98abb, 0x6f8cb2, 0xbb7e76, 0x438e86]
        }
        _ => [0x277b70, 0x6d91c2, 0xd69551, 0xa0697e, 0x78a9a1],
    };
    hyozu::palette::gradient(colors)
}

impl App {
    fn new() -> Self {
        let variants = charts::variants();
        let data = variants
            .iter()
            .map(|v| {
                let chart = v.render(hyozu::spatial::Camera::default());
                if chart.get_palette().is_some() {
                    chart
                } else {
                    chart.palette(chart_colors(v.category))
                }
            })
            .collect();
        Self {
            theme: Theme::light(),
            dark: false,
            browse: false,
            width: 1200.,
            filter: None,
            search: String::new(),
            variants,
            data,
            selected: None,
            camera: hyozu::spatial::Camera::default(),
            source: false,
            copied: false,
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            iced::window::resize_events().map(|(_, s)| Message::Resize(s.width)),
            iced::keyboard::listen().filter_map(|event| match event {
                iced::keyboard::Event::KeyPressed {
                    key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
                    ..
                } => Some(Message::Back),
                _ => None,
            }),
        ])
    }

    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::ToggleDark => {
                self.dark = !self.dark;
                self.theme = if self.dark { Theme::dark() } else { Theme::light() };
            }
            Message::Orbit(delta) => {
                if let Some(id) = self.selected {
                    self.camera.azimuth = (self.camera.azimuth + delta).rem_euclid(360.);
                    let v = &self.variants[id.0];
                    self.data[id.0] = v.render(self.camera).palette(chart_colors(v.category));
                }
            }
            Message::ToggleBrowse => self.browse = !self.browse,
            Message::Resize(width) => self.width = width,
            Message::Filter(cat) => {
                self.filter = cat;
                self.selected = None;
                self.browse = false;
            }
            Message::Search(value) => {
                self.search = value;
                self.selected = None;
            }
            Message::Select(id) => {
                self.selected = Some(id);
                self.source = false;
                self.copied = false;
                self.browse = false;
            }
            Message::Back => {
                self.selected = None;
                self.browse = false;
            }
            Message::Source(value) => self.source = value,
            Message::Copy => {
                if let Some(id) = self.selected {
                    return iced::clipboard::write(self.variants[id.0].source.to_owned())
                        .map(|r| Message::Copied(r.is_ok()));
                }
            }
            Message::Copied(success) => self.copied = success,
            Message::Chart(action) => {
                if let Some(id) = self.selected {
                    self.data[id.0].perform(action);
                }
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let compact = self.width < 760.;
        let mark = container(heading("h", 20).color(iced::Color::WHITE))
            .center_x(33)
            .center_y(33)
            .style(|_| container::Style {
                background: Some(iced::Color::from_rgb8(29, 91, 77).into()),
                border: iced::Border::default().rounded(9),
                ..Default::default()
            });
        let brand = row![mark, heading("Hyozu", 19)].align_y(Alignment::Center).spacing(10);
        let search = text_input("Search charts…", &self.search)
            .on_input(Message::Search)
            .padding([8, 12])
            .width(if compact { 155. } else { 245. });
        let mode = if self.dark { icon::sun() } else { icon::moon() };
        let toggle: Element<'_, Message> = control(mode.size(16).boxed())
            .style(Outline)
            .on_press(Message::ToggleDark)
            .into();
        let mut top = row![brand, spring(), search, toggle]
            .align_y(Alignment::Center)
            .spacing(12);
        if compact {
            top = top.push(control("Browse").style(Ghost).on_press(Message::ToggleBrowse).into());
        }
        let header = container(top)
            .width(Fill)
            .padding(if compact { [10, 12] } else { [13, 24] });
        let content = if compact && self.browse {
            self.sidebar()
        } else {
            let main = if let Some(id) = self.selected {
                self.detail(id)
            } else {
                self.grid()
            };
            if compact {
                main
            } else {
                row![
                    self.sidebar(),
                    Element::<Message>::from(icedcn::separator().orientation(icedcn::Vertical)),
                    main
                ]
                .height(Fill)
                .boxed()
            }
        };
        column![header, Element::<Message>::from(icedcn::separator()), content]
            .height(Fill)
            .width(Fill)
            .boxed()
    }

    fn sidebar(&self) -> Element<'_, Message> {
        let mut nav = column![text("Library").size(12).color(self.theme.muted.foreground)].spacing(3);
        nav = nav.push(self.nav_item(None, "All charts", self.variants.len()));
        nav = nav.push(
            container(text("Chart families").size(12).color(self.theme.muted.foreground))
                .padding([11, 0])
                .boxed(),
        );
        for &cat in Category::ALL {
            let count = self.variants.iter().filter(|v| v.category == cat).count();
            nav = nav.push(self.nav_item(Some(cat), cat.label(), count));
        }
        container(scrollable(nav).height(Fill))
            .width(if self.width < 760. {
                Fill
            } else {
                iced::Length::Fixed(192.)
            })
            .height(Fill)
            .padding([15, 12])
            .boxed()
    }

    fn nav_item<'a>(&self, cat: Option<Category>, label: &'a str, count: usize) -> Element<'a, Message> {
        let active = cat == self.filter;
        button(row![text(label).size(13), spring(), text(count.to_string()).size(11)].align_y(Alignment::Center))
            .on_press(Message::Filter(cat))
            .width(Fill)
            .padding([6, 10])
            .style(move |theme: &Theme, status| {
                let highlighted = active || matches!(status, button::Status::Hovered | button::Status::Pressed);
                button::Style {
                    background: highlighted.then(|| iced::Background::Color(theme.muted.base.into())),
                    text_color: theme.neutral.foreground.into(),
                    border: iced::Border::default().rounded(6),
                    ..Default::default()
                }
            })
            .boxed()
    }

    fn grid(&self) -> Element<'_, Message> {
        let query = self.search.trim().to_lowercase();
        let mut visible: Vec<_> = self
            .variants
            .iter()
            .filter(|v| {
                self.filter.is_none_or(|cat| v.category == cat)
                    && (v.name.to_lowercase().contains(&query)
                        || v.category.label().to_lowercase().contains(&query)
                        || self.data[v.id.0]
                            .get_title()
                            .is_some_and(|s| s.to_lowercase().contains(&query)))
            })
            .collect();
        if self.filter.is_none() && query.is_empty() {
            const FIRST: &[usize] = &[25, 19, 30, 4, 27, 8];
            visible.sort_by_key(|v| FIRST.iter().position(|&i| i == v.id.0).unwrap_or(FIRST.len() + v.id.0));
        }
        let title = self.filter.map_or("Chart gallery", Category::label);
        let count = visible.len();
        let mut grid = column![
            heading(title, 30),
            text(format!("{count} {}", if count == 1 { "example" } else { "examples" }))
                .color(self.theme.muted.foreground)
                .size(14),
            iced::widget::space().height(8)
        ]
        .spacing(6);
        let columns = if self.width >= 1480. {
            3
        } else if self.width >= 980. {
            2
        } else {
            1
        };
        for chunk in visible.chunks(columns) {
            let mut r = iced::widget::Row::new().spacing(16);
            for &v in chunk {
                r = r.push(self.card(v));
            }
            for _ in chunk.len()..columns {
                r = r.push(spring());
            }
            grid = grid.push(r.boxed()).push(iced::widget::space().height(4).boxed());
        }
        if visible.is_empty() {
            grid = grid.push(
                icedcn::card(
                    column![
                        heading("No matching charts", 18),
                        text("Try another name or choose a different family.")
                    ]
                    .spacing(6)
                    .boxed(),
                )
                .padding(20)
                .width(Fill)
                .into(),
            );
        }
        scrollable(grid.padding(if self.width < 760. { 16 } else { 26 }))
            .height(Fill)
            .width(Fill)
            .boxed()
    }

    fn card<'a>(&'a self, v: &'a Variant) -> Element<'a, Message> {
        let h = appearance::chart_height(self.width < 760.);
        let chart = themer(
            Some(self.theme.iced_theme().clone()),
            hyozu::chart::<Message, iced::Theme>(&self.data[v.id.0])
                .style(hyozu::chart::transparent)
                .padding(10)
                .height(h),
        );
        let header = row![
            heading(v.name, 17),
            spring(),
            text("↗").size(17).color(self.theme.muted.foreground)
        ]
        .align_y(Alignment::Center);
        let detail = text(v.category.label()).size(12).color(self.theme.muted.foreground);
        let card: Element<'_, Message> = icedcn::card(column![header, detail, chart].spacing(4).boxed())
            .padding(16)
            .width(Fill)
            .into();
        button(card)
            .width(Fill)
            .padding(0)
            .on_press(Message::Select(v.id))
            .style(|_, _| button::Style {
                text_color: iced::Color::BLACK,
                ..Default::default()
            })
            .boxed()
    }
}

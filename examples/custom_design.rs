//! Example demonstrating custom Theme AND custom Design (Palette) that are
//! completely separate types.
//!
//! This shows that:
//! - The app can use a custom Theme (not iced::Theme) for iced widgets
//! - The chart can use a completely different Design type (Palette) for styling
//! - Both are independent and neither needs to be iced::Theme

use iced::widget::{button, center, column, container, row, space, text};
use iced::{Color, Task};

use hyozu::line::marker::Shape;
use hyozu::{chart, data, line, lines};

use theme::Theme; // custom theme type. see end of file.

pub type Renderer = iced::Renderer;
pub type Element<'a, Message> = iced::Element<'a, Message, Theme, Renderer>;

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("hyozu - custom theme + palette")
        .window_size([800.0, 500.0])
        .centered()
        .run()
}

struct App {
    palette: palette::Palette,
    data: data::Data,
}

#[derive(Debug, Clone)]
enum Message {
    SetPalette(usize),
}

impl App {
    fn new() -> Self {
        // provide x-axis values for each point
        let series1 = [
            (0.0, 20.0),
            (1.0, 45.0),
            (2.0, 35.0),
            (3.0, 60.0),
            (4.0, 50.0),
            (5.0, 75.0),
        ];

        // or imply 0..n
        let series2 = [30.0, 25.0, 55.0, 40.0, 65.0, 55.0];
        let series3 = [10.0, 35.0, 25.0, 50.0, 30.0, 45.0];
        let series4 = [40.0, 30.0, 45.0, 35.0, 55.0, 60.0];

        // Each series gets a different marker shape
        let data = lines![
            line(series1).markers(Shape::Circle),
            line(series2).markers(Shape::Square),
            line(series3).markers(Shape::Diamond),
            line(series4).markers(Shape::Triangle),
        ]
        .into();

        // this would also work
        // let data = data(vec![
        //     line(series1).into(),
        //     line(series2).into(),
        //     line(series3).into(),
        //     line(series4).into(),
        // ]);

        Self {
            palette: palette::Palette::ocean(),
            data,
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SetPalette(i) => {
                self.palette = palette::Palette::all()[i].clone();
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let palettes = palette::Palette::all();

        let palette_buttons = row(palettes.iter().enumerate().map(|(i, p)| {
            let selected = p.name == self.palette.name;
            btn(p.name, selected, Message::SetPalette(i))
        }))
        .spacing(4);

        let swatches = row(self.palette.data.iter().map(swatch)).spacing(8);

        let controls = column![
            row![text("Palette:").size(12).width(60), palette_buttons]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            row![text("Colors:").size(12).width(60), swatches]
                .spacing(8)
                .align_y(iced::Alignment::Center),
        ]
        .spacing(8);

        let chart = chart(&self.data).design(&self.palette).style(hyozu::chart::filled);

        center(column![controls, chart].spacing(20)).padding(20).into()
    }
}

fn btn<'a>(label: &'a str, selected: bool, msg: Message) -> Element<'a, Message> {
    button(text(label).size(12))
        .padding([6, 12])
        .style(move |theme: &theme::Theme, status| {
            if selected {
                theme::button::primary(theme, button::Status::Active)
            } else {
                theme::button::subtle(theme, status)
            }
        })
        .on_press(msg)
        .into()
}

/// Simple color swatch.
fn swatch<'a>(color: &'a Color) -> Element<'a, Message> {
    container(space())
        .width(24)
        .height(24)
        .style(move |_theme| container::Style {
            background: Some((*color).into()),
            border: iced::Border::default().rounded(4),
            ..Default::default()
        })
        .into()
}

// Custom Palette for chart Design. Completely separate from Theme

mod palette {
    use iced::{Color, Font, color};

    /// A color palette for chart styling. This is NOT a Theme - it's purely
    /// for chart design and implements only hyozu::Design.
    #[derive(Debug, Clone)]
    pub struct Palette {
        pub name: &'static str,
        pub background: Color,
        pub text: Color,
        pub divider: Color,
        pub data: Vec<Color>,
    }

    impl Palette {
        pub fn ocean() -> Self {
            Self {
                name: "Ocean",
                background: color!(0xF2FAFF),
                text: color!(0x1A334D),
                divider: color!(0x1A334D, 0.15),
                data: vec![color!(0x0078BF), color!(0x00A6D9), color!(0x33CCCC), color!(0x66E6B3)],
            }
        }

        pub fn sunset() -> Self {
            Self {
                name: "Sunset",
                background: color!(0xFFFAF2),
                text: color!(0x4D261A),
                divider: color!(0x4D261A, 0.15),
                data: vec![color!(0xE64D33), color!(0xF28033), color!(0xF2B34D), color!(0xD96680)],
            }
        }

        pub fn forest() -> Self {
            Self {
                name: "Forest",
                background: color!(0xF5FAF2),
                text: color!(0x264026),
                divider: color!(0x264026, 0.15),
                data: vec![color!(0x33804D), color!(0x66A64D), color!(0x99BF66), color!(0x4D9980)],
            }
        }

        pub fn midnight() -> Self {
            Self {
                name: "Midnight",
                background: color!(0x1F242E),
                text: color!(0xE6E6EB),
                divider: color!(0xE6E6EB, 0.15),
                data: vec![color!(0x6699FF), color!(0xB380FF), color!(0xFF80B3), color!(0x80E6E6)],
            }
        }

        pub fn all() -> Vec<Self> {
            vec![Self::ocean(), Self::sunset(), Self::forest(), Self::midnight()]
        }
    }

    // Palette implements hyozu::Design for chart styling
    impl hyozu::Design for Palette {
        fn background_color(&self) -> Color {
            self.background
        }

        fn text_color(&self) -> hyozu::Color {
            self.text.into()
        }

        fn text_pair(&self) -> hyozu::Pair {
            let inverted = Color::from_rgb(1.0 - self.text.r, 1.0 - self.text.g, 1.0 - self.text.b);
            hyozu::Pair::new(self.text, inverted)
        }

        fn data_colors(&self) -> Vec<hyozu::Color> {
            self.data.iter().map(|c| (*c).into()).collect()
        }

        fn divider_color(&self) -> hyozu::Color {
            self.divider.into()
        }

        fn axis_color(&self) -> hyozu::Color {
            self.text.into()
        }

        fn font(&self) -> Font {
            Font::default()
        }

        fn font_size(&self) -> f32 {
            12.0
        }
    }
}

// Custom Theme for iced app. Completely separate from Palette

mod theme {
    use iced::{Color, color};

    /// A custom theme for iced widgets. This is separate from the chart's
    /// Palette and only implements the iced Catalog traits needed for widgets.
    #[derive(Debug, Clone)]
    pub struct Theme {
        pub background: Color,
        pub text: Color,
        pub accent: Color,
        pub border: Color,
    }

    impl Default for Theme {
        fn default() -> Self {
            // A neutral gray theme. Deliberately different from any palette
            Self {
                background: color!(0xFAFAFA),
                text: color!(0x333333),
                accent: color!(0x555555),
                border: color!(0xDDDDDD),
            }
        }
    }

    // Implement iced::theme::Base for the application
    impl iced::theme::Base for Theme {
        fn default(_preference: iced::theme::Mode) -> Self {
            <Self as Default>::default()
        }

        fn mode(&self) -> iced::theme::Mode {
            iced::theme::Mode::Light
        }

        fn base(&self) -> iced::theme::Style {
            iced::theme::Style {
                background_color: self.background,
                text_color: self.text,
            }
        }

        fn palette(&self) -> Option<iced::theme::Palette> {
            None
        }

        fn name(&self) -> &str {
            "Custom"
        }
    }

    // Theme must implement hyozu::Design since Chart<..., Theme> requires it
    impl hyozu::Design for Theme {
        fn background_color(&self) -> Color {
            self.background
        }

        fn text_color(&self) -> hyozu::Color {
            self.text.into()
        }

        fn text_pair(&self) -> hyozu::Pair {
            hyozu::Pair::new(self.text, self.background)
        }

        fn data_colors(&self) -> Vec<hyozu::Color> {
            // Grayscale fallback - but we override with Palette anyway
            vec![
                color!(0x333333).into(),
                color!(0x666666).into(),
                color!(0x999999).into(),
                color!(0xCCCCCC).into(),
            ]
        }

        fn divider_color(&self) -> hyozu::Color {
            self.border.into()
        }

        fn axis_color(&self) -> hyozu::Color {
            self.text.into()
        }

        fn font(&self) -> iced::Font {
            iced::Font::default()
        }

        fn font_size(&self) -> f32 {
            12.0
        }
    }

    pub mod button {
        use super::Theme;
        use iced::widget::button::{Catalog, Status, Style, StyleFn};
        use iced::{Background, Border};

        impl Catalog for Theme {
            type Class<'a> = StyleFn<'a, Self>;

            fn default<'a>() -> Self::Class<'a> {
                Box::new(primary)
            }

            fn style(&self, class: &Self::Class<'_>, status: Status) -> Style {
                class(self, status)
            }
        }

        pub fn primary(theme: &Theme, status: Status) -> Style {
            let base = Style {
                background: Some(Background::Color(theme.accent)),
                text_color: theme.background,
                border: Border::default().rounded(4),
                ..Style::default()
            };

            match status {
                Status::Active => base,
                Status::Hovered => Style {
                    background: Some(theme.accent.scale_alpha(0.8).into()),
                    ..base
                },
                Status::Pressed => Style {
                    background: Some(theme.accent.scale_alpha(0.6).into()),
                    ..base
                },
                Status::Disabled => Style {
                    background: base.background.map(|bg| bg.scale_alpha(0.5)),
                    text_color: base.text_color.scale_alpha(0.5),
                    ..base
                },
            }
        }

        pub fn subtle(theme: &Theme, status: Status) -> Style {
            let base = Style {
                background: None,
                text_color: theme.text,
                border: Border {
                    color: theme.border,
                    width: 1.0,
                    radius: 4.0.into(),
                },
                ..Style::default()
            };

            match status {
                Status::Active => base,
                Status::Hovered => Style {
                    background: Some(theme.border.scale_alpha(0.6).into()),
                    ..base
                },
                Status::Pressed => Style {
                    background: Some(theme.border.scale_alpha(0.8).into()),
                    ..base
                },
                Status::Disabled => Style {
                    text_color: base.text_color.scale_alpha(0.5),
                    ..base
                },
            }
        }
    }

    pub mod container {
        use super::Theme;
        use iced::widget::container::{Catalog, Style, StyleFn};

        impl Catalog for Theme {
            type Class<'a> = StyleFn<'a, Self>;

            fn default<'a>() -> Self::Class<'a> {
                Box::new(default)
            }

            fn style(&self, class: &Self::Class<'_>) -> Style {
                class(self)
            }
        }

        fn default(_: &Theme) -> Style {
            Style::default()
        }
    }

    pub mod text {
        use super::Theme;
        use iced::widget::text::{Catalog, Style, StyleFn};

        impl Catalog for Theme {
            type Class<'a> = StyleFn<'a, Self>;

            fn default<'a>() -> Self::Class<'a> {
                Box::new(default)
            }

            fn style(&self, class: &Self::Class<'_>) -> Style {
                class(self)
            }
        }

        fn default(_: &Theme) -> Style {
            Style { color: None }
        }
    }
}

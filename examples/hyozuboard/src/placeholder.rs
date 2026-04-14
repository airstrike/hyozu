use iced::widget::{center, text};
use iced::{Element, Font, font};

pub fn view<'a, Message: 'a>() -> Element<'a, Message> {
    center(text("hyozuboard").size(28).font(Font {
        weight: font::Weight::Bold,
        ..Font::DEFAULT
    }))
    .into()
}

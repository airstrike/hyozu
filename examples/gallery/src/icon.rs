// Generated automatically by iced_lucide at build time.
// Do not edit manually.
// eceab15af4a1ba4d3213e0f47fcf77d11a264b37488366a994fc274ac3c802e9
use iced::Font;
use iced::widget::{Text, text};

pub const FONT: &[u8] = include_bytes!("../fonts/lucide.ttf");

/// All icons as `(name, codepoint_str)` pairs.
/// Use this to populate an icon-picker widget.
#[allow(dead_code)]
pub const ALL_ICONS: &[(&str, &str)] = &[("arrow_left", "\u{E048}"), ("moon", "\u{E11E}"), ("sun", "\u{E178}")];

pub fn arrow_left<'a>() -> Text<'a> {
    icon("\u{E048}")
}

pub fn moon<'a>() -> Text<'a> {
    icon("\u{E11E}")
}

pub fn sun<'a>() -> Text<'a> {
    icon("\u{E178}")
}

/// Render any Lucide icon by its codepoint string.
/// Use this together with [`ALL_ICONS`] to display icons dynamically:
/// ```ignore
/// for (name, cp) in ALL_ICONS {
///     button(render(cp)).on_press(Msg::Pick(name.to_string()))
/// }
/// ```
pub fn render(codepoint: &str) -> Text<'_> {
    text(codepoint).font(Font::with_family("lucide"))
}

fn icon(codepoint: &str) -> Text<'_> {
    render(codepoint)
}

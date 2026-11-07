// Generated automatically by iced_lucide at build time.
// Do not edit manually.
// 3cab32e3737b5f315cc978685e952f0f9fcab68a85817f8fcc08eabe131fdb4b

//! Icon fonts for this crate.
//!
//! Each font here is a subset containing only the icons this
//! project asks for, cut from these upstream sets:
//!
//! - **Lucide** — ISC — <https://lucide.dev/icons>
//!
//! Redistributing these fonts means carrying their licenses with you.

#![allow(dead_code)]

use iced::Font;
use iced::widget::Text;

/// Lucide subset — ISC.
pub const FONT_LUCIDE: &[u8] = include_bytes!("../fonts/lucide.ttf");

/// Every font this module needs.
///
/// Register all of them before rendering any icon:
///
/// ```ignore
/// let mut app = iced::application(App::new, App::update, App::view);
///
/// for font in icon::FONTS {
///     app = app.fonts([*font]);
/// }
///
/// app.run()
/// ```
// One entry per line stays readable as the list grows, and keeps a
// `cargo fmt` in the consuming crate from rewriting generated output.
#[rustfmt::skip]
pub const FONTS: &[&[u8]] = &[
    FONT_LUCIDE,
];

/// The single font this module uses.
pub const FONT: &[u8] = FONT_LUCIDE;

/// One of the icon sets this module draws from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Family {
    /// Identifier, matching [`Icon::family`].
    pub id: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// The `iced_lucide` Cargo feature that provides this set.
    pub feature: &'static str,
    /// License the icons are published under.
    pub license: &'static str,
    /// Where to browse the set.
    pub url: &'static str,
}

/// The icon sets this module draws from.
///
/// [`Icon::family`] holds an `id` from this list, so it is both what a
/// filter UI wants and where an "about" screen finds its attribution.
#[rustfmt::skip]
pub const FAMILIES: &[Family] = &[
    Family { id: "lucide", name: "Lucide", feature: "lucide", license: "ISC", url: "https://lucide.dev/icons" },
];

const LUCIDE: Font = Font::new("lucide");

/// An icon: its name, the character that draws it, and the font that carries it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Icon {
    /// The upstream icon name.
    pub name: &'static str,
    /// The character to render.
    pub codepoint: &'static str,
    /// The font the character belongs to.
    pub font: Font,
    /// Identifier of the set this icon came from, matching [`FAMILIES`].
    pub family: &'static str,
}

/// Every icon in this module.
///
/// Use this with [`render`] to build a picker:
///
/// ```ignore
/// for icon in ALL_ICONS {
///     button(render(*icon)).on_press(Message::Pick(icon.name))
/// }
/// ```
// One icon per line; rustfmt would otherwise explode each struct
// literal across five lines and make this unreadable.
#[rustfmt::skip]
pub const ALL_ICONS: &[Icon] = &[
    Icon { name: "arrow-left", codepoint: "\u{E048}", font: LUCIDE, family: "lucide" },
    Icon { name: "moon", codepoint: "\u{E11E}", font: LUCIDE, family: "lucide" },
    Icon { name: "sun", codepoint: "\u{E178}", font: LUCIDE, family: "lucide" },
];

/// `arrow-left` from Lucide.
pub fn arrow_left<'a, Theme>() -> Text<'a, Theme>
where
    Theme: iced::widget::text::Catalog + 'a,
{
    iced::widget::text("\u{E048}").font(LUCIDE)
}

/// `moon` from Lucide.
pub fn moon<'a, Theme>() -> Text<'a, Theme>
where
    Theme: iced::widget::text::Catalog + 'a,
{
    iced::widget::text("\u{E11E}").font(LUCIDE)
}

/// `sun` from Lucide.
pub fn sun<'a, Theme>() -> Text<'a, Theme>
where
    Theme: iced::widget::text::Catalog + 'a,
{
    iced::widget::text("\u{E178}").font(LUCIDE)
}

/// Render any icon from [`ALL_ICONS`].
pub fn render<'a, Theme>(icon: Icon) -> Text<'a, Theme>
where
    Theme: iced::widget::text::Catalog + 'a,
{
    iced::widget::text(icon.codepoint).font(icon.font)
}

/// Look an icon up by its upstream name.
pub fn find(name: &str) -> Option<Icon> {
    ALL_ICONS.iter().find(|icon| icon.name == name).copied()
}

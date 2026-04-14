pub mod card;
pub mod footer;
pub mod metric;
pub mod sizing;
pub mod table;

/// A design token resolved to a concrete color by the active theme.
#[derive(serde::Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Token {
    Primary,
    Secondary,
    Accent,
    Success,
    Danger,
    Warning,
    Info,
    Muted,
}

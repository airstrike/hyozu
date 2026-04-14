use iced::{Color, Theme};

use crate::model;

// ── Layout ──────────────────────────────────────────────────────────

pub const COLS: u16 = 12;
pub const CELL_H: f32 = 200.0;
pub const GRID_SPACING: f32 = 4.0;

// ── Token resolution ────────────────────────────────────────────────

/// Resolves a design token to a concrete color from the active theme.
pub fn resolve(token: model::Token, theme: &Theme) -> Color {
    let palette = theme.palette();
    match token {
        model::Token::Primary => palette.primary.base.color,
        model::Token::Secondary => palette.secondary.base.color,
        model::Token::Accent => palette.primary.strong.color,
        model::Token::Success => palette.success.base.color,
        model::Token::Danger => palette.danger.base.color,
        model::Token::Warning => palette.warning.base.color,
        model::Token::Info => palette.primary.weak.color,
        model::Token::Muted => palette.background.strong.text,
    }
}

/// Maps a metric delta direction to a theme color.
pub fn direction_color(dir: model::metric::Direction, theme: &Theme) -> Color {
    let palette = theme.palette();
    match dir {
        model::metric::Direction::Positive => palette.success.base.color,
        model::metric::Direction::Negative => palette.danger.base.color,
        model::metric::Direction::Neutral => palette.background.strong.text,
    }
}

/// Returns the muted text color from the theme palette.
pub fn muted(theme: &Theme) -> Color {
    theme.palette().background.strong.text
}

// ── Timestamp humanization ──────────────────────────────────────────

/// Formats an ISO 8601 timestamp as a relative string ("4m ago").
pub fn humanize(iso: &str) -> String {
    let Ok(ts) = iso.parse::<jiff::Timestamp>() else {
        return iso.to_string();
    };
    let now = jiff::Timestamp::now();
    let secs = now.as_second() - ts.as_second();

    if secs < 0 {
        return iso.to_string();
    }

    match secs {
        0..60 => "just now".to_string(),
        60..3600 => format!("{}m ago", secs / 60),
        3600..86400 => format!("{}h ago", secs / 3600),
        _ => format!("{}d ago", secs / 86400),
    }
}

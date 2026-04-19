//! [`tatami::scalar::Result`] → KPI card iced [`Element`].
//!
//! A KPI card is `column![title, (subtitle), primary_value, (delta), (sparkline)]`
//! — a text-forward composition hyozu doesn't (yet) ship as a dedicated widget.
//! The adapter renders one here so dashboards drive KPI tiles directly from a
//! scalar query.
//!
//! The `KpiLayout` struct carries the rendering hints — which metric index is
//! the primary value, which (if any) is the delta, optional sparkline data.
//! Sparklines don't come from a scalar query (scalar is a single cell); the
//! caller passes trend values separately, typically sourced from a companion
//! series query.

use crate::core::{Element, Font, Length, Pixels, Theme};
use crate::sparkline::sparkline;
use crate::tatami::cell::cell_f64;
use crate::widget::{Renderer, column, text};
use tatami::{Cell, scalar};

/// Layout hints for [`card`] — which cells to render as primary value /
/// delta, plus optional sparkline data and headings.
#[derive(Default)]
pub struct KpiLayout<'a> {
    /// Card heading (e.g. "FY2026 Revenue").
    pub title: &'a str,
    /// Optional muted second line (e.g. "vs prior month").
    pub subtitle: Option<&'a str>,
    /// Index into [`scalar::Result::values`] for the primary value. Defaults
    /// to 0.
    pub primary: usize,
    /// Optional index into [`scalar::Result::values`] for the delta cell.
    pub delta: Option<usize>,
    /// Optional trend values for a sparkline below the delta. Expected
    /// length ≥ 2. Values outside the typical `f64` numeric range render
    /// as flat.
    pub sparkline: Option<&'a [f64]>,
}

// Bold font resolution uses hyozu's standard text::Style pattern — the same
// one mark labels (bar/line/axis) use. `text::Style::new().weight(text::Bold)`
// becomes a concrete `Font` via `resolved_font(Font::DEFAULT)`. Fully-
// qualified path on `crate::text` avoids shadowing the iced `text(..)`
// widget function imported above.
fn bold_font() -> Font {
    crate::text::Style::new()
        .weight(crate::text::Bold)
        .resolved_font(Font::DEFAULT)
}

/// Build a KPI card [`Element`] from a scalar result + layout.
///
/// The adapter is [`Message`]-generic — the card is inert. Callers that
/// need click handlers wrap the returned element in their own interactive
/// container.
#[must_use]
pub fn card<'a, Message: 'a>(result: &scalar::Result, layout: KpiLayout<'a>) -> Element<'a, Message, Theme, Renderer> {
    let values = result.values();

    let title = text(layout.title.to_owned()).font(bold_font()).size(14.0);

    let primary_text = values
        .get(layout.primary)
        .map(format_cell)
        .unwrap_or_else(|| "—".to_string());
    let primary = text(primary_text).size(Pixels(28.0));

    let mut col = column![title];
    if let Some(sub) = layout.subtitle {
        col = col.push(text(sub.to_owned()).size(12.0));
    }
    col = col.push(primary);

    if let Some(i) = layout.delta
        && let Some(cell) = values.get(i)
    {
        let delta_str = format_delta(cell);
        col = col.push(text(delta_str).size(12.0));
    }

    if let Some(spark_values) = layout.sparkline
        && spark_values.len() >= 2
    {
        col = col.push(sparkline(spark_values).width(Length::Fill).height(Length::Fixed(28.0)));
    }

    col.spacing(4).into()
}

fn format_cell(cell: &Cell) -> String {
    let v = cell_f64(cell);
    if v.is_nan() {
        match cell {
            Cell::Error { message } => format!("⚠ {message}"),
            _ => "—".into(),
        }
    } else if v.abs() >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v.abs() >= 1_000.0 {
        format!("{:.1}k", v / 1_000.0)
    } else {
        format!("{v:.2}")
    }
}

fn format_delta(cell: &Cell) -> String {
    let v = cell_f64(cell);
    if v.is_nan() {
        "—".into()
    } else if v > 0.0 {
        format!("▲ {:.1}%", v * 100.0)
    } else if v < 0.0 {
        format!("▼ {:.1}%", v.abs() * 100.0)
    } else {
        format!("· {:.1}%", v * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tatami::query::Tuple;

    #[test]
    fn format_cell_valid_small_number() {
        let c = Cell::Valid {
            value: 12.3,
            unit: None,
            format: None,
        };
        assert_eq!(format_cell(&c), "12.30");
    }

    #[test]
    fn format_cell_valid_millions() {
        let c = Cell::Valid {
            value: 24_800_000.0,
            unit: None,
            format: None,
        };
        assert_eq!(format_cell(&c), "24.8M");
    }

    #[test]
    fn format_cell_missing_dashes() {
        let c = Cell::Missing {
            reason: tatami::missing::Reason::NoFacts,
        };
        assert_eq!(format_cell(&c), "—");
    }

    #[test]
    fn format_cell_error_prefixes_warn_glyph() {
        let c = Cell::Error {
            message: "div/0".into(),
        };
        assert_eq!(format_cell(&c), "⚠ div/0");
    }

    #[test]
    fn format_delta_positive_uses_up_arrow() {
        let c = Cell::Valid {
            value: 0.042,
            unit: None,
            format: None,
        };
        assert_eq!(format_delta(&c), "▲ 4.2%");
    }

    #[test]
    fn format_delta_negative_uses_down_arrow() {
        let c = Cell::Valid {
            value: -0.018,
            unit: None,
            format: None,
        };
        assert_eq!(format_delta(&c), "▼ 1.8%");
    }

    #[test]
    fn card_builds_element_without_panic() {
        let result = scalar::Result::new(Tuple::empty(), vec![
            Cell::Valid {
                value: 24_800_000.0,
                unit: None,
                format: None,
            },
            Cell::Valid {
                value: 0.042,
                unit: None,
                format: None,
            },
        ]);
        let _: Element<'_, (), Theme, Renderer> = card(&result, KpiLayout {
            title: "FY2026 Revenue",
            subtitle: Some("vs prior month"),
            primary: 0,
            delta: Some(1),
            sparkline: Some(&[1.0, 1.2, 1.1, 1.3, 1.4]),
        });
    }
}

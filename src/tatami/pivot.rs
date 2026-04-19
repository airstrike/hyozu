//! [`tatami::pivot::Result`] → iced table [`Element`].
//!
//! Renders a row/column grid from a pivot result. Row + column headers are
//! stringified from each [`tatami::Tuple`] — last-segment of the head
//! member's path, joined with spaces if the tuple has multiple members.
//! Cell values format by their [`tatami::Cell::Valid::format`] hint
//! (percent-aware); missing cells render as `—`, errors as `⚠ {message}`.
//!
//! v0.1 scope: a plain row/column grid using iced primitives. Conditional
//! colouring (red/green for variance) is out of scope here — callers that
//! want it can either wrap the returned element or use the cell formatter
//! [`format_cell`] plus their own row builder.

use crate::core::{Element, Font, Length, Theme};
use crate::widget::{Column, Renderer, column, container, row, rule, text};
use tatami::{Cell, Tuple, pivot};

// Bold font resolution uses hyozu's standard `text::Style` pattern — the
// same one mark labels (bar/line/axis) use. `text::Style::new().weight(text::Bold)`
// becomes a concrete `Font` via `resolved_font(Font::DEFAULT)`. Fully-
// qualified path on `crate::text` avoids shadowing the iced `text(..)`
// widget function imported above.
fn bold_font() -> Font {
    crate::text::Style::new()
        .weight(crate::text::Bold)
        .resolved_font(Font::DEFAULT)
}

/// Build a pivot-table [`Element`] from a [`pivot::Result`].
///
/// Pins `Theme = crate::core::Theme` (iced's default) to match hyozu's other
/// convenience helpers; a custom-theme caller can copy this function as a
/// starting template.
#[must_use]
pub fn table<'a, Message: 'a>(result: &'a pivot::Result) -> Element<'a, Message, Theme, Renderer> {
    // Header row: blank top-left corner cell, then one cell per col header.
    let header_cells =
        std::iter::once(empty_cell::<Message>()).chain(result.col_headers().iter().map(header_cell::<Message>));
    let header: Element<'a, Message, Theme, Renderer> = row(header_cells).spacing(8).into();

    // Body rows: row-header label + one cell per column.
    let body_rows = result.row_headers().iter().zip(result.cells().iter()).map(
        |(row_header, cells)| -> Element<'a, Message, Theme, Renderer> {
            let cells_iter =
                std::iter::once(header_cell::<Message>(row_header)).chain(cells.iter().map(value_cell::<Message>));
            row(cells_iter).spacing(8).into()
        },
    );

    column![header, rule::horizontal(2), Column::with_children(body_rows).spacing(4),]
        .spacing(4)
        .into()
}

/// Format a single cell for display in a pivot. Honors the `format` hint on
/// `Valid` cells: strings containing `%` render the value × 100 with a `%`
/// suffix; anything else falls back to a magnitude-scaled numeric.
#[must_use]
pub fn format_cell(cell: &Cell) -> String {
    match cell {
        Cell::Valid { value, format, .. } => {
            if let Some(fmt) = format.as_ref()
                && fmt.as_str().contains('%')
            {
                format!("{:.1}%", *value * 100.0)
            } else if value.abs() >= 1_000_000.0 {
                format!("{:.2}M", *value / 1_000_000.0)
            } else if value.abs() >= 1_000.0 {
                format!("{:.1}k", *value / 1_000.0)
            } else {
                format!("{value:.2}")
            }
        }
        Cell::Missing { .. } => "—".to_string(),
        Cell::Error { message } => format!("⚠ {message}"),
        // `Cell` is `#[non_exhaustive]`.
        _ => "?".to_string(),
    }
}

fn header_cell<'a, Message: 'a>(tuple: &Tuple) -> Element<'a, Message, Theme, Renderer> {
    container(
        text(format_tuple(tuple))
            .font(bold_font())
            .size(12.0)
            .width(Length::Fill),
    )
    .width(Length::FillPortion(2))
    .into()
}

fn value_cell<'a, Message: 'a>(cell: &Cell) -> Element<'a, Message, Theme, Renderer> {
    container(text(format_cell(cell)).size(12.0).width(Length::Fill))
        .width(Length::FillPortion(3))
        .into()
}

fn empty_cell<'a, Message: 'a>() -> Element<'a, Message, Theme, Renderer> {
    container(text("")).width(Length::FillPortion(2)).into()
}

fn format_tuple(tuple: &Tuple) -> String {
    let segments: Vec<String> = tuple
        .members()
        .iter()
        .map(|m| {
            m.path
                .segments()
                .last()
                .map(|s| s.as_str().to_string())
                .unwrap_or_default()
        })
        .filter(|s| !s.is_empty())
        .collect();
    if segments.is_empty() {
        "—".to_string()
    } else {
        segments.join(" · ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tatami::query::Tuple;
    use tatami::schema::Format;

    fn valid(v: f64) -> Cell {
        Cell::Valid {
            value: v,
            unit: None,
            format: None,
        }
    }

    fn valid_pct(v: f64) -> Cell {
        Cell::Valid {
            value: v,
            unit: None,
            format: Some(Format::new("0.0%")),
        }
    }

    #[test]
    fn format_cell_honors_percent_hint() {
        let c = valid_pct(0.724);
        assert_eq!(format_cell(&c), "72.4%");
    }

    #[test]
    fn format_cell_scales_magnitudes_without_format_hint() {
        assert_eq!(format_cell(&valid(24_800_000.0)), "24.80M");
        // `{:.1}` rounds-to-even: 1.25 → "1.2"; 1350 → "1.4" (also even).
        assert_eq!(format_cell(&valid(1_250.0)), "1.2k");
        assert_eq!(format_cell(&valid(0.42)), "0.42");
    }

    #[test]
    fn format_cell_missing_and_error_paths() {
        assert_eq!(
            format_cell(&Cell::Missing {
                reason: tatami::missing::Reason::NoFacts
            }),
            "—"
        );
        assert_eq!(format_cell(&Cell::Error { message: "boom".into() }), "⚠ boom");
    }

    #[test]
    fn table_builds_element_without_panic() {
        let r = pivot::Result::new(vec![Tuple::empty()], vec![Tuple::empty(), Tuple::empty()], vec![vec![
            valid(1.0),
            valid(2.0),
        ]]);
        let _: Element<'_, (), Theme, Renderer> = table(&r);
    }
}

//! Combo chart — two grouped bar series on the primary y-axis plus one
//! line series on the secondary y-axis. Mirrors the shape inboard's
//! "Revenue × GOP Margin by Month" tile produces from
//! `src/workspace/infolet/view/chart.rs::project_series_data`.
//!
//! Inboard's `build_axis_marks` splits the authored series by
//! `axis_side` (Primary vs Secondary), then for each side calls
//! `chart_build::bar_marks(...)` or `line_marks(...)` on the "majority"
//! kind. For a Column chart with two Revenue bars (Primary) and one
//! GOP-margin line (Secondary), that produces:
//!
//!   - primary: `hyozu::bars(vec![bar(curr), bar(prior)]).with_layout(Grouped)`
//!     wrapped as a single Bars mark
//!   - secondary: `vec![Mark::Line(margin)]`
//!   - stitched via `primary.secondary(secondary_marks)`
//!
//! Reported bug: the rendered tile shows all bars as one combined
//! series and the line is missing / mis-routed. This example builds
//! the same Data shape using the same hyozu API the inboard renderer
//! calls — so running it tells you whether the bug is in hyozu itself
//! or in inboard's data construction upstream of this point.
//!
//! If hyozu renders this example correctly (two distinct bar groups
//! per month + a dashed line on a right-side axis), the bug is in
//! inboard. If hyozu renders it the same broken way (one fused bar
//! series, no line on the right axis), the bug is in hyozu.

use iced::widget::center;
use iced::{Task, Theme};

use hyozu::data::mark::bar::Layout as BarLayout;
use hyozu::{LineStyle, Mark, bar, bars, data, line};

pub fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .window_size([1000.0, 560.0])
        .title("hyozu • combo chart (two bars + secondary line)")
        .theme(App::theme)
        .run()
}

struct App {
    chart: hyozu::Data,
}

pub type Message = ();

/// Hewton FY2025 monthly labels, matching the tile's x-axis.
const MONTHS: [&str; 12] = [
    "Jul 24", "Aug 24", "Sep 24", "Oct 24", "Nov 24", "Dec 24", "Jan 25", "Feb 25", "Mar 25", "Apr 25", "May 25",
    "Jun 25",
];

impl App {
    fn new() -> Self {
        // Two primary-axis bar series. In inboard these are produced
        // by `chart_build::bar_marks(series, labels, Layout::Grouped,
        // ...)` where `series` is the Vec<SeriesData> for both
        // Revenue (current) and Revenue (prior year). The renderer
        // names each via `bar(...).with_name(...)` from the
        // SeriesData::label field; colors come through the palette
        // resolver. Here we hand-pick plausible Hewton numbers.
        let revenue_current: Vec<f64> = vec![
            12_400_000.0,
            13_800_000.0,
            11_900_000.0,
            14_200_000.0,
            15_600_000.0,
            17_300_000.0,
            10_800_000.0,
            11_500_000.0,
            13_100_000.0,
            14_900_000.0,
            16_200_000.0,
            18_400_000.0,
        ];
        let revenue_prior: Vec<f64> = vec![
            11_100_000.0,
            12_300_000.0,
            10_700_000.0,
            12_900_000.0,
            14_100_000.0,
            15_800_000.0,
            9_700_000.0,
            10_400_000.0,
            11_800_000.0,
            13_400_000.0,
            14_600_000.0,
            16_700_000.0,
        ];

        // GOP margin %: secondary-axis line series. In inboard this
        // is one row of the `tatami::series::Result` whose authored
        // `axis_side == Secondary` — partitioned off the primary
        // bars and routed via `primary_marks.secondary(...)`.
        let gop_margin: Vec<f64> = vec![0.34, 0.36, 0.32, 0.37, 0.39, 0.42, 0.28, 0.30, 0.33, 0.37, 0.40, 0.44];

        // Primary side — Grouped bars, exactly as
        // `chart_build::bar_marks` produces. The vec inside
        // `hyozu::bars(...)` is what differentiates "two grouped
        // series" from "one fused series": both bars are siblings
        // inside a single Bars mark, and `.with_layout(Grouped)`
        // tells hyozu to render them side-by-side per category.
        let revenue_bars = bars(vec![
            bar(revenue_current).with_name("Revenue (FY25)"),
            bar(revenue_prior).with_name("Revenue (FY24)"),
        ])
        .with_layout(BarLayout::Grouped);

        // Secondary side — line series. Inboard wraps this as a
        // `Mark::Line(...)` and passes a `vec![Mark::Line(...)]`
        // into `.secondary(...)`. Dashed style + a tighter stroke
        // matches the convention dual_axis.rs uses for the
        // secondary-axis overlay.
        let gop_line = line(gop_margin).with_name("GOP Margin %").style(LineStyle::Dashed);

        Self {
            chart: data(revenue_bars)
                .x_axis_labels(MONTHS)
                .y_axis_labels(currency)
                .y_axis(|a| a.title("Revenue").show_grid(true))
                .secondary(vec![Mark::Line(gop_line)])
                .right_axis(|a| a.title("GOP Margin").labels(percent).with_bounds(0.0, 0.6))
                .title("Revenue × GOP Margin by Month — Hewton FY2025")
                .legend(hyozu::legend::Config::above()),
        }
    }

    fn view(&self) -> iced::Element<'_, Message> {
        center(
            hyozu::chart(&self.chart)
                .design(&Theme::Light)
                .padding(20)
                .style(|d| hyozu::chart::Style {
                    corners: 3.0.into(),
                    ..hyozu::chart::default(d)
                }),
        )
        .padding(20)
        .into()
    }

    fn update(&mut self, _: Message) -> Task<Message> {
        Task::none()
    }

    fn theme(&self) -> Theme {
        Theme::Light
    }
}

/// Compact dollar format — `$12.4M` etc. The inboard tile uses a
/// `format::Spec` with the millions scale; we approximate it here.
fn currency(value: f64) -> String {
    let abs = value.abs();
    let (scaled, suffix) = if abs >= 1_000_000_000.0 {
        (value / 1_000_000_000.0, "B")
    } else if abs >= 1_000_000.0 {
        (value / 1_000_000.0, "M")
    } else if abs >= 1_000.0 {
        (value / 1_000.0, "K")
    } else {
        (value, "")
    };
    if value < 0.0 {
        format!("-${:.1}{}", scaled.abs(), suffix)
    } else {
        format!("${:.1}{}", scaled, suffix)
    }
}

/// Whole-number percentage — `34%` for `0.34`.
fn percent(v: f64) -> String {
    format!("{:.0}%", v * 100.0)
}

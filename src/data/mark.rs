pub mod bar;
pub mod line;

pub use bar::{Bars, IntoBars, bar, bars};
pub use line::{IntoLines, Line, line};

/// Represents a visual mark in a chart (bars, lines, scatter, etc.)
#[derive(Debug, Clone)]
pub enum Mark {
    Bars(Bars),
    Line(Line),
}

impl From<Bars> for Mark {
    fn from(bars: Bars) -> Self {
        Mark::Bars(bars)
    }
}

impl From<Line> for Mark {
    fn from(line: Line) -> Self {
        Mark::Line(line)
    }
}

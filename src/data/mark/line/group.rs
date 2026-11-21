//! Excel's line chart group: several line series that share high-low lines
//! and up/down bars.
//!
//! Both decorations compare the series at each x position, so they belong to
//! the group rather than to any one line. A [`Group`] reduces its series to
//! one [`Column`] summary per x position and hands that summary to its first
//! line, which draws the decorations behind every series in the group. The
//! lines stay ordinary [`Line`] marks with their own per-point hover.

use super::{Connector, Line};
use crate::color::Color;
use crate::data::{IntoData, Mark};

/// Several line series drawn as one Excel chart group.
///
/// # Examples
///
/// ```
/// use hyozu::line::{self, Connector, UpDown};
///
/// let chart = hyozu::data(
///     line::group([line::line([3, 5, 4]), line::line([4, 4, 6])])
///         .high_low_lines(Connector::new())
///         .up_down_bars(UpDown::new()),
/// );
/// ```
#[derive(Debug, Clone)]
pub struct Group {
    lines: Vec<Line>,
    high_low: Option<Connector>,
    up_down: Option<UpDown>,
}

/// Bars between the first and the last series at each x position. An up bar
/// marks a position where the last series ends above the first; a down bar,
/// below.
///
/// Unset, up bars are hollow and down bars fill with the theme's text
/// color, both outlined in that ink: Excel's white and black bars, adapted
/// to whatever surface the chart sits on.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UpDown {
    up: Option<Color>,
    down: Option<Color>,
    border: Option<Color>,
    gap: Gap,
}

/// Space between neighboring up/down bars as a percentage of a bar's width,
/// Excel's "gap width": from 0 to 500, 150 by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Gap(u16);

/// What the series of a group hold at one x position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Column {
    pub(crate) x: f64,
    /// The lowest and highest finite values, when at least two series have
    /// one here.
    pub(crate) range: Option<(f64, f64)>,
    /// The first and last series' values, when both are finite here.
    pub(crate) ends: Option<(f64, f64)>,
}

/// The group summary carried by the first line of a [`Group`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Decorations {
    pub(crate) columns: Vec<Column>,
    pub(crate) high_low: Option<Connector>,
    pub(crate) up_down: Option<UpDown>,
}

/// Groups line series so they can share high-low lines and up/down bars.
pub fn group(lines: impl IntoIterator<Item = Line>) -> Group {
    Group {
        lines: lines.into_iter().collect(),
        high_low: None,
        up_down: None,
    }
}

impl Group {
    /// Draws a vertical line at each x position from the lowest to the
    /// highest value across the group's series.
    pub fn high_low_lines(mut self, connector: impl Into<Option<Connector>>) -> Self {
        self.high_low = connector.into();
        self
    }

    /// Draws up/down bars between the first and the last series.
    pub fn up_down_bars(mut self, bars: impl Into<Option<UpDown>>) -> Self {
        self.up_down = bars.into();
        self
    }

    /// Draws drop lines from every point of every series in the group.
    pub fn drop_lines(mut self, connector: impl Into<Option<Connector>>) -> Self {
        let connector = connector.into();
        for line in &mut self.lines {
            line.drop_lines = connector.clone();
        }
        self
    }

    /// Returns the group's series.
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    /// Converts the group into its line marks, the first carrying the
    /// group's decorations.
    pub fn into_marks(self) -> Vec<Mark> {
        let mut lines = self.lines;
        if self.high_low.is_some() || self.up_down.is_some() {
            let columns = columns(&lines.iter().map(|l| l.points.as_slice()).collect::<Vec<_>>());
            if let Some(first) = lines.first_mut() {
                first.group = Some(Box::new(Decorations {
                    columns,
                    high_low: self.high_low,
                    up_down: self.up_down,
                }));
            }
        }
        lines.into_iter().map(Mark::Line).collect()
    }
}

impl IntoData for Group {
    fn into_data(self) -> crate::Data {
        let categorical = self.up_down.is_some();
        let data = self.into_marks().into_data();
        if categorical {
            // Up/down bars occupy a band around each position, so the
            // positions sit between ticks as Excel's do.
            data.x_axis(|_| crate::Bars::axes(crate::data::mark::bar::Direction::Vertical).0)
        } else {
            data
        }
    }
}

impl From<Group> for crate::Data {
    fn from(group: Group) -> Self {
        group.into_data()
    }
}

impl UpDown {
    /// Up/down bars in the theme's colors with Excel's 150% gap width.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the fill of up bars.
    pub fn up(mut self, color: impl Into<Color>) -> Self {
        self.up = Some(color.into());
        self
    }

    /// Sets the fill of down bars.
    pub fn down(mut self, color: impl Into<Color>) -> Self {
        self.down = Some(color.into());
        self
    }

    /// Sets the outline of both bars.
    pub fn border(mut self, color: impl Into<Color>) -> Self {
        self.border = Some(color.into());
        self
    }

    /// Sets the gap width.
    pub fn gap(mut self, gap: impl Into<Gap>) -> Self {
        self.gap = gap.into();
        self
    }

    /// Returns the up bar fill, if set.
    pub fn up_value(&self) -> Option<&Color> {
        self.up.as_ref()
    }

    /// Returns the down bar fill, if set.
    pub fn down_value(&self) -> Option<&Color> {
        self.down.as_ref()
    }

    /// Returns the outline color, if set.
    pub fn border_value(&self) -> Option<&Color> {
        self.border.as_ref()
    }

    /// Returns the gap width.
    pub fn gap_value(&self) -> Gap {
        self.gap
    }
}

impl Gap {
    /// The widest gap Excel allows, in percent of a bar's width.
    pub const MAX: u16 = 500;

    /// A gap width in percent, clamped to `0..=500`.
    pub fn new(percent: u16) -> Self {
        Self(percent.min(Self::MAX))
    }

    /// Returns the gap width in percent.
    pub fn percent(self) -> u16 {
        self.0
    }

    /// The share of one category step a bar covers: `1 / (1 + gap)`.
    pub(crate) fn bar_share(self) -> f32 {
        1.0 / (1.0 + f32::from(self.0) / 100.0)
    }
}

impl Default for Gap {
    fn default() -> Self {
        Self(150)
    }
}

impl From<u16> for Gap {
    fn from(percent: u16) -> Self {
        Self::new(percent)
    }
}

/// Summarizes the series at each distinct finite x position, in ascending x.
fn columns(series: &[&[crate::data::Datum]]) -> Vec<Column> {
    let mut samples: Vec<(f64, usize, f64)> = series
        .iter()
        .enumerate()
        .flat_map(|(s, points)| points.iter().map(move |p| (p.x, s, p.y)))
        .filter(|(x, _, _)| x.is_finite())
        .collect();
    samples.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let last = series.len().saturating_sub(1);
    samples
        .chunk_by(|a, b| a.0 == b.0)
        .map(|chunk| {
            let finite = chunk.iter().filter(|(_, _, y)| y.is_finite());
            let (count, lo, hi) = finite.fold((0, f64::INFINITY, f64::NEG_INFINITY), |(n, lo, hi), (_, _, y)| {
                (n + 1, lo.min(*y), hi.max(*y))
            });
            let value = |s: usize| {
                chunk
                    .iter()
                    .find(|(_, series, y)| *series == s && y.is_finite())
                    .map(|(_, _, y)| *y)
            };
            let ends = if last > 0 { value(0).zip(value(last)) } else { None };
            Column {
                x: chunk[0].0,
                range: (count >= 2).then_some((lo, hi)),
                ends,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::line::line;

    #[test]
    fn columns_span_every_series_at_each_position() {
        let marks = group([
            line([(0.0, 3.0), (1.0, 5.0), (2.0, f64::NAN)]),
            line([(0.0, 7.0), (1.0, 2.0), (2.0, 4.0)]),
            line([(0.0, 4.0), (1.0, 6.0), (2.0, 1.0)]),
        ])
        .high_low_lines(Connector::new())
        .up_down_bars(UpDown::new())
        .into_marks();
        let Mark::Line(first) = &marks[0] else { panic!("line") };
        let decorations = first.group.as_deref().expect("decorations on the first line");
        assert_eq!(decorations.columns, [
            Column {
                x: 0.0,
                range: Some((3.0, 7.0)),
                ends: Some((3.0, 4.0))
            },
            Column {
                x: 1.0,
                range: Some((2.0, 6.0)),
                ends: Some((5.0, 6.0))
            },
            Column {
                x: 2.0,
                range: Some((1.0, 4.0)),
                ends: None
            },
        ]);
        let Mark::Line(second) = &marks[1] else { panic!("line") };
        assert!(second.group.is_none());
    }

    #[test]
    fn undecorated_groups_are_plain_lines() {
        let marks = group([line([1, 2]), line([2, 3])]).into_marks();
        assert!(marks.iter().all(|m| matches!(m, Mark::Line(l) if l.group.is_none())));
    }

    #[test]
    fn gap_width_clamps_to_excel_range() {
        assert_eq!(Gap::new(900).percent(), 500);
        assert!((Gap::default().bar_share() - 0.4).abs() < 1e-6);
    }
}

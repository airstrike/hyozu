//! Small multiples with independent chart state and optionally shared axes.
//!
//! Build a [`Grid`] from finished panels with [`Grid::new`], or from data
//! rows grouped by a key:
//!
//! - [`wrap`] splits rows by one key and wraps the panels into columns,
//!   each under a strip naming its group (ggplot's `facet_wrap`).
//! - [`grid`] crosses a row key with a column key into a matrix, with
//!   column strips on top and row strips on the right (`facet_grid`).
//!
//! Groups keep the order their keys first appear in, so pre-sorted rows
//! give sorted panels; [`Wrap::sorted`] and [`Cross::sorted`] sort by
//! label instead. Shared axes show their tick labels on the outer panels
//! only: y labels on the first column, x labels on the bottom row.
use std::fmt::Display;

use crate::Data;
use crate::core::{Element, Length, alignment};
use crate::widget::{Column, Row, Widget};

/// Which axes the panels of a [`Grid`] share.
#[derive(Clone, Copy, Debug)]
pub enum Scales {
    /// Every panel shares both axes.
    Fixed,
    /// The x axis is free: per panel in a wrap, per column in a cross.
    FreeX,
    /// The y axis is free: per panel in a wrap, per row in a cross.
    FreeY,
    /// Both axes are free.
    Free,
}

impl Scales {
    fn shares_x(self) -> bool {
        matches!(self, Self::Fixed | Self::FreeY)
    }

    fn shares_y(self) -> bool {
        matches!(self, Self::Fixed | Self::FreeX)
    }
}

/// The rows of one facet, under its strip label.
#[derive(Clone, Debug, PartialEq)]
pub struct Group<R> {
    row: Option<String>,
    column: String,
    rows: Vec<R>,
}

impl<R> Group<R> {
    /// The strip label: the wrap key, or the column key of a cross.
    pub fn label(&self) -> &str {
        &self.column
    }

    /// The row key of a cross; `None` in a wrap.
    pub fn row(&self) -> Option<&str> {
        self.row.as_deref()
    }

    /// The data rows in this facet, in their original order.
    pub fn rows(&self) -> &[R] {
        &self.rows
    }
}

/// Rows split by one key, ready to wrap into panels. Built by [`wrap`].
#[derive(Clone, Debug)]
pub struct Wrap<R> {
    groups: Vec<Group<R>>,
    columns: usize,
    scales: Scales,
}

/// Splits `rows` into one facet per distinct `key`, in first-appearance
/// order, wrapped into three columns with fixed scales by default.
pub fn wrap<R, K: Display>(rows: impl IntoIterator<Item = R>, key: impl Fn(&R) -> K) -> Wrap<R> {
    let mut groups: Vec<Group<R>> = Vec::new();
    for row in rows {
        let label = key(&row).to_string();
        match groups.iter_mut().find(|group| group.column == label) {
            Some(group) => group.rows.push(row),
            None => groups.push(Group {
                row: None,
                column: label,
                rows: vec![row],
            }),
        }
    }
    Wrap {
        groups,
        columns: 3,
        scales: Scales::Fixed,
    }
}

impl<R> Wrap<R> {
    /// Sets how many panels sit side by side before wrapping.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns;
        self
    }

    /// Sets which axes the panels share.
    pub fn scales(mut self, scales: Scales) -> Self {
        self.scales = scales;
        self
    }

    /// Orders the facets by label instead of first appearance.
    pub fn sorted(mut self) -> Self {
        self.groups.sort_by(|a, b| a.column.cmp(&b.column));
        self
    }

    /// The facets, in panel order.
    pub fn groups(&self) -> &[Group<R>] {
        &self.groups
    }

    /// Charts each facet and lays the panels out. Returns `None` for zero
    /// columns.
    pub fn chart(self, mut chart: impl FnMut(&Group<R>) -> Data) -> Option<Grid> {
        if self.columns == 0 {
            return None;
        }
        let mut panels: Vec<Data> = self.groups.iter().map(&mut chart).collect();
        let all: Vec<usize> = (0..panels.len()).collect();
        pool(&mut panels, &all, self.scales.shares_x(), self.scales.shares_y());
        let slots = (0..panels.len()).map(Some).collect();
        let mut grid = Grid {
            strips: Strips::Panels(self.groups.into_iter().map(|group| group.column).collect()),
            panels,
            columns: self.columns,
            slots,
        };
        grid.outer_axes(self.scales.shares_x(), self.scales.shares_y());
        Some(grid)
    }
}

/// Rows crossed by a row key and a column key. Built by [`grid`].
#[derive(Clone, Debug)]
pub struct Cross<R> {
    rows: Vec<String>,
    columns: Vec<String>,
    /// Row-major cells; `None` where no data row has that key pair.
    cells: Vec<Option<Group<R>>>,
    scales: Scales,
}

/// Crosses `rows` by `row_key` and `column_key` into a facet matrix, each
/// key in first-appearance order, with fixed scales by default. Key pairs
/// with no rows leave an empty cell.
pub fn grid<R, A: Display, B: Display>(
    rows: impl IntoIterator<Item = R>,
    row_key: impl Fn(&R) -> A,
    column_key: impl Fn(&R) -> B,
) -> Cross<R> {
    let mut row_labels: Vec<String> = Vec::new();
    let mut column_labels: Vec<String> = Vec::new();
    let mut keyed = Vec::new();
    let index = |labels: &mut Vec<String>, label: String| match labels.iter().position(|l| *l == label) {
        Some(i) => i,
        None => {
            labels.push(label);
            labels.len() - 1
        }
    };
    for row in rows {
        let r = index(&mut row_labels, row_key(&row).to_string());
        let c = index(&mut column_labels, column_key(&row).to_string());
        keyed.push((r, c, row));
    }
    let mut cells: Vec<Option<Group<R>>> = (0..row_labels.len() * column_labels.len()).map(|_| None).collect();
    for (r, c, row) in keyed {
        cells[r * column_labels.len() + c]
            .get_or_insert_with(|| Group {
                row: Some(row_labels[r].clone()),
                column: column_labels[c].clone(),
                rows: Vec::new(),
            })
            .rows
            .push(row);
    }
    Cross {
        rows: row_labels,
        columns: column_labels,
        cells,
        scales: Scales::Fixed,
    }
}

impl<R> Cross<R> {
    /// Sets which axes the panels share. A free x axis stays shared down
    /// each column and a free y axis across each row, as in ggplot.
    pub fn scales(mut self, scales: Scales) -> Self {
        self.scales = scales;
        self
    }

    /// Orders both the row and the column keys by label instead of first
    /// appearance.
    pub fn sorted(self) -> Self {
        let order = |labels: &[String]| {
            let mut order: Vec<usize> = (0..labels.len()).collect();
            order.sort_by(|&a, &b| labels[a].cmp(&labels[b]));
            order
        };
        let row_order = order(&self.rows);
        let column_order = order(&self.columns);
        let width = self.columns.len();
        let mut cells: Vec<Option<Group<R>>> = self.cells.into_iter().collect();
        let mut sorted = Vec::with_capacity(cells.len());
        for &r in &row_order {
            for &c in &column_order {
                sorted.push(cells[r * width + c].take());
            }
        }
        Self {
            rows: row_order.iter().map(|&r| self.rows[r].clone()).collect(),
            columns: column_order.iter().map(|&c| self.columns[c].clone()).collect(),
            cells: sorted,
            scales: self.scales,
        }
    }

    /// The row keys, top to bottom.
    pub fn row_labels(&self) -> &[String] {
        &self.rows
    }

    /// The column keys, left to right.
    pub fn column_labels(&self) -> &[String] {
        &self.columns
    }

    /// The facet at a row and column, if any rows have that key pair.
    pub fn group(&self, row: usize, column: usize) -> Option<&Group<R>> {
        if column >= self.columns.len() {
            return None;
        }
        self.cells.get(row * self.columns.len() + column)?.as_ref()
    }

    /// Charts each non-empty facet and lays out the matrix. Returns `None`
    /// when there are no rows.
    pub fn chart(self, mut chart: impl FnMut(&Group<R>) -> Data) -> Option<Grid> {
        let width = self.columns.len();
        if width == 0 {
            return None;
        }
        let mut panels = Vec::new();
        let slots: Vec<Option<usize>> = self
            .cells
            .iter()
            .map(|cell| {
                cell.as_ref().map(|group| {
                    panels.push(chart(group));
                    panels.len() - 1
                })
            })
            .collect();
        let cells_where = |keep: &dyn Fn(usize) -> bool| -> Vec<usize> {
            slots
                .iter()
                .enumerate()
                .filter(|(cell, _)| keep(*cell))
                .filter_map(|(_, slot)| *slot)
                .collect()
        };
        if self.scales.shares_x() {
            pool(&mut panels, &cells_where(&|_| true), true, false);
        } else {
            for c in 0..width {
                pool(&mut panels, &cells_where(&|cell| cell % width == c), true, false);
            }
        }
        if self.scales.shares_y() {
            pool(&mut panels, &cells_where(&|_| true), false, true);
        } else {
            for r in 0..self.rows.len() {
                pool(&mut panels, &cells_where(&|cell| cell / width == r), false, true);
            }
        }
        let mut grid = Grid {
            panels,
            columns: width,
            slots,
            strips: Strips::Margins {
                rows: self.rows,
                columns: self.columns,
            },
        };
        // Within a column x is always shared, within a row y is, so only
        // the outer panels need tick labels.
        grid.outer_axes(true, true);
        Some(grid)
    }
}

/// Strip labels drawn around the panels.
#[derive(Clone, Debug)]
enum Strips {
    /// Panels keep their own titles.
    None,
    /// One strip above each panel, in panel order.
    Panels(Vec<String>),
    /// Column strips across the top and row strips down the right side.
    Margins { rows: Vec<String>, columns: Vec<String> },
}

/// A laid-out set of chart panels.
#[derive(Clone, Debug)]
pub struct Grid {
    panels: Vec<Data>,
    columns: usize,
    /// Row-major cells, each holding a panel index or nothing.
    slots: Vec<Option<usize>>,
    strips: Strips,
}

impl Grid {
    /// Panels retain their titles, palettes and secondary axes. Shared bounds
    /// are calculated from the pooled primary data once, at construction.
    pub fn new(mut panels: Vec<Data>, columns: usize, scales: Scales) -> Option<Self> {
        if columns == 0 {
            return None;
        }
        let all: Vec<usize> = (0..panels.len()).collect();
        pool(&mut panels, &all, scales.shares_x(), scales.shares_y());
        Some(Self {
            slots: all.into_iter().map(Some).collect(),
            panels,
            columns,
            strips: Strips::None,
        })
    }

    pub fn panels(&self) -> &[Data] {
        &self.panels
    }
    pub fn panel_mut(&mut self, index: usize) -> Option<&mut Data> {
        self.panels.get_mut(index)
    }

    /// Hides tick labels on axes shared with a neighbor: y labels off the
    /// first column, x labels off the bottom row of cells.
    fn outer_axes(&mut self, x: bool, y: bool) {
        let rows = self.slots.len().div_ceil(self.columns);
        for (cell, slot) in self.slots.iter().enumerate() {
            let Some(panel) = slot.and_then(|index| self.panels.get_mut(index)) else {
                continue;
            };
            let (row, column) = (cell / self.columns, cell % self.columns);
            let mut changed = false;
            if x && row + 1 < rows
                && let Some(axis) = panel.x_axis_mut()
            {
                *axis = axis.clone().show_labels(false);
                changed = true;
            }
            if y && column > 0
                && let Some(axis) = panel.y_axis_mut()
            {
                *axis = axis.clone().show_labels(false);
                changed = true;
            }
            if changed {
                panel.invalidate();
            }
        }
    }

    /// Each panel uses the caller's ordinary chart widget and event mapping.
    /// `render` receives the panel index, as used by [`Self::panel_mut`].
    pub fn view_with<'a, M: 'a, T>(
        &'a self,
        mut render: impl FnMut(usize, &'a Data) -> Element<'a, M, T, crate::widget::Renderer>,
    ) -> Element<'a, M, T, crate::widget::Renderer>
    where
        T: crate::widget::text::Catalog + crate::widget::container::Catalog + 'a,
    {
        type Cell<'a, M, T> = Element<'a, M, T, crate::widget::Renderer>;
        let label_text = |label: &'a str| crate::widget::text::<T>(label).size(12);
        let strip = |label: &'a str| -> Cell<'a, M, T> {
            crate::widget::container(label_text(label))
                .center_x(Length::Fill)
                .padding([2, 4])
                .boxed()
        };
        let row_strip = |label: Option<&'a str>, width: f32| -> Cell<'a, M, T> {
            crate::widget::container(label_text(label.unwrap_or_default()))
                .width(width)
                .height(Length::Fill)
                .padding([0, 4])
                .align_y(alignment::Vertical::Center)
                .boxed()
        };
        // Row strips get one width so the columns line up with the
        // column strips above them.
        let row_width = match &self.strips {
            Strips::Margins { rows, .. } => {
                Some(rows.iter().map(|label| label.chars().count()).max().unwrap_or(0) as f32 * 7.0 + 12.0)
            }
            _ => None,
        };
        let mut column = Column::<Cell<'a, M, T>>::new().spacing(6);
        if let (Strips::Margins { columns, .. }, Some(width)) = (&self.strips, row_width) {
            let mut header = Row::<Cell<'a, M, T>>::new().spacing(6);
            for label in columns {
                header = header.push(strip(label));
            }
            column = column.push(header.push(crate::widget::space().width(width).boxed()).boxed());
        }
        for (row, cells) in self.slots.chunks(self.columns).enumerate() {
            let mut line = Row::<Cell<'a, M, T>>::new().spacing(6).height(Length::Fill);
            for slot in cells {
                let cell = match slot.and_then(|index| Some((index, self.panels.get(index)?))) {
                    Some((index, panel)) => match &self.strips {
                        Strips::Panels(labels) => Column::<Cell<'a, M, T>>::new()
                            .push(strip(labels.get(index).map_or("", String::as_str)))
                            .push(render(index, panel))
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .boxed(),
                        Strips::None | Strips::Margins { .. } => render(index, panel),
                    },
                    None => crate::widget::space().width(Length::Fill).boxed(),
                };
                line = line.push(cell);
            }
            for _ in cells.len()..self.columns {
                line = line.push(crate::widget::space().width(Length::Fill).boxed());
            }
            if let (Strips::Margins { rows, .. }, Some(width)) = (&self.strips, row_width) {
                line = line.push(row_strip(rows.get(row).map(String::as_str), width));
            }
            column = column.push(line.boxed());
        }
        column.width(Length::Fill).height(Length::Fill).boxed()
    }
}

/// Gives the `members` the pooled bounds of their primary data on the
/// chosen axes.
fn pool(panels: &mut [Data], members: &[usize], x: bool, y: bool) {
    if !x && !y {
        return;
    }
    let mut ranges: Option<crate::data::area::Bounds> = None;
    for panel in members
        .iter()
        .filter_map(|&i| panels.get(i))
        .filter(|p| !p.marks().is_empty() && (p.x_axis_ref().is_some() || p.y_axis_ref().is_some()))
    {
        let b = panel.primary_bounds();
        let pooled = ranges.get_or_insert(b);
        pooled.x_min = pooled.x_min.min(b.x_min);
        pooled.x_max = pooled.x_max.max(b.x_max);
        pooled.y_min = pooled.y_min.min(b.y_min);
        pooled.y_max = pooled.y_max.max(b.y_max);
    }
    let Some(b) = ranges else {
        return;
    };
    for &i in members {
        let Some(panel) = panels.get_mut(i) else {
            continue;
        };
        let x_linear = panel.x_scale_ref().nice() && panel.x_scale_ref().transform() == crate::scale::Transform::Linear;
        let y_linear = panel.y_scale().nice && panel.y_scale().transform == crate::scale::Transform::Linear;
        if x && let Some(axis) = panel.x_axis_mut() {
            *axis = shared(axis, b.x_min, b.x_max, x_linear);
        }
        if y && let Some(axis) = panel.y_axis_mut() {
            *axis = shared(axis, b.y_min, b.y_max, y_linear);
        }
        panel.invalidate();
    }
}

/// The axis with explicit bounds over the pooled range, rounded outward the
/// way the axis kind rounds an unshared range, so shared axes land on the
/// same nice ticks a lone chart would. Non-linear or un-niced scales keep
/// the raw range.
fn shared(axis: &crate::Axis, min: f64, max: f64, nice: bool) -> crate::Axis {
    let (lo, hi) = if nice {
        let bounds = axis.kind().bounds(min, max);
        (bounds.min(), bounds.max())
    } else {
        (min, max)
    };
    axis.clone().with_bounds(lo, hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pooling_only_replaces_shared_dimensions() {
        let make = || {
            vec![
                crate::data(crate::line([(1., 1.), (2., 3.)])),
                crate::data(crate::line([(5., 20.), (6., 30.)])),
            ]
        };
        let g = Grid::new(make(), 2, Scales::FreeY).expect("two facets");
        for panel in g.panels() {
            let x = panel.x_axis_ref().expect("x axis");
            // The pooled 1..6 rounds out the way a lone chart's axis would.
            let nice = x.kind().bounds(1., 6.);
            assert_eq!((x.lower_bound, x.upper_bound), (Some(nice.min()), Some(nice.max())));
            assert!(nice.min() <= 1. && nice.max() >= 6.);
            assert_eq!(panel.y_axis_ref().expect("y axis").upper_bound, None);
        }
        let fixed = Grid::new(make(), 1, Scales::Fixed).expect("fixed axes");
        let y = fixed.panels()[0].y_axis_ref().expect("y axis");
        assert_eq!(y.upper_bound, Some(y.kind().bounds(1., 30.).max()));
    }

    /// (region, month, sales)
    fn sales() -> Vec<(&'static str, &'static str, f64)> {
        vec![
            ("West", "Q1", 4.0),
            ("East", "Q1", 2.0),
            ("West", "Q2", 6.0),
            ("North", "Q2", 9.0),
            ("East", "Q2", 3.0),
            ("West", "Q3", 5.0),
        ]
    }

    fn panel(group: &Group<(&str, &str, f64)>) -> Data {
        let points: Vec<(f64, f64)> = group.rows().iter().enumerate().map(|(i, r)| (i as f64, r.2)).collect();
        crate::data(crate::line(points))
    }

    #[test]
    fn wrap_groups_by_first_appearance_and_keeps_row_order() {
        let facets = wrap(sales(), |r| r.0);
        let labels: Vec<_> = facets.groups().iter().map(Group::label).collect();
        assert_eq!(labels, ["West", "East", "North"]);
        let west: Vec<_> = facets.groups()[0].rows().iter().map(|r| r.1).collect();
        assert_eq!(west, ["Q1", "Q2", "Q3"]);
        assert_eq!(facets.groups()[0].row(), None);
        let sorted: Vec<_> = wrap(sales(), |r| r.0)
            .sorted()
            .groups()
            .iter()
            .map(|g| g.label().to_owned())
            .collect();
        assert_eq!(sorted, ["East", "North", "West"]);
    }

    #[test]
    fn wrap_strips_and_shared_axes_label_only_outer_panels() {
        let grid = wrap(sales(), |r| r.0).columns(2).chart(panel).expect("two columns");
        let Strips::Panels(strips) = &grid.strips else {
            panic!("wrap draws a strip per panel");
        };
        assert_eq!(strips, &["West", "East", "North"]);
        let labels = |i: usize| {
            let p = &grid.panels()[i];
            (
                p.x_axis_ref().expect("x").has_labels(),
                p.y_axis_ref().expect("y").has_labels(),
            )
        };
        // West, East on the top row; North alone on the bottom row.
        assert_eq!(labels(0), (false, true));
        assert_eq!(labels(1), (false, false));
        assert_eq!(labels(2), (true, true));
        // Fixed scales pool every panel: 2..9 rounds out to 10.
        for p in grid.panels() {
            assert_eq!(p.y_axis_ref().expect("y").upper_bound, Some(10.0));
        }
        let free = wrap(sales(), |r| r.0)
            .columns(2)
            .scales(Scales::Free)
            .chart(panel)
            .expect("free");
        assert!(free.panels().iter().all(|p| p.y_axis_ref().expect("y").has_labels()));
        assert!(wrap(sales(), |r| r.0).columns(0).chart(panel).is_none());
    }

    #[test]
    fn cross_orders_keys_leaves_empty_cells_and_pools_per_row() {
        let cross = grid(sales(), |r| r.0, |r| r.1);
        assert_eq!(cross.row_labels(), ["West", "East", "North"]);
        assert_eq!(cross.column_labels(), ["Q1", "Q2", "Q3"]);
        assert_eq!(cross.group(2, 0), None, "North has no Q1 rows");
        let cell = cross.group(1, 1).expect("East Q2");
        assert_eq!((cell.row(), cell.label()), (Some("East"), "Q2"));
        let sorted = grid(sales(), |r| r.0, |r| r.1).sorted();
        assert_eq!(sorted.row_labels(), ["East", "North", "West"]);
        assert_eq!(sorted.group(0, 1).expect("East Q2").rows()[0].2, 3.0);

        let g = cross.scales(Scales::FreeY).chart(panel).expect("matrix");
        assert_eq!(g.panels().len(), 6, "one panel per non-empty cell");
        assert_eq!(g.slots, [
            Some(0),
            Some(1),
            Some(2),
            Some(3),
            Some(4),
            None,
            None,
            Some(5),
            None
        ]);
        // y is free per row: West's 4..6 rounds out to 6.5, East's 2..3 to
        // 3.2 and North's lone 9 to 10.
        let top = |i: usize| g.panels()[i].y_axis_ref().expect("y").upper_bound;
        assert_eq!(
            (top(0), top(2), top(3), top(5)),
            (Some(6.5), Some(6.5), Some(3.2), Some(10.0))
        );
        // Only the first column keeps y labels, only the bottom row x labels.
        assert!(g.panels()[0].y_axis_ref().expect("y").has_labels());
        assert!(!g.panels()[1].y_axis_ref().expect("y").has_labels());
        assert!(!g.panels()[4].x_axis_ref().expect("x").has_labels());
        assert!(g.panels()[5].x_axis_ref().expect("x").has_labels());
    }
}

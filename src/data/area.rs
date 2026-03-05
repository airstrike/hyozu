use crate::data::{Axis, Mark};

/// Computed bounds for chart data.
#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

/// A plotting area containing marks and their associated axes.
#[derive(Debug, Clone, Default)]
pub struct Area {
    pub(crate) marks: Vec<Mark>,
    pub(crate) x_axis: Option<Axis>,
    pub(crate) y_axis: Option<Axis>,
}

impl Area {
    /// Create an empty area with no marks or axes.
    pub fn empty() -> Self {
        Self {
            marks: Vec::new(),
            x_axis: None,
            y_axis: None,
        }
    }

    /// Check if this area is empty (no marks).
    pub fn is_empty(&self) -> bool {
        self.marks.is_empty()
    }

    /// Get the marks in this area.
    pub fn marks(&self) -> &[Mark] {
        &self.marks
    }

    /// Returns a reference to a bars mark by index.
    ///
    /// Index refers to the nth Bars mark in this area.
    pub fn bars(&self, index: usize) -> Option<&crate::Bars> {
        self.marks
            .iter()
            .filter_map(|m| match m {
                Mark::Bars(b) => Some(b),
                _ => None,
            })
            .nth(index)
    }

    /// Returns a reference to a line mark by index.
    ///
    /// Index refers to the nth Line mark in this area.
    pub fn line(&self, index: usize) -> Option<&crate::Line> {
        self.marks
            .iter()
            .filter_map(|m| match m {
                Mark::Line(l) => Some(l),
                _ => None,
            })
            .nth(index)
    }

    /// Returns a mutable reference to a bars mark by index.
    pub fn bars_mut(&mut self, index: usize) -> Option<&mut crate::Bars> {
        self.marks
            .iter_mut()
            .filter_map(|m| match m {
                Mark::Bars(b) => Some(b),
                _ => None,
            })
            .nth(index)
    }

    /// Returns a reference to a pie mark by index.
    ///
    /// Index refers to the nth Pie mark in this area.
    pub fn pie(&self, index: usize) -> Option<&crate::Pie> {
        self.marks
            .iter()
            .filter_map(|m| match m {
                Mark::Pie(p) => Some(p),
                _ => None,
            })
            .nth(index)
    }

    /// Returns a mutable reference to a line mark by index.
    pub fn line_mut(&mut self, index: usize) -> Option<&mut crate::Line> {
        self.marks
            .iter_mut()
            .filter_map(|m| match m {
                Mark::Line(l) => Some(l),
                _ => None,
            })
            .nth(index)
    }

    /// Returns a mutable reference to the X axis.
    pub fn x_axis_mut(&mut self) -> Option<&mut Axis> {
        self.x_axis.as_mut()
    }

    /// Returns a mutable reference to the Y axis.
    pub fn y_axis_mut(&mut self) -> Option<&mut Axis> {
        self.y_axis.as_mut()
    }

    /// Configure the X axis with a closure.
    ///
    /// # Example
    /// ```ignore
    /// area.x_axis(|axis| axis.with_label_placement(Placement::BetweenTicks))
    /// ```
    pub fn x_axis(mut self, f: impl FnOnce(Axis) -> Axis) -> Self {
        if let Some(axis) = self.x_axis.take() {
            self.x_axis = Some(f(axis));
        }
        self
    }

    /// Configure the Y axis with a closure.
    ///
    /// # Example
    /// ```ignore
    /// area.y_axis(|axis| axis.with_label_format(|v| format!("${}", v)))
    /// ```
    pub fn y_axis(mut self, f: impl FnOnce(Axis) -> Axis) -> Self {
        if let Some(axis) = self.y_axis.take() {
            self.y_axis = Some(f(axis));
        }
        self
    }

    /// Computes the bounds of all marks in this area.
    pub fn bounds(&self) -> Bounds {
        use crate::bar::Layout;
        use crate::mark::area::Layout as AreaLayout;
        use std::collections::HashMap;

        let mut x_min = f64::INFINITY;
        let mut x_max = f64::NEG_INFINITY;
        let mut y_min = f64::INFINITY;
        let mut y_max = f64::NEG_INFINITY;

        for mark in &self.marks {
            match mark {
                Mark::Area(area_mark) => {
                    if area_mark.layout == AreaLayout::Stacked {
                        let mut sums: HashMap<i64, f64> = HashMap::new();
                        for series in &area_mark.series {
                            for point in &series.points {
                                let x_key = (point.x * 1000.0).round() as i64;
                                *sums.entry(x_key).or_insert(0.0) += point.y;
                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                            }
                        }
                        for sum in sums.values() {
                            y_min = y_min.min(*sum);
                            y_max = y_max.max(*sum);
                        }
                    } else {
                        for series in &area_mark.series {
                            for point in &series.points {
                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                                y_min = y_min.min(point.y);
                                y_max = y_max.max(point.y);
                            }
                        }
                    }
                }
                Mark::Bars(bars) => {
                    // For stacked layout, compute cumulative sums
                    if bars.layout == Layout::Stacked {
                        let mut sums: HashMap<i64, f64> = HashMap::new();

                        for series in &bars.series {
                            for point in &series.points {
                                let x_key = (point.x * 1000.0).round() as i64;
                                *sums.entry(x_key).or_insert(0.0) += point.y;

                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                            }
                        }

                        for sum in sums.values() {
                            y_min = y_min.min(*sum);
                            y_max = y_max.max(*sum);
                        }
                    } else {
                        // For grouped/overlaid, use individual values
                        for series in &bars.series {
                            for point in &series.points {
                                x_min = x_min.min(point.x);
                                x_max = x_max.max(point.x);
                                y_min = y_min.min(point.y);
                                y_max = y_max.max(point.y);
                            }
                        }
                    }
                }
                Mark::Line(line) => {
                    for point in &line.points {
                        x_min = x_min.min(point.x);
                        x_max = x_max.max(point.x);
                        y_min = y_min.min(point.y);
                        y_max = y_max.max(point.y);
                    }
                }
                Mark::Waterfall(wf) => {
                    let mut running: f64 = 0.0;
                    for (i, entry) in wf.entries.iter().enumerate() {
                        x_min = x_min.min(i as f64);
                        x_max = x_max.max(i as f64);
                        match entry.kind {
                            crate::mark::waterfall::EntryKind::Total => {
                                running = entry.value;
                            }
                            _ => {
                                running += entry.value;
                            }
                        }
                        y_min = y_min.min(running).min(0.0);
                        y_max = y_max.max(running);
                    }
                }
                Mark::Xy(xy) => {
                    for point in &xy.points {
                        x_min = x_min.min(point.x);
                        x_max = x_max.max(point.x);
                        y_min = y_min.min(point.y);
                        y_max = y_max.max(point.y);
                    }
                }
                Mark::Rule(rule) => match rule.orientation {
                    crate::mark::rule::RuleOrientation::Horizontal => {
                        y_min = y_min.min(rule.value);
                        y_max = y_max.max(rule.value);
                    }
                    crate::mark::rule::RuleOrientation::Vertical => {
                        x_min = x_min.min(rule.value);
                        x_max = x_max.max(rule.value);
                    }
                },
                // Pie and Gauge don't use Cartesian bounds
                Mark::Pie(_) | Mark::Gauge(_) => {}
            }
        }

        // Ensure we have valid bounds even for empty data
        if x_min.is_infinite() || x_max.is_infinite() {
            x_min = 0.0;
            x_max = 1.0;
        }
        if y_min.is_infinite() || y_max.is_infinite() {
            y_min = 0.0;
            y_max = 1.0;
        }

        // For bar/waterfall/area charts, extend y to include zero
        for mark in &self.marks {
            if matches!(
                mark,
                Mark::Area(_) | Mark::Bars(_) | Mark::Waterfall(_)
            ) {
                y_min = y_min.min(0.0);
                break;
            }
        }

        Bounds {
            x_min,
            x_max,
            y_min,
            y_max,
        }
    }
}

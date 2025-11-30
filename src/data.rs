pub mod area;
pub mod axis;
pub mod datum;
pub mod mark;

pub use area::Area;
pub use axis::{Axis, Orientation};
pub use datum::{Datum, IntoDatums};
pub use mark::{Bars, Line, Mark, bar, bars, line};

/// Trait for types that can be converted into chart Data.
///
/// This is used instead of `Into<Data>` to avoid coherence issues
/// with blanket implementations.
pub trait IntoData {
    fn into_data(self) -> Data;
}

/// Build chart data for hyozu
pub fn data(t: impl IntoData) -> Data {
    t.into_data()
}

/// Chart data containing one or more plotting areas.
///
/// This is the main type you store in your application state.
/// Data owns all the source data: series, axes, title, legend.
#[derive(Debug, Clone, Default)]
pub struct Data {
    /// Primary plotting area (bottom-left axes)
    pub(crate) primary: Area,

    /// Secondary plotting area (top-right axes)
    pub(crate) secondary: Area,

    /// Optional title
    pub(crate) title: Option<String>,
    // TODO: Add legend in future
}

impl From<Mark> for Area {
    fn from(mark: Mark) -> Self {
        // Configure axes based on mark type using the mark's factory methods
        let (x_axis, y_axis) = match &mark {
            Mark::Bars(_) => (Some(Bars::x_axis()), Some(Bars::y_axis())),
            Mark::Line(_) => (Some(Line::x_axis()), Some(Line::y_axis())),
        };

        Self {
            marks: vec![mark],
            x_axis,
            y_axis,
        }
    }
}

impl From<Vec<Mark>> for Area {
    fn from(marks: Vec<Mark>) -> Self {
        if marks.is_empty() {
            return Self::empty();
        }

        // Configure axes based on first mark type using mark factory methods
        let (x_axis, y_axis) = match marks.first() {
            Some(Mark::Bars(_)) => (Some(Bars::x_axis()), Some(Bars::y_axis())),
            Some(Mark::Line(_)) => (Some(Line::x_axis()), Some(Line::y_axis())),
            None => (None, None), // shouldn't happen
        };

        Self {
            marks,
            x_axis,
            y_axis,
        }
    }
}

impl IntoData for Mark {
    fn into_data(self) -> Data {
        Data {
            primary: Area::from(self),
            secondary: Area::empty(),
            title: None,
        }
    }
}

impl IntoData for Bars {
    fn into_data(self) -> Data {
        Mark::from(self).into_data()
    }
}

impl IntoData for Line {
    fn into_data(self) -> Data {
        Mark::from(self).into_data()
    }
}

impl IntoData for Vec<Mark> {
    fn into_data(self) -> Data {
        Data {
            primary: Area::from(self),
            secondary: Area::empty(),
            title: None,
        }
    }
}

// From impls for .into() ergonomics
impl From<Mark> for Data {
    fn from(mark: Mark) -> Self {
        mark.into_data()
    }
}

impl From<Vec<Mark>> for Data {
    fn from(marks: Vec<Mark>) -> Self {
        marks.into_data()
    }
}

impl Data {
    /// Sets the title for the chart.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Configure the X axis of the primary plotting area.
    ///
    /// # Example
    /// ```ignore
    /// Data::from(bars).x_axis(|axis| axis.with_label_placement(Placement::BetweenTicks))
    /// ```
    pub fn x_axis(mut self, f: impl FnOnce(Axis) -> Axis) -> Self {
        self.primary = self.primary.x_axis(f);
        self
    }

    /// Configure the Y axis of the primary plotting area.
    ///
    /// # Example
    /// ```ignore
    /// Data::from(bars).y_axis(|axis| axis.with_label_format(|v| format!("${}", v)))
    /// ```
    pub fn y_axis(mut self, f: impl FnOnce(Axis) -> Axis) -> Self {
        self.primary = self.primary.y_axis(f);
        self
    }

    /// Set labels for the X axis of the primary plotting area.
    ///
    /// This is a convenience method for setting axis labels without
    /// needing to use the full `.x_axis(|axis| ...)` closure pattern.
    ///
    /// # Examples
    /// ```ignore
    /// // Categorical labels
    /// Data::from(bars).x_axis_labels(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
    ///
    /// // Formatted numeric labels
    /// Data::from(line).x_axis_labels(|v| format!("{:.1}", v))
    /// ```
    pub fn x_axis_labels(mut self, labels: impl Into<axis::Labels>) -> Self {
        self.primary = self.primary.x_axis(|axis| axis.labels(labels));
        self
    }

    /// Set labels for the Y axis of the primary plotting area.
    ///
    /// This is a convenience method for setting axis labels without
    /// needing to use the full `.y_axis(|axis| ...)` closure pattern.
    ///
    /// # Examples
    /// ```ignore
    /// // Format values as currency
    /// Data::from(bars).y_axis_labels(|v| format!("${:.0}", v))
    ///
    /// // Format as percentages
    /// Data::from(line).y_axis_labels(|v| format!("{:.0}%", v * 100.0))
    /// ```
    pub fn y_axis_labels(mut self, labels: impl Into<axis::Labels>) -> Self {
        self.primary = self.primary.y_axis(|axis| axis.labels(labels));
        self
    }

    /// Sets explicit bounds for the x-axis.
    ///
    /// This is useful when you want to fix the x-axis range regardless of data,
    /// or when auto-scaling produces an undesirable range.
    ///
    /// Pass `None` to auto-scale that bound from data.
    ///
    /// # Examples
    /// ```ignore
    /// // Fix x-axis from 0 to 100
    /// Data::from(line).x_axis_bounds(0.0, 100.0)
    ///
    /// // Pin lower bound at 0, auto-scale upper
    /// Data::from(line).x_axis_bounds(0.0, None)
    /// ```
    pub fn x_axis_bounds(
        mut self,
        lower: impl Into<Option<f64>>,
        upper: impl Into<Option<f64>>,
    ) -> Self {
        self.primary = self.primary.x_axis(|axis| axis.with_bounds(lower, upper));
        self
    }

    /// Sets explicit bounds for the y-axis.
    ///
    /// This is useful when you want to fix the y-axis range regardless of data,
    /// or when auto-scaling produces an undesirable range.
    ///
    /// Pass `None` to auto-scale that bound from data.
    ///
    /// # Examples
    /// ```ignore
    /// // Fix y-axis from 0 to 100
    /// Data::from(line).y_axis_bounds(0.0, 100.0)
    ///
    /// // Pin lower bound at 0, auto-scale upper
    /// Data::from(line).y_axis_bounds(0.0, None)
    /// ```
    pub fn y_axis_bounds(
        mut self,
        lower: impl Into<Option<f64>>,
        upper: impl Into<Option<f64>>,
    ) -> Self {
        self.primary = self.primary.y_axis(|axis| axis.with_bounds(lower, upper));
        self
    }

    /// Returns a reference to the marks in the primary area.
    pub fn marks(&self) -> &[Mark] {
        self.primary.marks()
    }

    /// Computes the bounds of the primary area.
    pub fn primary_bounds(&self) -> area::Bounds {
        self.primary.bounds()
    }

    // === Property accessors ===

    /// Returns a reference to the primary plotting area.
    pub fn primary(&self) -> &Area {
        &self.primary
    }

    /// Returns a reference to the title.
    pub fn get_title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns a reference to a bars mark by index.
    pub fn bars(&self, index: usize) -> Option<&Bars> {
        self.primary.bars(index)
    }

    /// Returns a reference to a line mark by index.
    pub fn line(&self, index: usize) -> Option<&Line> {
        self.primary.line(index)
    }

    /// Returns a reference to the X axis.
    pub fn x_axis_ref(&self) -> Option<&Axis> {
        self.primary.x_axis.as_ref()
    }

    /// Returns a reference to the Y axis.
    pub fn y_axis_ref(&self) -> Option<&Axis> {
        self.primary.y_axis.as_ref()
    }

    /// Returns a mutable reference to a bars mark by index.
    pub fn bars_mut(&mut self, index: usize) -> Option<&mut Bars> {
        self.primary.bars_mut(index)
    }

    /// Returns a mutable reference to a line mark by index.
    pub fn line_mut(&mut self, index: usize) -> Option<&mut Line> {
        self.primary.line_mut(index)
    }

    /// Returns a mutable reference to the X axis.
    pub fn x_axis_mut(&mut self) -> Option<&mut Axis> {
        self.primary.x_axis_mut()
    }

    /// Returns a mutable reference to the Y axis.
    pub fn y_axis_mut(&mut self) -> Option<&mut Axis> {
        self.primary.y_axis_mut()
    }
}

/// Actions that can be performed on chart data.
///
/// This follows the iced text_editor pattern.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Set a property on a chart item.
    Set(crate::item::Item),
}

impl Data {
    /// Performs an action on the data.
    ///
    /// Updates the data based on user interactions.
    pub fn perform(&mut self, action: Action) {
        use crate::item::Item;

        match action {
            Action::Set(item) => match item {
                Item::Title(title) => {
                    self.title = Some(title);
                }
                Item::Bars(index, property) => {
                    if let Some(Mark::Bars(bars)) =
                        self.primary.marks.get_mut(index)
                    {
                        property.apply(bars);
                    }
                }
                Item::Line(index, property) => {
                    if let Some(Mark::Line(line)) =
                        self.primary.marks.get_mut(index)
                    {
                        property.apply(line);
                    }
                }
                Item::XAxis(property) => {
                    if let Some(axis) = &mut self.primary.x_axis {
                        property.apply(axis);
                    }
                }
                Item::YAxis(property) => {
                    if let Some(axis) = &mut self.primary.y_axis {
                        property.apply(axis);
                    }
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bars;

    #[test]
    fn test_data_from_mark() {
        let mark = bars([100, 200, 300]);
        let data = mark.into_data();
        assert_eq!(data.primary.marks.len(), 1);
        assert!(data.title.is_none());
    }

    #[test]
    fn test_data_from_vec() {
        let mark1 = Mark::Bars(bars([100, 200]));
        let mark2 = Mark::Bars(bars([300, 400]));
        let data = vec![mark1, mark2].into_data();
        assert_eq!(data.primary.marks.len(), 2);
    }

    #[test]
    fn test_data_title() {
        let data = bars([100, 200]).into_data().title("Sales Data");
        assert_eq!(data.title, Some("Sales Data".to_string()));
    }

    #[test]
    fn test_data_title_chainable() {
        let data =
            vec![Mark::Bars(bars([100, 200])), Mark::Bars(bars([300, 400]))]
                .into_data()
                .title("Multi-Series Chart");

        assert_eq!(data.primary.marks.len(), 2);
        assert_eq!(data.title, Some("Multi-Series Chart".to_string()));
    }

    #[test]
    fn test_action_perform() {
        // TODO
    }
}

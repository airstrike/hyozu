use crate::color::Color;

pub mod label;

pub use label::Label;

/// A single slice in a pie/donut chart.
#[derive(Debug, Clone)]
pub struct Slice {
    pub(crate) value: f64,
    pub(crate) color: Option<Color>,
    pub(crate) label: Option<Label>,
    /// Optional name for this slice (used in legends).
    pub(crate) name: Option<String>,
}

impl Slice {
    /// Sets the color for this slice.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Sets the label for this slice.
    pub fn label(mut self, label: impl Into<Label>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the name for this slice (used in legends).
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Sets the label position for this slice (no-op when no label is set).
    pub fn set_label_position(&mut self, position: label::Position) {
        if let Some(label) = &mut self.label {
            label.set_position(position);
        }
    }

    /// Sets which slices show labels (no-op when no label is set).
    pub fn set_label_show(&mut self, show: label::Show) {
        if let Some(label) = &mut self.label {
            label.set_show(show);
        }
    }

    /// Sets the label color for this slice.
    pub fn set_label_color(&mut self, color: Option<Color>) {
        if let Some(label) = &mut self.label {
            label.set_color(color);
        }
    }

    /// Sets the label size for this slice.
    pub fn set_label_size(&mut self, size: Option<crate::core::Pixels>) {
        if let Some(label) = &mut self.label {
            label.set_size(size);
        }
    }

    /// Sets the label font weight for this slice.
    pub fn set_label_weight(&mut self, weight: Option<crate::core::font::Weight>) {
        if let Some(label) = &mut self.label {
            label.set_weight(weight);
        }
    }

    /// Sets the label font style for this slice.
    pub fn set_label_style(&mut self, style: Option<crate::core::font::Style>) {
        if let Some(label) = &mut self.label {
            label.set_style(style);
        }
    }

    /// Sets the label background fill for this slice.
    pub fn set_label_fill(&mut self, fill: Option<Color>) {
        if let Some(label) = &mut self.label {
            label.set_fill(fill);
        }
    }

    /// Returns the value of this slice.
    pub fn value(&self) -> f64 {
        self.value
    }

    /// Returns the color of this slice, if set.
    pub fn get_color(&self) -> Option<&Color> {
        self.color.as_ref()
    }

    /// Returns the name of this slice.
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns a reference to the label configuration.
    pub fn get_label(&self) -> Option<&Label> {
        self.label.as_ref()
    }
}

/// Creates a single slice with a value.
pub fn slice(value: impl Into<f64>) -> Slice {
    Slice {
        value: value.into(),
        color: None,
        label: None,
        name: None,
    }
}

/// Pie/donut chart specification.
///
/// A donut is a pie with a hole. Set `hole` to 0.0 for a full pie,
/// or to a value in `0.0..1.0` to create a donut (proportion of radius).
#[derive(Debug, Clone)]
pub struct Pie {
    pub(crate) slices: Vec<Slice>,
    /// Inner hole radius as proportion of outer radius (0.0 = pie, 0.0..1.0 = donut)
    pub(crate) hole: f32,
    /// Gap between slices in pixels.
    pub(crate) gap: f32,
}

/// Creates a pie chart from slice values.
///
/// # Examples
///
/// ```
/// use hyozu::pie;
///
/// // Simple pie from values
/// let chart = pie([30, 50, 20]);
///
/// // Donut chart
/// let chart = pie([30, 50, 20]).hole(0.6);
/// ```
pub fn pie(data: impl IntoPie) -> Pie {
    data.into_pie()
}

/// Trait for converting various inputs into a Pie.
pub trait IntoPie {
    fn into_pie(self) -> Pie;
}

// From an array/vec of numeric values
impl<T, const N: usize> IntoPie for [T; N]
where
    T: Into<f64> + Copy,
{
    fn into_pie(self) -> Pie {
        Pie {
            slices: self
                .iter()
                .map(|&v| Slice {
                    value: v.into(),
                    color: None,
                    label: None,
                    name: None,
                })
                .collect(),
            hole: 0.0,
            gap: 0.0,
        }
    }
}

// From a Vec of numeric values
impl<T> IntoPie for Vec<T>
where
    T: Into<f64> + Copy,
{
    fn into_pie(self) -> Pie {
        Pie {
            slices: self
                .iter()
                .map(|v| Slice {
                    value: (*v).into(),
                    color: None,
                    label: None,
                    name: None,
                })
                .collect(),
            hole: 0.0,
            gap: 0.0,
        }
    }
}

// From an array of Slices
impl<const N: usize> IntoPie for [Slice; N] {
    fn into_pie(self) -> Pie {
        Pie {
            slices: self.into(),
            hole: 0.0,
            gap: 0.0,
        }
    }
}

// From a Vec of Slices
impl IntoPie for Vec<Slice> {
    fn into_pie(self) -> Pie {
        Pie {
            slices: self,
            hole: 0.0,
            gap: 0.0,
        }
    }
}

impl Pie {
    /// Sets the inner hole radius as a proportion of the outer radius.
    ///
    /// - `0.0` = full pie (no hole)
    /// - `0.6` = donut with 60% hole
    /// - Values are clamped to `0.0..1.0`
    pub fn hole(mut self, hole: f32) -> Self {
        self.hole = hole.clamp(0.0, 0.99);
        self
    }

    /// Returns the slices.
    pub fn slices(&self) -> &[Slice] {
        &self.slices
    }

    /// Returns a mutable reference to the slices.
    pub fn slices_mut(&mut self) -> &mut Vec<Slice> {
        &mut self.slices
    }

    /// Returns the hole proportion.
    pub fn hole_value(&self) -> f32 {
        self.hole
    }

    /// Sets the gap between slices in pixels.
    pub fn gap(mut self, pixels: f32) -> Self {
        self.gap = pixels.max(0.0);
        self
    }

    /// Applies a label configuration to all slices that don't already have one.
    pub fn labels(mut self, label: impl Into<label::Label>) -> Self {
        let label = label.into();
        for slice in &mut self.slices {
            if slice.label.is_none() {
                slice.label = Some(label.clone());
            }
        }
        self
    }

    // === Chart-wide label setters (only affect slices that already have a label) ===

    pub fn set_label_position(&mut self, position: label::Position) {
        for slice in &mut self.slices {
            slice.set_label_position(position);
        }
    }

    pub fn set_label_show(&mut self, show: label::Show) {
        for slice in &mut self.slices {
            slice.set_label_show(show);
        }
    }

    pub fn set_label_color(&mut self, color: Option<Color>) {
        for slice in &mut self.slices {
            slice.set_label_color(color);
        }
    }

    pub fn set_label_size(&mut self, size: Option<crate::core::Pixels>) {
        for slice in &mut self.slices {
            slice.set_label_size(size);
        }
    }

    pub fn set_label_weight(&mut self, weight: Option<crate::core::font::Weight>) {
        for slice in &mut self.slices {
            slice.set_label_weight(weight);
        }
    }

    pub fn set_label_style(&mut self, style: Option<crate::core::font::Style>) {
        for slice in &mut self.slices {
            slice.set_label_style(style);
        }
    }

    pub fn set_label_fill(&mut self, fill: Option<Color>) {
        for slice in &mut self.slices {
            slice.set_label_fill(fill);
        }
    }

    /// Pie charts have no axes.
    pub fn x_axis() -> Option<crate::data::Axis> {
        None
    }

    /// Pie charts have no axes.
    pub fn y_axis() -> Option<crate::data::Axis> {
        None
    }
}

impl From<Pie> for crate::Data {
    fn from(pie: Pie) -> Self {
        use crate::data::IntoData;
        pie.into_data()
    }
}

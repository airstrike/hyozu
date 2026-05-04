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

/// Pie chart specification.
///
/// `pie([...])` is the data layer — just the slices and presentation
/// concerns shared with donuts (gaps, labels). For a donut, pass the
/// `Data` to `donut(&data)` and configure the hole radius there.
///
/// Per-mark `value_scale` lets one pie format its slice values
/// differently from the rest of the chart. The format chain reads:
/// guide override (e.g. `legend::Config::value_format`) → this field →
/// `Data::value_scale` → built-in default. See [`crate::scale`] for the
/// full chain.
#[derive(Debug, Clone)]
pub struct Pie {
    pub(crate) slices: Vec<Slice>,
    /// Gap between slices in pixels.
    pub(crate) gap: f32,
    /// Optional per-mark value-format override. `None` falls through to
    /// the data-level [`Data::value_scale`].
    pub(crate) value_scale: Option<crate::scale::Scale<f64>>,
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
            gap: 0.0,
            value_scale: None,
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
            gap: 0.0,
            value_scale: None,
        }
    }
}

// From an array of Slices
impl<const N: usize> IntoPie for [Slice; N] {
    fn into_pie(self) -> Pie {
        Pie {
            slices: self.into(),
            gap: 0.0,
            value_scale: None,
        }
    }
}

// From a Vec of Slices
impl IntoPie for Vec<Slice> {
    fn into_pie(self) -> Pie {
        Pie {
            slices: self,
            gap: 0.0,
            value_scale: None,
        }
    }
}

impl Pie {
    /// Returns the slices.
    pub fn slices(&self) -> &[Slice] {
        &self.slices
    }

    /// Returns a mutable reference to the slices.
    pub fn slices_mut(&mut self) -> &mut Vec<Slice> {
        &mut self.slices
    }

    /// Sets the gap between slices in pixels.
    pub fn gap(mut self, pixels: f32) -> Self {
        self.gap = pixels.max(0.0);
        self
    }

    /// Sets a per-mark format closure for slice values.
    ///
    /// Overrides [`crate::Data::value_scale`] for this pie only. Lower
    /// priority than [`crate::data::legend::Config::value_format`]
    /// (legend-only override) and per-row [`crate::Tooltip::format`]
    /// (tooltip-only override).
    ///
    /// ```ignore
    /// use hyozu::pie;
    /// pie([86.2, 40.5, 12.8])
    ///     .value_format(|v: &f64| format!("${v:.1}M"));
    /// ```
    pub fn value_format(mut self, f: impl Fn(&f64) -> String + Send + Sync + 'static) -> Self {
        self.value_scale = Some(crate::scale::Scale::new().format(f));
        self
    }

    /// Returns the per-mark value-scale override, if any.
    pub fn value_scale(&self) -> Option<&crate::scale::Scale<f64>> {
        self.value_scale.as_ref()
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

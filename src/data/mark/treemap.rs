use crate::color::Color;

/// A single item in a treemap chart.
#[derive(Debug, Clone)]
pub struct Item {
    pub(crate) label: String,
    pub(crate) value: f32,
    pub(crate) color: Option<Color>,
}

impl Item {
    /// Sets the color for this item.
    pub fn color(mut self, color: impl Into<Color>) -> Self {
        self.color = Some(color.into());
        self
    }

    /// Returns the label of this item.
    pub fn get_label(&self) -> &str {
        &self.label
    }

    /// Returns the value of this item.
    pub fn value(&self) -> f32 {
        self.value
    }

    /// Returns the color of this item, if set.
    pub fn get_color(&self) -> Option<&Color> {
        self.color.as_ref()
    }
}

/// Creates a single treemap item with a label and value.
pub fn item(label: impl Into<String>, value: impl Into<f32>) -> Item {
    Item {
        label: label.into(),
        value: value.into(),
        color: None,
    }
}

/// Treemap chart specification.
///
/// Displays hierarchical data as nested rectangles, where each rectangle's
/// area is proportional to its value. Uses the squarified treemap algorithm
/// for good aspect ratios.
#[derive(Debug, Clone)]
pub struct Treemap {
    pub(crate) items: Vec<Item>,
    /// Gap between rectangles in pixels.
    pub(crate) gap: f32,
}

/// Creates a treemap chart from items.
///
/// # Examples
///
/// ```
/// use hyozu::treemap;
/// use hyozu::mark::treemap::item;
///
/// // Simple treemap from items
/// let chart = treemap([
///     item("Sales", 100.0),
///     item("Marketing", 60.0),
///     item("Engineering", 80.0),
/// ]);
///
/// // With custom colors
/// use hyozu::Color;
/// let chart = treemap([
///     item("A", 50.0).color(Color::from_rgb8(0x1B, 0x3A, 0x5C)),
///     item("B", 30.0).color(Color::from_rgb8(0x2C, 0x5F, 0x8A)),
/// ]);
/// ```
pub fn treemap(data: impl IntoTreemap) -> Treemap {
    data.into_treemap()
}

/// Trait for converting various inputs into a Treemap.
pub trait IntoTreemap {
    fn into_treemap(self) -> Treemap;
}

// From an array of Items
impl<const N: usize> IntoTreemap for [Item; N] {
    fn into_treemap(self) -> Treemap {
        Treemap {
            items: self.into(),
            gap: 2.0,
        }
    }
}

// From a Vec of Items
impl IntoTreemap for Vec<Item> {
    fn into_treemap(self) -> Treemap {
        Treemap { items: self, gap: 2.0 }
    }
}

// From an array of (label, value) tuples
impl<S, V, const N: usize> IntoTreemap for [(S, V); N]
where
    S: Into<String>,
    V: Into<f32>,
{
    fn into_treemap(self) -> Treemap {
        Treemap {
            items: self
                .into_iter()
                .map(|(label, value)| Item {
                    label: label.into(),
                    value: value.into(),
                    color: None,
                })
                .collect(),
            gap: 2.0,
        }
    }
}

// From a Vec of (label, value) tuples
impl<S, V> IntoTreemap for Vec<(S, V)>
where
    S: Into<String>,
    V: Into<f32>,
{
    fn into_treemap(self) -> Treemap {
        Treemap {
            items: self
                .into_iter()
                .map(|(label, value)| Item {
                    label: label.into(),
                    value: value.into(),
                    color: None,
                })
                .collect(),
            gap: 2.0,
        }
    }
}

impl Treemap {
    /// Sets the gap between rectangles in pixels.
    pub fn gap(mut self, pixels: f32) -> Self {
        self.gap = pixels.max(0.0);
        self
    }

    /// Returns the items.
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    /// Returns a mutable reference to the items.
    pub fn items_mut(&mut self) -> &mut Vec<Item> {
        &mut self.items
    }

    /// Treemap charts have no axes.
    pub fn x_axis() -> Option<crate::data::Axis> {
        None
    }

    /// Treemap charts have no axes.
    pub fn y_axis() -> Option<crate::data::Axis> {
        None
    }
}

impl From<Treemap> for crate::Data {
    fn from(treemap: Treemap) -> Self {
        use crate::data::IntoData;
        treemap.into_data()
    }
}

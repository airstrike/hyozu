use crate::data::mark::area;
use crate::map::Map;

pub mod series {
    use crate::color::Color;
    use crate::map::Map;

    #[derive(Debug, Clone, PartialEq)]
    pub enum Property {
        Color(Option<Color>),
        Opacity(f32),
        Stroke(Option<f32>),
    }

    impl Map for Property {}

    impl Property {
        pub fn apply(&self, series: &mut super::area::Series) {
            match self {
                Property::Color(c) => series.color = *c,
                Property::Opacity(o) => series.opacity = *o,
                Property::Stroke(s) => series.stroke = *s,
            }
        }
    }

    #[allow(non_snake_case)]
    pub fn Color(value: Option<Color>) -> Property {
        Property::Color(value)
    }

    #[allow(non_snake_case)]
    pub fn Opacity(value: f32) -> Property {
        Property::Opacity(value)
    }

    #[allow(non_snake_case)]
    pub fn Stroke(value: Option<f32>) -> Property {
        Property::Stroke(value)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Property {
    Layout(area::Layout),
    Series {
        index: usize,
        property: series::Property,
    },
}

impl Map for Property {}

impl Property {
    pub fn apply(&self, area: &mut area::Area) {
        match self {
            Property::Layout(l) => area.layout = *l,
            Property::Series { index, property } => {
                if let Some(s) = area.series_mut().get_mut(*index) {
                    property.apply(s);
                }
            }
        }
    }
}

pub use Property::Layout;

#[allow(non_snake_case)]
pub fn Series(index: usize) -> impl Fn(series::Property) -> Property {
    move |property| Property::Series { index, property }
}

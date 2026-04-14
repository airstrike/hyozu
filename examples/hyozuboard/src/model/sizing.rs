use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct Sizing {
    pub default: Name,
    pub variants: BTreeMap<Name, Variant>,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Name {
    Small,
    Large,
    Wide,
    Tall,
    Full,
}

#[derive(Deserialize)]
pub struct Variant {
    pub grid: [u16; 2],
    pub show: Vec<Show>,
    #[serde(default)]
    pub max_rows: Option<usize>,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Show {
    Value,
    Delta,
    Chart,
    Rows,
    Headers,
}

impl Sizing {
    /// Returns the default variant.
    pub fn default_variant(&self) -> &Variant {
        &self.variants[&self.default]
    }

    /// Find best-fit variant: exact grid match, or fallback to default.
    pub fn resolve(&self, w: u16, h: u16) -> (&Name, &Variant) {
        self.variants
            .iter()
            .find(|(_, v)| v.grid == [w, h])
            .unwrap_or_else(|| (&self.default, &self.variants[&self.default]))
    }
}

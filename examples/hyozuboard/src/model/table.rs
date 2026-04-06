use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Deserialize)]
pub struct Body {
    pub columns: Vec<Column>,
    pub data: Vec<BTreeMap<String, serde_json::Value>>,
    #[serde(default)]
    pub on_row_action: Option<RowAction>,
}

#[derive(Deserialize)]
pub struct Column {
    pub key: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(rename = "type")]
    pub kind: Kind,
}

#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Text,
    Tag,
    Number,
    Timestamp,
    Url,
}

#[derive(Deserialize)]
pub struct RowAction {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub url_column: Option<String>,
}

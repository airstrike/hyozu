use serde::Deserialize;

#[derive(Deserialize)]
pub struct Footer {
    pub author: Author,
    #[serde(default)]
    pub data_source: Option<Source>,
    pub updated: String,
}

#[derive(Deserialize)]
pub struct Author {
    pub user_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct Source {
    pub system: String,
    #[serde(default)]
    pub agent: Option<String>,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigInitResult {
    pub ok: bool,
    pub path: String,
    pub format: String,
    pub builder: String,
    pub source_sets: Vec<ConfigInitSourceSet>,
    pub overwritten: bool,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigInitSourceSet {
    pub name: String,
    #[serde(rename = "type")]
    pub source_type: String,
    pub path: String,
}

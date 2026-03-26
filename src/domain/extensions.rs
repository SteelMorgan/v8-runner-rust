use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionsResult {
    pub ok: bool,
    pub steps: Vec<ExtensionsStep>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionsStep {
    pub target: String,
    pub action: String,
    pub ok: bool,
    pub message: Option<String>,
    pub duration_ms: u64,
}

pub mod configuration;
pub mod holder;
pub mod intermediary;
pub mod loader;
pub mod rule;
pub mod ruleset;
pub mod types;

pub use types::*;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Match {
    pub uri: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Plugin {
    pub name: String,
    pub with: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyResponseFips {
    #[serde(rename = "setHeaders")]
    pub set_headers: Option<HashMap<String, String>>,
    #[serde(rename = "deleteHeaders")]
    pub delete_headers: Option<Vec<String>>,
    pub body: Option<Vec<BodyManipulation>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyResponseProxy {
    #[serde(rename = "setHeaders")]
    pub add_headers: Option<HashMap<String, String>>,
    #[serde(rename = "keepHeaders")]
    pub delete_headers: Option<Vec<String>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BodyManipulation {
    pub at: String,
    pub with: Value,
}

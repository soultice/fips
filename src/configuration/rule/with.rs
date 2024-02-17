use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::configuration::configuration::Plugin;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct With {
    pub sleep: Option<u64>,
    pub probability: Option<f32>,
    pub plugins: Option<Vec<Plugin>>,
}


use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::configuration::types::Plugin;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct With {
    pub plugins: Vec<Plugin>,
}


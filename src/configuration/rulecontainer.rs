use serde::{Deserialize, Serialize};
use serde_json::Value;
use schemars::JsonSchema;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct RuleContainer {
    pub path: Option<String>,
    pub item: Option<Value>,
}
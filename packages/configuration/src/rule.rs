use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Rule {
    pub path: Option<String>,
    pub item: Option<Value>,
}

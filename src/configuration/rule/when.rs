use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use super::super::configuration::Match;


#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct When {
    #[serde(rename = "matchesUris")]
    pub matches: Vec<Match>,
    #[serde(rename = "matchesMethods")]
    pub matches_methods: Option<Vec<String>>,
    #[serde(rename = "bodyContains")]
    pub body_contains: Option<String>,
}


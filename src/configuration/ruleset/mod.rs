use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use super::rule::Rule;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RuleSet {
    Rule(Rule),
}

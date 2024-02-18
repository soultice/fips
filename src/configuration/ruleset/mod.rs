use schemars::JsonSchema;
use serde::{Serialize, Deserialize};
use crate::configuration::rule::Rule;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum RuleSet {
    Rule(Rule),
}

impl RuleSet {
    pub fn into_inner(&self) -> &Rule {
        match self {
            RuleSet::Rule(rule) => rule,
        }
    }
}


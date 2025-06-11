use eyre::Result;
use super::rule::Rule;
use super::intermediary::Intermediary;
use super::rule::error::ConfigurationError;
use crate::plugin_registry::ExternalFunctions;

#[derive(Debug, Default)]
pub struct RuleContainer {
    pub rules: Vec<Rule>,
}

impl RuleContainer {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
        }
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    pub fn get_rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn get_rules_mut(&mut self) -> &mut [Rule] {
        &mut self.rules
    }

    pub async fn apply(&mut self, intermediary: &mut Intermediary, functions: &ExternalFunctions) -> Result<(), ConfigurationError> {
        for rule in &self.rules {
            if rule.apply_when(intermediary) {
                rule.apply_then(intermediary, functions).await?;
            }
        }
        Ok(())
    }
}

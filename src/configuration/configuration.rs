use eyre::Result;
use hyper::http::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::rule::Rule;
use super::intermediary::Intermediary;
use super::holder::RuleContainer;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Config {
    pub active_rule_indices: Vec<usize>,
    pub fe_selected_rule: Option<i32>,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("No matching rule found")]
    NoMatchingRule,
    #[error("Rule evaluation failed: {0}")]
    RuleEvaluation(String),
    #[error("Failed to load config: {0}")]
    LoadError(String),
}

impl Config {
    pub fn new() -> Self {
        Self {
            active_rule_indices: Vec::new(),
            fe_selected_rule: None,
            rules: Vec::new(),
        }
    }

    pub fn reload(&mut self, _path: &str) -> Result<(), ConfigError> {
        // Load configuration from the specified path
        // This is just a placeholder - implement actual loading logic
        Ok(())
    }

    pub async fn check_rule(&self, _intermediary: &Intermediary) -> Result<RuleContainer, ConfigError> {
        let mut container = RuleContainer::new();

        // Only process active rules
        for idx in &self.active_rule_indices {
            if let Some(rule) = self.rules.get(*idx) {
                container.add_rule(rule.clone());
            }
        }

        // Return early if no rules matched
        if container.get_rules().is_empty() {
            return Err(ConfigError::NoMatchingRule);
        }

        Ok(container)
    }

    pub fn toggle_rule(&mut self) -> bool {
        if let Some(idx) = self.fe_selected_rule.and_then(|i| Some(i as usize)) {
            if self.active_rule_indices.contains(&idx) {
                self.active_rule_indices.retain(|&x| x != idx);
                false
            } else {
                self.active_rule_indices.push(idx);
                true
            }
        } else {
            false
        }
    }

    pub fn select_next(&mut self) {
        if let Some(current) = self.fe_selected_rule {
            if (current as usize) < self.rules.len() - 1 {
                self.fe_selected_rule = Some(current + 1);
            }
        } else if !self.rules.is_empty() {
            self.fe_selected_rule = Some(0);
        }
    }

    pub fn select_previous(&mut self) {
        if let Some(current) = self.fe_selected_rule {
            if current > 0 {
                self.fe_selected_rule = Some(current - 1);
            }
        } else if !self.rules.is_empty() {
            self.fe_selected_rule = Some((self.rules.len() - 1) as i32);
        }
    }
}

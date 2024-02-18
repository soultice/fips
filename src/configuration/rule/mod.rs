pub mod error;
pub mod then;
pub mod when;
pub mod with;

use error::ConfigurationError;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use eyre::{ContextCompat, Result};
use regex::RegexSet;
use rand::Rng;

use super::rule::then::Then;
use super::rule::when::When;
use super::rule::with::With;
use super::intermediary::Intermediary;

use crate::plugin_registry::ExternalFunctions;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Rule {
    pub name: String,
    pub when: When,
    pub then: Then,
    pub with: Option<With>,
    #[serde(skip)]
    pub path: String,
    #[serde(skip)]
    pub plugins: Option<ExternalFunctions>,
}

impl Rule {
    pub fn should_apply(&self, intermediary: &Intermediary) -> Result<()> {
        let mut rng = rand::thread_rng();

        let uri_regex = RegexSet::new(
            self.when
                .matches
                .iter()
                .map(|m| m.uri.as_str())
                .collect::<Vec<&str>>(),
        )?;

        let uri = intermediary
            .clone()
            .uri
            .wrap_err("could not retrieve uri")?;

        let some_uris_match = uri_regex.is_match(uri.path());
        if !some_uris_match {
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }

        let some_methods_match =
            self.when.matches_methods.as_ref().map_or(true, |methods| {
                methods
                    .iter()
                    .any(|m| m == intermediary.clone().method.unwrap().as_str())
            });

        if !some_methods_match {
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }

        let some_body_contains =
            self.when
                .body_contains
                .as_ref()
                .map_or(true, |body_contains| {
                    intermediary.body.as_str().unwrap().contains(body_contains)
                });

        if !some_body_contains {
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }

        let probability_matches = self
            .with
            .as_ref()
            .unwrap_or(&With {
                probability: Some(1.0),
                plugins: None,
                sleep: None,
            })
            .probability
            .map(|probability| {
                let random_number = rng.gen_range(0.0, 0.99);
                random_number < probability
            })
            .unwrap_or(true);

        if !probability_matches {
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }

        Ok(())
    }
}

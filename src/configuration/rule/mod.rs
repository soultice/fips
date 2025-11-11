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

        #[cfg(feature = "logging")]
        log::debug!("[rule::should_apply] start rule='{}'", self.name); // correlation id logged at call site

        // Build URI regex set from rule
        let regex_vec: Vec<&str> = self
            .when
            .matches
            .iter()
            .map(|m| m.uri.as_str())
            .collect();
        let uri_regex = RegexSet::new(regex_vec.clone())?;
        #[cfg(feature = "logging")]
        log::trace!("[rule::should_apply] uri regexes={:?}", regex_vec);

        let uri = intermediary
            .uri
            .as_ref()
            .wrap_err("could not retrieve uri")?;
        let path = uri.path();
        #[cfg(feature = "logging")]
        log::trace!("[rule::should_apply] request.path='{}'", path);

        let some_uris_match = uri_regex.is_match(path);
        if !some_uris_match {
            #[cfg(feature = "logging")]
            log::debug!("[rule::should_apply] NO MATCH uri path='{}' rule='{}'", path, self.name);
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        } else {
            #[cfg(feature = "logging")]
            log::debug!("[rule::should_apply] MATCH uri path='{}' rule='{}'", path, self.name);
        }

        // Method matching
        let some_methods_match = self
            .when
            .matches_methods
            .as_ref()
            .is_none_or(|methods| {
                let result = methods.iter().any(|m| {
                    intermediary
                        .method
                        .as_ref()
                        .is_some_and(|method| m == method.as_str())
                });
                #[cfg(feature = "logging")]
                {
                    let incoming = intermediary.method.as_ref().map(|m| m.to_string()).unwrap_or("<none>".to_string());
                    log::trace!("[rule::should_apply] method check incoming='{}' allowed={:?} result={}" , incoming, methods, result);
                }
                result
            });
        if !some_methods_match {
            #[cfg(feature = "logging")]
            log::debug!("[rule::should_apply] NO MATCH method rule='{}'", self.name);
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }
        #[cfg(feature = "logging")]
        log::debug!("[rule::should_apply] MATCH method rule='{}'", self.name);

        // Body contains check
        let some_body_contains = self
            .when
            .body_contains
            .as_ref()
            .is_none_or(|body_contains| {
                let body_str = intermediary.body.to_string();
                let contains = body_str.contains(body_contains);
                #[cfg(feature = "logging")]
                log::trace!("[rule::should_apply] body_contains pattern='{}' contains={}", body_contains, contains);
                contains
            });
        if !some_body_contains {
            #[cfg(feature = "logging")]
            log::debug!("[rule::should_apply] NO MATCH body rule='{}'", self.name);
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }
        #[cfg(feature = "logging")]
        log::debug!("[rule::should_apply] MATCH body rule='{}'", self.name);

        // Probability gate
        let probability_cfg = self.with.as_ref().unwrap_or(&With { probability: Some(1.0), plugins: None, sleep: None }).probability;
        let probability_matches = probability_cfg.map(|probability| {
            let random_number = rng.gen_range(0.0, 0.99);
            let pass = random_number < probability;
            #[cfg(feature = "logging")]
            log::trace!("[rule::should_apply] probability random_number={} threshold={} pass={}", random_number, probability, pass);
            pass
        }).unwrap_or(true);
        if !probability_matches {
            #[cfg(feature = "logging")]
            log::debug!("[rule::should_apply] NO MATCH probability rule='{}'", self.name);
            return Err(ConfigurationError::RuleDoesNotMatch.into());
        }
        #[cfg(feature = "logging")]
        log::debug!("[rule::should_apply] MATCH probability rule='{}'", self.name);

        #[cfg(feature = "logging")]
        log::debug!("[rule::should_apply] ALL MATCH rule='{}' -> APPLY", self.name);
        Ok(())
    }
}

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use schemars::JsonSchema;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Match {
    pub uri: String,
    pub body: Option<String>,
}

use super::rule::error::ConfigurationError;
use super::intermediary::Intermediary;

impl Match {
    pub fn is_match(&self, intermediary: &Intermediary) -> bool {
        // Check URL pattern match
        if let Some(ref uri) = intermediary.uri {
            if !self.uri_matches(uri.path()) {
                return false;
            }
        }

        // Check body pattern match if specified
        if let Some(ref body_pattern) = self.body {
            if let Some(ref actual_body) = intermediary.body {
                if !actual_body.contains(body_pattern) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    pub fn verify(&self, intermediary: &Intermediary) -> Result<(), ConfigurationError> {
        // Verify URI is valid
        if self.uri.is_empty() {
            return Err(ConfigurationError::MalformedUri("Empty URI pattern".to_string()));
        }

        // Check if intermediary has a URI to match against
        if intermediary.uri.is_none() {
            return Err(ConfigurationError::NoUriError);
        }

        Ok(())
    }

    fn uri_matches(&self, path: &str) -> bool {
        // Simple glob-like pattern matching
        let pattern = self.uri.replace("*", ".*");
        let re = regex::Regex::new(&format!("^{}$", pattern)).unwrap_or_else(|_| regex::Regex::new(".*").unwrap());
        re.is_match(path)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Plugin {
    pub name: String,
    pub with: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyResponseFips {
    #[serde(rename = "setHeaders")]
    pub set_headers: Option<HashMap<String, String>>,
    #[serde(rename = "deleteHeaders")]
    pub delete_headers: Option<Vec<String>>,
    pub body: Option<Vec<BodyManipulation>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyResponseProxy {
    #[serde(rename = "setHeaders")]
    pub add_headers: Option<HashMap<String, String>>,
    #[serde(rename = "keepHeaders")]
    pub delete_headers: Option<Vec<String>>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BodyManipulation {
    pub at: String,
    pub with: Value,
}

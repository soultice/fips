use eyre::{eyre, Result};
use lazy_static::lazy_static;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, path::PathBuf};

use crate::plugin_registry::ExternalFunctions;

use super::loader::{DeserializationError, YamlFileLoader};

use super::rule::{then::Then, when::When, Rule};
use super::ruleset::RuleSet;

lazy_static! {
    static ref HTTP_METHODS: Vec<String> = vec![
        String::from("GET"),
        String::from("OPTIONS"),
        String::from("POST"),
        String::from("PUT"),
        String::from("DELETE"),
        String::from("HEAD"),
        String::from("TRACE"),
        String::from("CONNECT"),
        String::from("PATCH"),
    ];
}

/*
Rule {
  when {
    matches
    bodyContains
    probability
  }
  then {
    type: Proxy / fips / mock / static
    forwardUri // only for proxy and fips
    modifyRequest // only for proxy and fips
    modifyResponse // only for proxy and fips
    ...
  }
  with {
    sleep
    plugins
  }
}
 */

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Match {
    pub uri: String,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
enum RuleType {
    NonForwarding,
    Forwarding,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModifyRequest {
    #[serde(rename = "setHeaders")]
    add_headers: Option<HashMap<String, String>>,
    #[serde(rename = "keepHeaders")]
    keep_headers: Option<Vec<String>>,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Plugin {
    pub path: String,
    pub name: String,
    pub args: Option<Value>,
}

#[derive(Deserialize, Clone, Debug, JsonSchema)]
pub struct Config {
    pub active_rule_indices: Vec<usize>,
    pub fe_selected_rule: usize,
    pub rules: Vec<RuleSet>,
    #[cfg(feature = "logging")]
    pub parse_errors: Vec<(String, String)>, // (path, error)
}

impl Default for Config {
    fn default() -> Self {
        Config {
            active_rule_indices: vec![0],
            fe_selected_rule: 0,
            rules: vec![RuleSet::Rule(Rule {
                name: String::from("Static fallback - no rules found"),
                plugins: None,
                when: When {
                    matches: vec![Match {
                        uri: String::from(".*"),
                        body: None,
                    }],
                    matches_methods: None,
                    body_contains: None,
                },
                then: Then::Static {
                    static_base_dir: Some(
                        std::env::current_dir()
                            .unwrap()
                            .into_os_string()
                            .into_string()
                            .unwrap(),
                    ),
                    strip_path: false,
                },
                with: None,
                path: String::from(""),
            })],
            #[cfg(feature = "logging")]
            parse_errors: vec![],
        }
    }
}

impl Config {
    pub fn load(paths: &[PathBuf]) -> Result<Config, DeserializationError> {
        let extensions = vec![String::from("yaml"), String::from("yml")];
        let loader = YamlFileLoader { extensions };
        let (mut rules, _errors) = loader.load_from_directories_with_errors(paths);

        if rules.is_empty() {
            let cfg = Config::default();
            #[cfg(feature = "logging")]
            { cfg.parse_errors = _errors; }
            return Ok(cfg);
        }

        //load plugins
        //TODO: error handling here, else one faulty plugin block destroys the whole config
        for rule in &mut rules {
            match rule {
                RuleSet::Rule(rule) => {
                    if let Some(with) = &rule.with {
                        if let Some(plugins) = &with.plugins {
                            // Create a single ExternalFunctions instance for this rule
                            let mut external_functions = ExternalFunctions::default();
                            
                            // Load all plugins for this rule into the same instance
                            for plugin in plugins {
                                let path = PathBuf::from(&plugin.path);
                                let absolute_path = path.canonicalize()?;
                                external_functions.load_from_file(&absolute_path)?;
                            }
                            
                            // Only set if we successfully loaded at least one plugin
                            if !plugins.is_empty() {
                                rule.plugins = Some(external_functions);
                            }
                        }
                    }
                }
            }
        }
        #[cfg(feature = "logging")]
        {
            return Ok(Config {
                active_rule_indices: (0..rules.len()).collect(),
                fe_selected_rule: 0,
                rules,
                parse_errors: _errors,
            });
        }
        #[cfg(not(feature = "logging"))]
        {
            return Ok(Config {
                active_rule_indices: (0..rules.len()).collect(),
                fe_selected_rule: 0,
                rules,
            });
        }
    }

    pub fn reload(&mut self, paths: &[PathBuf]) -> Result<()> {
        //TODO enable plugin reload
        match Config::load(paths) {
            Ok(new_config) => {
                self.rules = new_config.rules;
                self.active_rule_indices = new_config.active_rule_indices;
                self.fe_selected_rule = new_config.fe_selected_rule;
                #[cfg(feature = "logging")]
                {
                    self.parse_errors = new_config.parse_errors;
                }
                Ok(())
            }
            Err(e) => Err(eyre!("Error reloading config: {e:?}")),
        }
    }

    pub fn select_next(&mut self) {
        self.fe_selected_rule = (self.fe_selected_rule + 1) % self.rules.len();
    }

    pub fn select_previous(&mut self) {
        self.fe_selected_rule =
            (self.fe_selected_rule + self.rules.len() - 1) % self.rules.len();
    }

    pub fn toggle_rule(&mut self) {
        if self.active_rule_indices.contains(&self.fe_selected_rule) {
            self.remove_from_active_indices();
        } else {
            self.add_to_active_indices();
        }
    }

    pub fn remove_from_active_indices(&mut self) {
        self.active_rule_indices
            .retain(|&x| x != self.fe_selected_rule);
    }

    pub fn add_to_active_indices(&mut self) {
        self.active_rule_indices.push(self.fe_selected_rule);
    }
}

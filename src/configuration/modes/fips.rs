use serde::{Deserialize, Serialize};
use std::{ collections::HashMap };
use hyper::Uri;
use std::str::{FromStr};
use schemars::JsonSchema;

use crate::{configuration::{rule_collection::{default_as_true, RuleTransformingFunctions, apply_plugins, ProxyFunctions, RuleCollectionError, CommonFunctions}, rule::Rule}, plugin_registry::ExternalFunctions};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct FIPS {
    pub name: Option<String>,
    pub path: String,
    #[serde(rename = "matchProbability")]
    pub match_with_prob: Option<f32>,
    #[serde(rename = "matchBodyContains")]
    pub match_body_contains: Option<String>,
    #[serde(rename = "matchMethods")]
    pub match_methods: Option<Vec<String>>,
    #[serde(rename = "serveStatic")]
    pub serve_static: Option<String>,
    #[serde(skip)]
    pub selected: bool,
    pub sleep: Option<u64>,
    #[serde(default = "default_as_true")]
    pub active: bool,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<u16>,
    #[serde(rename = "forwardUri")]
    pub forward_uri: String,
    #[serde(rename = "forwardHeaders")]
    pub forward_headers: Option<Vec<String>>,
    #[serde(rename = "backwardHeaders")]
    pub backward_headers: Option<Vec<String>>,
    pub headers: Option<HashMap<String, String>>,
    pub rules: Option<Vec<Rule>>,
}

impl RuleTransformingFunctions for FIPS {
    fn apply_plugins(&mut self, template: &ExternalFunctions) {
        if let Some(rules) = &mut self.rules {
            for rule in rules {
                if let Some(item) = &mut rule.item {
                    apply_plugins(item, template);
                }
            }
        }
    }
}

impl ProxyFunctions for FIPS {
    fn get_forward_uri(&self) -> String {
        self.forward_uri.clone()
    }

    fn get_forward_headers(&self) -> Option<Vec<String>> {
        self.forward_headers.clone()
    }

    fn get_backward_headers(&self) -> Option<Vec<String>> {
        self.backward_headers.clone()
    }

    fn form_forward_path(&self, uri: &Uri) -> Result<Uri, RuleCollectionError> {
        Ok(Uri::from_str(&format!("{}{uri}", self.get_forward_uri()))?)
    }
}

impl CommonFunctions for FIPS {
    fn get_name(&self) -> Option<String> {
        self.name.clone()
    }

    fn get_path(&self) -> String {
        self.path.clone()
    }

    fn get_match_with_prob(&self) -> Option<f32> {
        self.match_with_prob
    }

    fn get_match_body_contains(&self) -> Option<String> {
        self.match_body_contains.clone()
    }

    fn get_match_methods(&self) -> Option<Vec<String>> {
        self.match_methods.clone()
    }

    fn get_selected(&self) -> bool {
        self.selected
    }

    fn get_sleep(&self) -> Option<u64> {
        self.sleep
    }

    fn get_active(&self) -> bool {
        self.active
    }

    fn get_headers(&self) -> Option<HashMap<String, String>> {
        self.headers.clone()
    }

    fn set_selected(&mut self) {
        self.selected = true;
    }

    fn set_unselected(&mut self) {
        self.selected = false;
    }

    fn set_active(&mut self) {
        self.active = true;
    }

    fn set_inactive(&mut self) {
        self.active = false;
    }
}

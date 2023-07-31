use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::{configuration::rule_collection::{default_as_true, RuleTransformingFunctions, apply_plugins, CommonFunctions}, plugin_registry::ExternalFunctions};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct MOCK {
    pub name: Option<String>,
    pub path: String,
    #[serde(rename = "matchProbability")]
    pub match_with_prob: Option<f32>,
    #[serde(rename = "matchBodyContains")]
    pub match_body_contains: Option<String>,
    #[serde(rename = "matchMethods")]
    pub match_methods: Option<Vec<String>>,
    #[serde(skip)]
    pub selected: bool,
    pub sleep: Option<u64>,
    #[serde(default = "default_as_true")]
    pub active: bool,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<u16>,
    #[serde(rename = "forwardUri")]
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<Value>,
}
impl RuleTransformingFunctions for MOCK {
    fn apply_plugins(&mut self, template: &ExternalFunctions) {
        if let Some(body) = &mut self.body {
            apply_plugins(body, template);
        }
    }
}

impl CommonFunctions for MOCK {
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

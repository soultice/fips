use super::mode::Mode;
use super::rule::Rule;
use hyper::Uri;
use plugin_registry::plugin::ExternalFunctions;
use serde::{Deserialize, Serialize};
use serde_json::ser::CompactFormatter;
use std::{
    collections::HashMap,
    convert::AsRef,
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
    str::FromStr,
};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuleCollectionError {
    #[error("Could not parse URI: {0}")]
    UriParseError(#[from] hyper::http::uri::InvalidUri),
    #[error("Could not format String: {0}")]
    StringFromatError(#[from] std::fmt::Error),
}

const MACH_ALL_REQUESTS_STR: &str = "^./*$";

// see https://github.com/serde-rs/serde/issues/1030
fn default_as_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct STATIC {
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
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub rules: Vec<Rule>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PROXY {
    pub path: String,
    pub name: Option<String>,
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
    #[serde(rename = "forwardUri")]
    pub forward_uri: String,
    #[serde(rename = "forwardHeaders")]
    pub forward_headers: Option<Vec<String>>,
    #[serde(rename = "backwardHeaders")]
    pub backward_headers: Option<Vec<String>>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RuleCollection {
    STATIC(STATIC),
    MOCK(MOCK),
    PROXY(PROXY),
    FIPS(FIPS),
}

impl From<&mut RuleCollection> for Mode {
    fn from(rule: &mut RuleCollection) -> Self {
        match rule {
            RuleCollection::STATIC(_) => Mode::STATIC,
            RuleCollection::MOCK(_) => Mode::MOCK,
            RuleCollection::PROXY(_) => Mode::PROXY,
            RuleCollection::FIPS(_) => Mode::FIPS,
        }
    }
}

impl Default for RuleCollection {
    fn default() -> RuleCollection {
        RuleCollection::STATIC(STATIC {
            name: Some(String::from(
                "static asset fallback rule if no others found",
            )),
            match_body_contains: None,
            match_methods: None,
            match_with_prob: None,
            sleep: None,
            selected: true,
            active: true,
            path: String::from(MACH_ALL_REQUESTS_STR),
            headers: None,
        })
    }
}

pub trait ProxyFunctions {
    fn get_forward_uri(&self) -> String;
    fn get_forward_headers(&self) -> Option<Vec<String>>;
    fn get_backward_headers(&self) -> Option<Vec<String>>;
    fn form_forward_path(&self, uri: &Uri) -> Result<Uri, RuleCollectionError>;
}

pub trait RuleTransformingFunctions {
    fn expand_rule_template(&mut self, template: &ExternalFunctions);
}

impl ProxyFunctions for PROXY {
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

impl RuleTransformingFunctions for FIPS {
    fn expand_rule_template(&mut self, template: &ExternalFunctions) {
        if let Some(rules) = &mut self.rules {
            for rule in rules {
                if let Some(item) = &mut rule.item {
                    recursive_expand(item, template);
                }
            }
        }
    }
}

impl RuleTransformingFunctions for MOCK {
    fn expand_rule_template(&mut self, template: &ExternalFunctions) {
        for rule in &mut self.rules {
            if let Some(item) = &mut rule.item {
                recursive_expand(item, template);
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

pub trait CommonFunctions {
    fn get_name(&self) -> Option<String>;
    fn get_path(&self) -> String;
    fn get_match_with_prob(&self) -> Option<f32>;
    fn get_match_body_contains(&self) -> Option<String>;
    fn get_match_methods(&self) -> Option<Vec<String>>;
    fn get_selected(&self) -> bool;
    fn get_sleep(&self) -> Option<u64>;
    fn get_active(&self) -> bool;
    fn get_headers(&self) -> Option<HashMap<String, String>>;
    fn set_selected(&mut self);
    fn set_unselected(&mut self);
    fn set_active(&mut self);
    fn set_inactive(&mut self);
}

impl CommonFunctions for STATIC {
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

impl CommonFunctions for PROXY {
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

impl Deref for RuleCollection {
    type Target = dyn CommonFunctions;

    fn deref(&self) -> &Self::Target {
        match self {
            RuleCollection::STATIC(s) => s,
            RuleCollection::MOCK(s) => s,
            RuleCollection::PROXY(s) => s,
            RuleCollection::FIPS(s) => s,
        }
    }
}

impl DerefMut for RuleCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            RuleCollection::STATIC(s) => s,
            RuleCollection::MOCK(s) => s,
            RuleCollection::PROXY(s) => s,
            RuleCollection::FIPS(s) => s,
        }
    }
}

impl Display for RuleCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            RuleCollection::STATIC(_s) => write!(f, "STATIC"),
            RuleCollection::MOCK(_s) => write!(f, "MOCK"),
            RuleCollection::PROXY(_s) => write!(f, "PROXY"),
            RuleCollection::FIPS(_s) => write!(f, "FIPS"),
        }
    }
}

fn recursive_expand(value: &mut serde_json::Value, plugins: &ExternalFunctions) {
    match value {
        serde_json::Value::String(val) => {
            if plugins.has(val) {
                let result = plugins.call(val, vec![]).expect("Invocation failed");
                let try_serialize = serde_json::from_str(&result);
                if let Ok(i) = try_serialize {
                    *value = i;
                } else {
                    *value = serde_json::Value::String(result);
                }
            }
        }
        serde_json::Value::Array(val) => {
            for i in val {
                recursive_expand(i, plugins);
            }
        }
        serde_json::Value::Object(val) => {
            let plugin = val.get("plugin");
            let args = val.get("args");
            match (plugin, args) {
                (Some(p), Some(a)) => {
                    if let (
                        serde_json::Value::String(function),
                        serde_json::Value::Array(arguments),
                    ) = (p, a)
                    {
                        let result = plugins
                            .call(function, arguments.clone())
                            .expect("Invocation failed");
                        let try_serialize = serde_json::from_str(&result);
                        if let Ok(i) = try_serialize {
                            *value = i;
                        } else {
                            *value = serde_json::Value::String(result);
                        }
                    }
                }
                _ => {
                    for (_, i) in val {
                        recursive_expand(i, plugins);
                    }
                }
            }
        }
        _ => {}
    }
}

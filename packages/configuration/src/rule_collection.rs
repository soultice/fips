use super::mode::Mode;
use super::rule::Rule;
use hyper::Uri;
use plugin_registry::plugin::ExternalFunctions;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

// see https://github.com/serde-rs/serde/issues/1030
fn default_as_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuleCollection {
    pub name: Option<String>,
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
    pub path: String,
    #[serde(rename = "responseStatus")]
    pub response_status: Option<u16>,
    #[serde(rename = "forwardUri")]
    pub forward_uri: Option<String>,
    #[serde(rename = "forwardHeaders")]
    pub forward_headers: Option<Vec<String>>,
    #[serde(rename = "backwardHeaders")]
    pub backward_headers: Option<Vec<String>>,
    pub headers: Option<HashMap<String, String>>,
    pub rules: Option<Vec<Rule>>,
}

impl Default for RuleCollection {
    fn default() -> RuleCollection {
        RuleCollection {
            name: Some(String::from("static asset fallback rule if no others found")),
            serve_static: Some(String::from(std::env::current_dir().unwrap().to_str().unwrap())),
            match_body_contains: None,
            match_methods: None,
            match_with_prob: None,
            sleep: None,
            selected: true,
            active: true,
            path: String::from("^/.*$"),
            forward_uri: None,
            headers: None,
            forward_headers: None,
            backward_headers: None,
            rules: None,
            response_status: None,
        }
    }
}

impl RuleCollection {
    pub fn expand_rule_template(&mut self, plugins: &ExternalFunctions) -> () {
        if let Some(rules) = &mut self.rules {
            for rule in rules {
                match &mut rule.item {
                    Some(item) => {
                        recursive_expand(item, plugins);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn mode(&self) -> Mode {
        let mode: Mode = match (&self.forward_uri, &self.rules, &self.serve_static) {
            (Some(_), Some(_), None) => Mode::FIPS,
            (None, Some(_), None) => Mode::MOCK,
            (None, None, Some(_)) => Mode::STATIC,
            _ => Mode::PROXY,
        };
        mode
    }

    pub fn forward_url(&self, uri: &Uri) -> Uri {
        let mut url_path = String::from("");
        if let Some(forward_url) = &self.forward_uri.clone() {
            url_path.push_str(&forward_url);
        }
        url_path.push_str(&uri.to_string());
        Uri::from_str(&url_path).ok().unwrap()
    }
}

fn recursive_expand(value: &mut serde_json::Value, plugins: &ExternalFunctions) {
    match value {
        serde_json::Value::String(val) => match val.as_str() {
            _ => {
                if plugins.has(val) {
                    let result = plugins.call(&val, vec![]).expect("Invocation failed");
                    let try_serialize = serde_json::from_str(&result);
                    if let Ok(i) = try_serialize {
                        *value = i;
                    } else {
                        *value = serde_json::Value::String(result);
                    }
                }
            }
        },
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
                    match (p, a) {
                        (
                            serde_json::Value::String(function),
                            serde_json::Value::Array(arguments),
                        ) => {
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
                        // wrong format of plugin & args combo
                        _ => {}
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

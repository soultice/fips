use crate::modes::fips::FIPS;
use crate::modes::host_static::STATIC;
use crate::modes::mock::MOCK;
use crate::modes::proxy::PROXY;

use hyper::Uri;
use plugin_registry::plugin::ExternalFunctions;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::{Display, Formatter},
    ops::{Deref, DerefMut},
};
use schemars::JsonSchema;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum RuleCollectionError {
    #[error("Could not parse URI: {0}")]
    UriParseError(#[from] hyper::http::uri::InvalidUri),
    #[error("Could not format String: {0}")]
    StringFromatError(#[from] std::fmt::Error),
}

const MACH_ALL_REQUESTS_STR: &str = "^/.*$";

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub enum RuleCollection {
    Static(STATIC),
    Mock(MOCK),
    Proxy(PROXY),
    Fips(FIPS),
}

impl Default for RuleCollection {
    fn default() -> RuleCollection {
        RuleCollection::Static(STATIC {
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
            static_base_dir: String::from(std::env::current_dir().unwrap().to_str().unwrap()),
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
    fn apply_plugins(&mut self, template: &ExternalFunctions);
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

impl Deref for RuleCollection {
    type Target = dyn CommonFunctions;

    fn deref(&self) -> &Self::Target {
        match self {
            RuleCollection::Static(s) => s,
            RuleCollection::Mock(s) => s,
            RuleCollection::Proxy(s) => s,
            RuleCollection::Fips(s) => s,
        }
    }
}

impl DerefMut for RuleCollection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            RuleCollection::Static(s) => s,
            RuleCollection::Mock(s) => s,
            RuleCollection::Proxy(s) => s,
            RuleCollection::Fips(s) => s,
        }
    }
}

impl Display for RuleCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            RuleCollection::Static(_s) => write!(f, "STATIC"),
            RuleCollection::Mock(_s) => write!(f, "MOCK"),
            RuleCollection::Proxy(_s) => write!(f, "PROXY"),
            RuleCollection::Fips(_s) => write!(f, "FIPS"),
        }
    }
}

pub fn default_as_true() -> bool {
    true
}

pub fn apply_plugins(value: &mut serde_json::Value, plugins: &ExternalFunctions) {
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
                apply_plugins(i, plugins);
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
                        apply_plugins(i, plugins);
                    }
                }
            }
        }
        _ => {}
    }
}

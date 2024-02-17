use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use schemars::JsonSchema;

use super::super::configuration::{ModifyResponseFips, ModifyResponseProxy};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "functionAs")]
pub enum Then {
    Fips {
        #[serde(rename = "forwardUri")]
        forward_uri: String,
        #[serde(rename = "modifyResponse")]
        modify_response: Option<ModifyResponseFips>,
    },
    Proxy {
        #[serde(rename = "forwardUri")]
        forward_uri: String,
        modify_response: Option<ModifyResponseProxy>,
    },
    Static {
        #[serde(rename = "baseDir")]
        static_base_dir: Option<String>,
    },
    Mock {
        body: Option<Value>,
        status: Option<String>,
        headers: Option<HashMap<String, String>>,
    },
}

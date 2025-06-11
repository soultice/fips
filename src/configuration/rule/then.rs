use std::collections::HashMap;
use std::str::FromStr;
use eyre::Result;
use hyper::http::{HeaderName, HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use schemars::JsonSchema;
use crate::configuration::types::{ModifyResponseFips, ModifyResponseProxy};
use crate::configuration::intermediary::Intermediary;
use crate::plugin_registry::ExternalFunctions;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type")]
pub enum Then {
    #[serde(rename = "fips")]
    Fips(FipsConfig),
    #[serde(rename = "proxy")]
    Proxy(ProxyConfig),
    #[serde(rename = "static")]
    Static(StaticConfig),
    #[serde(rename = "mock")]
    Mock(MockConfig),
}

impl Then {
    pub async fn apply(&self, intermediary: &mut Intermediary, functions: &ExternalFunctions) -> Result<()> {
        match self {
            Then::Fips(config) => config.apply(intermediary, functions).await,
            Then::Proxy(config) => config.apply(intermediary, functions).await,
            Then::Static(config) => config.apply(intermediary).await,
            Then::Mock(config) => config.apply(intermediary).await,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FipsConfig {
    pub modify_request: Option<HashMap<String, Value>>,
    pub modify_response: Option<ModifyResponseFips>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProxyConfig {
    pub url: String,
    pub modify_request: Option<HashMap<String, Value>>,
    pub modify_response: Option<ModifyResponseProxy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StaticConfig {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MockConfig {
    pub status: u16,
    pub headers: Option<HashMap<String, String>>,
    pub body: Option<Value>,
}

impl FipsConfig {
    pub async fn apply(&self, intermediary: &mut Intermediary, functions: &ExternalFunctions) -> Result<()> {
        if let Some(ref modify_request) = self.modify_request {
            // Apply request modifications
            apply_json_modifications(intermediary, modify_request)?;
        }
        
        if let Some(ref modify_response) = self.modify_response {
            // Apply response modifications when response is ready
            apply_fips_response_modifications(intermediary, modify_response, functions).await?;
        }
        
        Ok(())
    }
}

impl ProxyConfig {
    pub async fn apply(&self, intermediary: &mut Intermediary, functions: &ExternalFunctions) -> Result<()> {
        if let Some(ref modify_request) = self.modify_request {
            // Apply request modifications
            apply_json_modifications(intermediary, modify_request)?;
        }
        
        if let Some(ref modify_response) = self.modify_response {
            // Apply response modifications when response is ready
            apply_proxy_response_modifications(intermediary, modify_response, functions).await?;
        }
        
        Ok(())
    }
}

impl StaticConfig {
    pub async fn apply(&self, _intermediary: &mut Intermediary) -> Result<()> {
        // TODO: Implement static file serving
        Ok(())
    }
}

impl MockConfig {
    pub async fn apply(&self, intermediary: &mut Intermediary) -> Result<()> {
        // Set response status
        intermediary.status = StatusCode::from_u16(self.status)?;
        
        // Set mock response headers
        if let Some(ref headers) = self.headers {
            for (key, value) in headers {
                let header_name = HeaderName::from_str(key)?;
                let header_value = HeaderValue::from_str(value)?;
                intermediary.headers.insert(header_name, header_value);
            }
        }
        
        // Set mock response body
        if let Some(ref body) = self.body {
            intermediary.body = Some(serde_json::to_string(body)?);
        }
        
        Ok(())
    }
}

// Helper functions for modifications
fn apply_json_modifications(intermediary: &mut Intermediary, modifications: &HashMap<String, Value>) -> Result<()> {
    if let Some(ref body) = intermediary.body {
        let mut json: Value = serde_json::from_str(body)?;
        for (path, value) in modifications {
            apply_json_path(&mut json, path, value.clone())?;
        }
        intermediary.body = Some(serde_json::to_string(&json)?);
    }
    Ok(())
}

fn apply_json_path(json: &mut Value, path: &str, value: Value) -> Result<()> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = json;
    
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            *current = value;
            break;
        }
        
        current = if let Some(obj) = current.as_object_mut() {
            obj.entry(part.to_string())
                .or_insert(Value::Object(serde_json::Map::new()))
        } else {
            return Ok(());
        };
    }
    
    Ok(())
}

async fn apply_fips_response_modifications(
    intermediary: &mut Intermediary,
    _modifications: &ModifyResponseFips,
    _functions: &ExternalFunctions
) -> Result<()> {
    // TODO: Implement FIPS response modifications
    Ok(())
}

async fn apply_proxy_response_modifications(
    intermediary: &mut Intermediary,
    _modifications: &ModifyResponseProxy,
    _functions: &ExternalFunctions
) -> Result<()> {
    // TODO: Implement proxy response modifications
    Ok(())
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_then_mock_deserialization() {
        let yaml = r#"
functionAs: Mock
body:
  message: "Hello"
status: "200"
headers:
  Content-Type: "application/json"
"#;
        let then: Result<Then, _> = serde_yaml::from_str(yaml);
        assert!(then.is_ok());
        
        if let Then::Mock { body, status, headers } = then.unwrap() {
            assert!(body.is_some());
            assert_eq!(status.as_ref().unwrap(), "200");
            assert!(headers.is_some());
        } else {
            panic!("Expected Mock variant");
        }
    }

    #[test]
    fn test_then_proxy_deserialization() {
        let yaml = r#"
functionAs: Proxy
forwardUri: "http://localhost:8080"
"#;
        let then: Result<Then, _> = serde_yaml::from_str(yaml);
        assert!(then.is_ok());
        
        if let Then::Proxy { forward_uri, .. } = then.unwrap() {
            assert_eq!(forward_uri, "http://localhost:8080");
        } else {
            panic!("Expected Proxy variant");
        }
    }

    #[test]
    fn test_then_fips_deserialization() {
        let yaml = r#"
functionAs: Fips
forwardUri: "http://localhost:9090"
"#;
        let then: Result<Then, _> = serde_yaml::from_str(yaml);
        assert!(then.is_ok());
        
        if let Then::Fips { forward_uri, .. } = then.unwrap() {
            assert_eq!(forward_uri, "http://localhost:9090");
        } else {
            panic!("Expected Fips variant");
        }
    }

    #[test]
    fn test_then_static_deserialization() {
        let yaml = r#"
functionAs: Static
baseDir: "./static"
"#;
        let then: Result<Then, _> = serde_yaml::from_str(yaml);
        assert!(then.is_ok());
        
        if let Then::Static { static_base_dir } = then.unwrap() {
            assert_eq!(static_base_dir.as_ref().unwrap(), "./static");
        } else {
            panic!("Expected Static variant");
        }
    }
}

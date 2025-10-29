use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use super::super::configuration::Match;


#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct When {
    #[serde(rename = "matchesUris")]
    pub matches: Vec<Match>,
    #[serde(rename = "matchesMethods")]
    pub matches_methods: Option<Vec<String>>,
    #[serde(rename = "bodyContains")]
    pub body_contains: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_when_deserialization() {
        let yaml = r#"
matchesUris:
  - uri: ^/test$
matchesMethods:
  - GET
  - POST
bodyContains: "search_term"
"#;
        let when: Result<When, _> = serde_yaml::from_str(yaml);
        assert!(when.is_ok());
        
        let when = when.unwrap();
        assert_eq!(when.matches.len(), 1);
        assert_eq!(when.matches_methods.as_ref().unwrap().len(), 2);
        assert_eq!(when.body_contains.as_ref().unwrap(), "search_term");
    }

    #[test]
    fn test_when_optional_fields() {
        let yaml = r#"
matchesUris:
  - uri: ^/test$
"#;
        let when: Result<When, _> = serde_yaml::from_str(yaml);
        assert!(when.is_ok());
        
        let when = when.unwrap();
        assert!(when.matches_methods.is_none());
        assert!(when.body_contains.is_none());
    }
}


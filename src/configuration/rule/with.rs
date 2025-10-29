use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::configuration::configuration::Plugin;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct With {
    pub sleep: Option<u64>,
    pub probability: Option<f32>,
    pub plugins: Option<Vec<Plugin>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_deserialization_all_fields() {
        let yaml = r#"
sleep: 1000
probability: 0.75
plugins:
  - name: TestPlugin
    path: "./plugins/test.so"
"#;
        let with: Result<With, _> = serde_yaml::from_str(yaml);
        assert!(with.is_ok());
        
        let with = with.unwrap();
        assert_eq!(with.sleep, Some(1000));
        assert_eq!(with.probability, Some(0.75));
        assert!(with.plugins.is_some());
    }

    #[test]
    fn test_with_deserialization_optional_fields() {
        let yaml = r#"
probability: 1.0
"#;
        let with: Result<With, _> = serde_yaml::from_str(yaml);
        assert!(with.is_ok());
        
        let with = with.unwrap();
        assert!(with.sleep.is_none());
        assert_eq!(with.probability, Some(1.0));
        assert!(with.plugins.is_none());
    }

    #[test]
    fn test_with_probability_bounds() {
        let yaml_low = r#"
probability: 0.0
"#;
        let with: Result<With, _> = serde_yaml::from_str(yaml_low);
        assert!(with.is_ok());
        assert_eq!(with.unwrap().probability, Some(0.0));

        let yaml_high = r#"
probability: 1.0
"#;
        let with: Result<With, _> = serde_yaml::from_str(yaml_high);
        assert!(with.is_ok());
        assert_eq!(with.unwrap().probability, Some(1.0));
    }
}


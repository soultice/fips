/// Integration tests for configuration loading and rule parsing

mod common;

use std::path::PathBuf;

#[test]
fn test_yaml_file_loader_loads_rules() {
    let loader = fips::configuration::loader::YamlFileLoader {
        extensions: vec![r"nrule\.yml$".to_string()],
    };

    let dirs = vec![PathBuf::from("./nconfig-test")];
    let result = loader.load_from_directories(&dirs);

    assert!(result.is_ok(), "Failed to load configuration files: {:?}", result.err());
    let rulesets = result.unwrap();
    println!("Loaded {} rulesets", rulesets.len());
    assert!(!rulesets.is_empty(), "No rules were loaded. Check that nconfig-test directory exists and contains .nrule.yml files");
}

#[test]
fn test_yaml_loader_filters_by_extension() {
    let loader = fips::configuration::loader::YamlFileLoader {
        extensions: vec![r"nrule\.yml$".to_string()],
    };

    let dirs = vec![PathBuf::from("./examples")];
    let result = loader.load_from_directories(&dirs);

    // Should successfully load but find no matching files (returns empty vec, not error)
    assert!(result.is_ok(), "Should succeed even with no matching files");
    assert_eq!(result.unwrap().len(), 0, "Should have filtered out all .yaml files");
}

#[test]
fn test_yaml_loader_invalid_path() {
    let loader = fips::configuration::loader::YamlFileLoader {
        extensions: vec![r".*\.nrule\.yml$".to_string()],
    };

    let dirs = vec![PathBuf::from("./nonexistent_directory")];
    let result = loader.load_from_directories(&dirs);

    assert!(result.is_err(), "Should fail with invalid path");
}

#[test]
fn test_ruleset_deserialization() {
    
    let content = r#"
- Rule:
    name: "Test Rule"
    when:
      matchesUris:
        - uri: ^/test$
    then:
      functionAs: "Mock"
      body:
        message: "test"
      status: "200"
      headers:
        Content-Type: "application/json"
"#;

    let rulesets: Result<Vec<fips::configuration::ruleset::RuleSet>, _> = 
        serde_yaml::from_str(content);
    
    assert!(rulesets.is_ok(), "Failed to deserialize test rule");
    let rulesets = rulesets.unwrap();
    assert_eq!(rulesets.len(), 1);
    
    match &rulesets[0] {
        fips::configuration::ruleset::RuleSet::Rule(rule) => {
            assert_eq!(rule.name, "Test Rule");
        }
    }
}

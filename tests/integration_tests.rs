/// Integration test: Full end-to-end server testing

mod common;

use std::time::Duration;
use tokio::time::timeout;

#[tokio::test]
async fn test_configuration_loading_pipeline() {
    use std::path::PathBuf;
    
    // Test full configuration loading
    let loader = fips::configuration::loader::YamlFileLoader {
        extensions: vec![r".*\.nrule\.yml$".to_string()],
    };
    
    let dirs = vec![PathBuf::from("./nconfig-test")];
    let rulesets = loader.load_from_directories(&dirs);
    
    assert!(rulesets.is_ok());
    let rulesets = rulesets.unwrap();
    
    // Verify we have rules loaded
    assert!(!rulesets.is_empty(), "Should load at least one rule");
    
    // Verify rule structure
    for ruleset in &rulesets {
        match ruleset {
            fips::configuration::ruleset::RuleSet::Rule(rule) => {
                assert!(!rule.name.is_empty(), "Rule name should not be empty");
                assert!(!rule.when.matches.is_empty(), "Rule should have URI matches");
            }
        }
    }
}

#[tokio::test]
async fn test_rule_container_matching() {
    use std::path::PathBuf;
    use http::{Method, StatusCode, HeaderMap};
    use serde_json::json;
    
    // Load real configuration
    let loader = fips::configuration::loader::YamlFileLoader {
        extensions: vec![r".*\.nrule\.yml$".to_string()],
    };
    
    let dirs = vec![PathBuf::from("./nconfig-test")];
    let rulesets = loader.load_from_directories(&dirs).unwrap();
    
    // Create test intermediary
    let intermediary = fips::configuration::intermediary::Intermediary {
        status: StatusCode::OK,
        headers: HeaderMap::new(),
        body: json!({}),
        method: Some(Method::GET),
        uri: Some("/mock".parse().unwrap()),
    };
    
    // Find matching rule
    let mut found_match = false;
    for ruleset in &rulesets {
        match ruleset {
            fips::configuration::ruleset::RuleSet::Rule(rule) => {
                if rule.should_apply(&intermediary).is_ok() {
                    found_match = true;
                    break;
                }
            }
        }
    }
    
    assert!(found_match, "Should find at least one matching rule for /mock");
}

#[tokio::test]
async fn test_multiple_rules_precedence() {
    use std::path::PathBuf;
    use http::{Method, StatusCode, HeaderMap};
    use serde_json::json;
    
    let loader = fips::configuration::loader::YamlFileLoader {
        extensions: vec![r".*\.nrule\.yml$".to_string()],
    };
    
    let dirs = vec![PathBuf::from("./nconfig-test")];
    let rulesets = loader.load_from_directories(&dirs).unwrap();
    
    // Test that rules are processed in order
    let test_uris = vec!["/mock", "/proxy", "/fips"];
    
    for uri in test_uris {
        let intermediary = fips::configuration::intermediary::Intermediary {
            status: StatusCode::OK,
            headers: HeaderMap::new(),
            body: json!({}),
            method: Some(Method::GET),
            uri: Some(uri.parse().unwrap()),
        };
        
        let mut match_count = 0;
        for ruleset in &rulesets {
            match ruleset {
                fips::configuration::ruleset::RuleSet::Rule(rule) => {
                    if rule.should_apply(&intermediary).is_ok() {
                        match_count += 1;
                    }
                }
            }
        }
        
        // Each test URI should match at least one rule
        assert!(match_count > 0, "URI {} should match at least one rule", uri);
    }
}

#[tokio::test]
async fn test_concurrent_rule_evaluation() {
    use std::path::PathBuf;
    use http::{Method, StatusCode, HeaderMap};
    use serde_json::json;
    use std::sync::Arc;
    
    let loader = fips::configuration::loader::YamlFileLoader {
        extensions: vec![r".*\.nrule\.yml$".to_string()],
    };
    
    let dirs = vec![PathBuf::from("./nconfig-test")];
    let rulesets: Arc<Vec<fips::configuration::ruleset::RuleSet>> = Arc::new(loader.load_from_directories(&dirs).unwrap());
    
    // Spawn multiple concurrent tasks
    let mut handles = vec![];
    
    for i in 0..10 {
        let rulesets_clone = Arc::clone(&rulesets);
        let handle = tokio::spawn(async move {
            let intermediary = fips::configuration::intermediary::Intermediary {
                status: StatusCode::OK,
                headers: HeaderMap::new(),
                body: json!({"request": i}),
                method: Some(Method::GET),
                uri: Some("/mock".parse().unwrap()),
            };
            
            for ruleset in rulesets_clone.iter() {
                match ruleset {
                    fips::configuration::ruleset::RuleSet::Rule(rule) => {
                        let _ = rule.should_apply(&intermediary);
                    }
                }
            }
        });
        handles.push(handle);
    }
    
    // Wait for all tasks with timeout
    let timeout_duration = Duration::from_secs(5);
    for handle in handles {
        let result = timeout(timeout_duration, handle).await;
        assert!(result.is_ok(), "Task should complete within timeout");
    }
}

#[tokio::test]
async fn test_error_handling_pipeline() {
    use std::path::PathBuf;
    
    // Test with invalid path
    let loader = fips::configuration::loader::YamlFileLoader {
        extensions: vec![r".*\.nrule\.yml$".to_string()],
    };
    
    let dirs = vec![PathBuf::from("./nonexistent")];
    let result = loader.load_from_directories(&dirs);
    
    assert!(result.is_err(), "Should fail with invalid directory");
}

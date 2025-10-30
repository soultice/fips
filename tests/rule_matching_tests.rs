//! Tests for rule matching logic

mod common;

use http::{Method, StatusCode};
use serde_json::json;

#[test]
fn test_rule_should_apply_uri_match() {
    let rule = create_test_rule(vec![r"^/test$"], None, None);
    let intermediary = create_intermediary_with_uri("/test");
    
    let result = rule.should_apply(&intermediary);
    assert!(result.is_ok(), "Rule should match URI /test");
}

#[test]
fn test_rule_should_not_apply_uri_mismatch() {
    let rule = create_test_rule(vec![r"^/test$"], None, None);
    let intermediary = create_intermediary_with_uri("/other");
    
    let result = rule.should_apply(&intermediary);
    assert!(result.is_err(), "Rule should not match URI /other");
}

#[test]
fn test_rule_should_apply_method_match() {
    let rule = create_test_rule(
        vec![r"^/test$"],
        Some(vec!["GET".to_string(), "POST".to_string()]),
        None,
    );
    let mut intermediary = create_intermediary_with_uri("/test");
    intermediary.method = Some(Method::POST);
    
    let result = rule.should_apply(&intermediary);
    assert!(result.is_ok(), "Rule should match POST method");
}

#[test]
fn test_rule_should_not_apply_method_mismatch() {
    let rule = create_test_rule(
        vec![r"^/test$"],
        Some(vec!["GET".to_string()]),
        None,
    );
    let mut intermediary = create_intermediary_with_uri("/test");
    intermediary.method = Some(Method::DELETE);
    
    let result = rule.should_apply(&intermediary);
    assert!(result.is_err(), "Rule should not match DELETE method");
}

#[test]
fn test_rule_should_apply_body_contains() {
    let rule = create_test_rule(
        vec![r"^/test$"],
        None,
        Some("important"),
    );
    let mut intermediary = create_intermediary_with_uri("/test");
    intermediary.body = json!({"data": "important information"});
    
    let result = rule.should_apply(&intermediary);
    assert!(result.is_ok(), "Rule should match body containing 'important'");
}

#[test]
fn test_rule_should_not_apply_body_missing() {
    let rule = create_test_rule(
        vec![r"^/test$"],
        None,
        Some("important"),
    );
    let mut intermediary = create_intermediary_with_uri("/test");
    intermediary.body = json!({"data": "other information"});
    
    let result = rule.should_apply(&intermediary);
    assert!(result.is_err(), "Rule should not match body without 'important'");
}

#[test]
fn test_rule_multiple_uri_patterns() {
    let rule = create_test_rule(
        vec![r"^/test$", r"^/api/.*$", r"^/v1/.*$"],
        None,
        None,
    );
    
    let test_cases = vec![
        ("/test", true),
        ("/api/users", true),
        ("/v1/products", true),
        ("/other", false),
    ];
    
    for (uri, should_match) in test_cases {
        let intermediary = create_intermediary_with_uri(uri);
        let result = rule.should_apply(&intermediary);
        
        if should_match {
            assert!(result.is_ok(), "Rule should match URI: {}", uri);
        } else {
            assert!(result.is_err(), "Rule should not match URI: {}", uri);
        }
    }
}

#[test]
fn test_rule_probability_deterministic() {
    // Probability of 1.0 should always match
    let rule = create_test_rule_with_probability(vec![r"^/test$"], 1.0);
    let intermediary = create_intermediary_with_uri("/test");
    
    // Test multiple times to ensure consistency
    for _ in 0..10 {
        let result = rule.should_apply(&intermediary);
        assert!(result.is_ok(), "Rule with probability 1.0 should always match");
    }
}

// Helper functions

fn create_test_rule(
    uri_patterns: Vec<&str>,
    methods: Option<Vec<String>>,
    body_contains: Option<&str>,
) -> fips::configuration::rule::Rule {
    use fips::configuration::rule::{when::When, then::Then};
    use fips::configuration::configuration::Match;
    
    fips::configuration::rule::Rule {
        name: "Test Rule".to_string(),
        when: When {
            matches: uri_patterns
                .iter()
                .map(|uri| Match {
                    uri: uri.to_string(),
                    body: None,
                })
                .collect(),
            matches_methods: methods,
            body_contains: body_contains.map(|s| s.to_string()),
        },
        then: Then::Mock {
            body: Some(json!({"test": "data"})),
            status: Some("200".to_string()),
            headers: None,
        },
        with: None,
        path: String::new(),
        plugins: None,
    }
}

fn create_test_rule_with_probability(
    uri_patterns: Vec<&str>,
    probability: f32,
) -> fips::configuration::rule::Rule {
    use fips::configuration::rule::{when::When, then::Then, with::With};
    use fips::configuration::configuration::Match;
    
    fips::configuration::rule::Rule {
        name: "Test Rule".to_string(),
        when: When {
            matches: uri_patterns
                .iter()
                .map(|uri| Match {
                    uri: uri.to_string(),
                    body: None,
                })
                .collect(),
            matches_methods: None,
            body_contains: None,
        },
        then: Then::Mock {
            body: Some(json!({"test": "data"})),
            status: Some("200".to_string()),
            headers: None,
        },
        with: Some(With {
            probability: Some(probability),
            plugins: None,
            sleep: None,
        }),
        path: String::new(),
        plugins: None,
    }
}

fn create_intermediary_with_uri(uri: &str) -> fips::configuration::intermediary::Intermediary {
    use http::HeaderMap;
    
    let full_uri = format!("http://localhost:8888{}", uri);
    fips::configuration::intermediary::Intermediary {
        status: StatusCode::OK,
        headers: HeaderMap::new(),
        body: json!({}),
        method: Some(Method::GET),
        uri: Some(full_uri.parse().unwrap()),
    }
}

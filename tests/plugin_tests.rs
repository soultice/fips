/// Tests for plugin system functionality

mod common;

use fips::Function;
use serde_json::{json, Value};

#[test]
fn test_plugin_invocation_error_types() {
    use fips::InvocationError;
    
    let error = InvocationError::InvalidArgumentCount {
        expected: 2,
        found: 1,
    };
    
    assert_eq!(
        error.to_string(),
        "Invalid argument count: expected 2, found 1"
    );
    
    let error = InvocationError::Other {
        msg: "Custom error".to_string(),
    };
    
    assert_eq!(error.to_string(), "Plugin Error: Custom error");
}

// Mock plugin function for testing
struct MockFunction {
    name: String,
}

impl fips::Function for MockFunction {
    fn call(&self, args: Value) -> Result<String, fips::InvocationError> {
        if let Value::Array(arr) = args {
            if arr.is_empty() {
                return Ok("default".to_string());
            }
            if let Some(Value::String(s)) = arr.get(0) {
                return Ok(format!("processed_{}", s));
            }
        }
        Err(fips::InvocationError::Other {
            msg: "Invalid arguments".to_string(),
        })
    }
    
    fn help(&self) -> Option<&str> {
        Some("Mock function for testing")
    }
}

#[test]
fn test_mock_function_with_args() {
    let func = MockFunction {
        name: "test".to_string(),
    };
    
    let result = func.call(json!(["input"]));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "processed_input");
}

#[test]
fn test_mock_function_without_args() {
    let func = MockFunction {
        name: "test".to_string(),
    };
    
    let result = func.call(json!([]));
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "default");
}

#[test]
fn test_mock_function_invalid_args() {
    let func = MockFunction {
        name: "test".to_string(),
    };
    
    let result = func.call(json!({"key": "value"}));
    assert!(result.is_err());
}

#[test]
fn test_mock_function_help() {
    let func = MockFunction {
        name: "test".to_string(),
    };
    
    assert_eq!(func.help(), Some("Mock function for testing"));
}

// Test function that validates argument count
struct ArgCountFunction {
    expected_args: usize,
}

impl fips::Function for ArgCountFunction {
    fn call(&self, args: Value) -> Result<String, fips::InvocationError> {
        if let Value::Array(arr) = &args {
            if arr.len() != self.expected_args {
                return Err(fips::InvocationError::InvalidArgumentCount {
                    expected: self.expected_args,
                    found: arr.len(),
                });
            }
            return Ok("success".to_string());
        }
        Err(fips::InvocationError::Other {
            msg: "Arguments must be an array".to_string(),
        })
    }
}

#[test]
fn test_function_argument_validation() {
    let func = ArgCountFunction { expected_args: 2 };
    
    // Correct number of arguments
    let result = func.call(json!(["arg1", "arg2"]));
    assert!(result.is_ok());
    
    // Too few arguments
    let result = func.call(json!(["arg1"]));
    assert!(result.is_err());
    if let Err(fips::InvocationError::InvalidArgumentCount { expected, found }) = result {
        assert_eq!(expected, 2);
        assert_eq!(found, 1);
    }
    
    // Too many arguments
    let result = func.call(json!(["arg1", "arg2", "arg3"]));
    assert!(result.is_err());
}

#[test]
fn test_function_non_array_arguments() {
    let func = ArgCountFunction { expected_args: 1 };
    
    let result = func.call(json!({"not": "array"}));
    assert!(result.is_err());
}

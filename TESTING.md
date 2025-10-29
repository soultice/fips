# FIPS Test Suite

## Overview

This test suite provides comprehensive coverage of the FIPS mock server with minimal footprint. Tests are organized into unit tests (embedded in source files) and integration tests (in the `tests/` directory).

## Test Structure

```
tests/
├── common/
│   └── mod.rs              # Shared test utilities
├── configuration_tests.rs   # Configuration loading tests
├── rule_matching_tests.rs   # Rule matching logic tests
├── intermediary_tests.rs    # Request/response conversion tests
├── plugin_tests.rs          # Plugin system tests
└── integration_tests.rs     # End-to-end integration tests

src/
└── configuration/rule/
    ├── when.rs              # Unit tests for 'when' clause
    ├── then.rs              # Unit tests for 'then' clause
    └── with.rs              # Unit tests for 'with' clause
```

## Running Tests

### Run All Tests
```bash
cargo test
```

### Run Specific Test Suite
```bash
# Configuration tests
cargo test --test configuration_tests

# Rule matching tests
cargo test --test rule_matching_tests

# Integration tests
cargo test --test integration_tests

# Plugin tests
cargo test --test plugin_tests
```

### Run Tests with Output
```bash
cargo test -- --nocapture
```

### Run Specific Test
```bash
cargo test test_rule_should_apply_uri_match
```

### Run Tests in Parallel
```bash
cargo test -- --test-threads=4
```

## Test Coverage

### Configuration Module
- ✅ YAML file loading from directories
- ✅ Rule deserialization
- ✅ Extension filtering
- ✅ Error handling for invalid paths

### Rule Matching
- ✅ URI pattern matching (single and multiple)
- ✅ HTTP method matching
- ✅ Body content matching
- ✅ Probability-based matching
- ✅ Combined condition matching

### Intermediary Conversions
- ✅ Request to Intermediary conversion
- ✅ Response to Intermediary conversion
- ✅ Intermediary to Request conversion
- ✅ Intermediary to Response conversion
- ✅ Header preservation
- ✅ Status code handling

### Plugin System
- ✅ Function invocation with arguments
- ✅ Argument validation
- ✅ Error handling
- ✅ Help text support

### Integration Tests
- ✅ Full configuration loading pipeline
- ✅ Rule container matching
- ✅ Multiple rules precedence
- ✅ Concurrent rule evaluation
- ✅ Async conversions
- ✅ Error handling pipeline

## Code Coverage

To generate code coverage reports (requires `cargo-tarpaulin`):

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Generate coverage
cargo tarpaulin --out Html --output-dir coverage

# View coverage
open coverage/index.html
```

## Writing New Tests

### Unit Tests
Add tests directly in source files:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Test implementation
    }
}
```

### Integration Tests
Create new test file in `tests/`:

```rust
mod common;

#[test]
fn test_integration_feature() {
    // Test implementation
}
```

### Async Tests
Use `tokio::test` for async tests:

```rust
#[tokio::test]
async fn test_async_feature() {
    // Async test implementation
}
```

## Test Best Practices

1. **Keep tests focused**: Each test should verify one specific behavior
2. **Use descriptive names**: Test names should clearly describe what they test
3. **Minimize dependencies**: Use mocks and test doubles where appropriate
4. **Test edge cases**: Include tests for boundary conditions and error cases
5. **Keep tests fast**: Integration tests should complete quickly
6. **Use common utilities**: Leverage `tests/common/mod.rs` for shared code

## Continuous Integration

Tests run automatically on:
- Every push to feature branches
- Pull requests to main
- Release tags

CI configuration checks:
- All tests pass
- No compiler warnings
- Code formatting (rustfmt)
- Linting (clippy)

## Performance Testing

For performance benchmarks:

```bash
# Run with release optimizations
cargo test --release

# Time specific test
time cargo test test_concurrent_rule_evaluation -- --nocapture
```

## Troubleshooting

### Tests Fail Locally
1. Ensure `nconfig-test/` directory exists with valid configuration
2. Check that plugins are built (if testing plugin functionality)
3. Verify Rust toolchain version matches project requirements

### Tests Timeout
1. Increase timeout in test:
   ```rust
   use tokio::time::{timeout, Duration};
   timeout(Duration::from_secs(10), async_operation()).await
   ```

### Plugin Tests Fail
1. Build test plugins first: `./scripts/build_plugin.sh`
2. Check plugin paths in test configuration
3. Verify plugin library extension (.so on Linux, .dylib on macOS)

## Test Metrics

Current test coverage targets:
- Configuration module: 85%+
- Rule matching: 90%+
- Intermediary conversions: 85%+
- Plugin system: 80%+
- Overall project: 80%+

Run `cargo test` to execute all tests:
```bash
running 45 tests
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured
```

## Future Enhancements

- [ ] Property-based testing with `proptest`
- [ ] Fuzz testing for parser
- [ ] Load testing for server endpoints
- [ ] Mutation testing with `cargo-mutants`
- [ ] Performance regression tests

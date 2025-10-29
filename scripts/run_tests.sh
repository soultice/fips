#!/bin/bash

# FIPS Test Runner Script
# Runs tests with various options and generates reports

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Print colored output
print_status() {
    echo -e "${GREEN}[✓]${NC} $1"
}

print_error() {
    echo -e "${RED}[✗]${NC} $1"
}

print_info() {
    echo -e "${YELLOW}[i]${NC} $1"
}

# Default values
RUN_COVERAGE=false
RUN_BENCHMARKS=false
VERBOSE=false
TEST_PATTERN=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -c|--coverage)
            RUN_COVERAGE=true
            shift
            ;;
        -b|--benchmark)
            RUN_BENCHMARKS=true
            shift
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -t|--test)
            TEST_PATTERN="$2"
            shift 2
            ;;
        -h|--help)
            echo "FIPS Test Runner"
            echo ""
            echo "Usage: ./run_tests.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -c, --coverage      Generate coverage report (requires cargo-tarpaulin)"
            echo "  -b, --benchmark     Run benchmarks"
            echo "  -v, --verbose       Show verbose output"
            echo "  -t, --test PATTERN  Run specific test pattern"
            echo "  -h, --help          Show this help message"
            echo ""
            echo "Examples:"
            echo "  ./run_tests.sh                    # Run all tests"
            echo "  ./run_tests.sh -c                 # Run with coverage"
            echo "  ./run_tests.sh -t rule_matching   # Run rule matching tests"
            echo "  ./run_tests.sh -v                 # Run with verbose output"
            exit 0
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use -h or --help for usage information"
            exit 1
            ;;
    esac
done

print_info "FIPS Test Suite Runner"
echo ""

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the project root."
    exit 1
fi

# Build project first
print_info "Building project..."
if cargo build --quiet; then
    print_status "Build successful"
else
    print_error "Build failed"
    exit 1
fi

# Run tests
print_info "Running tests..."
echo ""

if [ -n "$TEST_PATTERN" ]; then
    print_info "Running tests matching pattern: $TEST_PATTERN"
    if $VERBOSE; then
        cargo test "$TEST_PATTERN" -- --nocapture
    else
        cargo test "$TEST_PATTERN"
    fi
else
    if $VERBOSE; then
        cargo test -- --nocapture
    else
        cargo test
    fi
fi

TEST_EXIT_CODE=$?

if [ $TEST_EXIT_CODE -eq 0 ]; then
    echo ""
    print_status "All tests passed!"
else
    echo ""
    print_error "Some tests failed"
    exit $TEST_EXIT_CODE
fi

# Run coverage if requested
if $RUN_COVERAGE; then
    echo ""
    print_info "Generating coverage report..."
    
    if ! command -v cargo-tarpaulin &> /dev/null; then
        print_error "cargo-tarpaulin not found"
        print_info "Install with: cargo install cargo-tarpaulin"
        exit 1
    fi
    
    cargo tarpaulin --out Html --output-dir coverage --exclude-files 'tests/*' --exclude-files 'build.rs'
    
    if [ $? -eq 0 ]; then
        print_status "Coverage report generated: coverage/index.html"
        
        # Try to open coverage report
        if command -v open &> /dev/null; then
            open coverage/index.html
        elif command -v xdg-open &> /dev/null; then
            xdg-open coverage/index.html
        fi
    else
        print_error "Coverage generation failed"
        exit 1
    fi
fi

# Run benchmarks if requested
if $RUN_BENCHMARKS; then
    echo ""
    print_info "Running benchmarks..."
    cargo test --release -- --ignored --nocapture
    
    if [ $? -eq 0 ]; then
        print_status "Benchmarks completed"
    else
        print_error "Benchmarks failed"
        exit 1
    fi
fi

echo ""
print_status "Test run complete!"

# Summary
echo ""
echo "Test Summary:"
echo "-------------"
cargo test -- --list | grep -c "test" | xargs -I {} echo "Total tests: {}"

if $RUN_COVERAGE; then
    echo "Coverage report: coverage/index.html"
fi

exit 0

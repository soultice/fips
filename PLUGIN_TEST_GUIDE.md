# FIPS Plugin Test - Setup & Execution Guide

## Created Plugin: test_timestamp

A demonstration plugin with three functions to test the FIPS plugin system after the hyper 1.x upgrade.

## Quick Start

### 1. Build the Plugin

```bash
cd /Users/FPFINGS/work/02_pet_projects/fips/plugins/test_timestamp
cargo build --release
```

This creates: `target/release/libtest_timestamp.dylib` (macOS) or `.so` (Linux)

### 2. Verify Plugin Configuration

The plugin is already configured in: `nconfig-test/rule-plugin-test.nrule.yml`

Two test endpoints are configured:
- `/plugin-test` - Tests all three plugin functions
- `/dynamic` - Tests dynamic timestamp and reverse functions

### 3. Start FIPS Server

```bash
cd /Users/FPFINGS/work/02_pet_projects/fips
cargo run -- -c ./nconfig-test/
```

### 4. Test the Plugin

In a new terminal:

```bash
# Test endpoint with all functions
curl http://localhost:8888/plugin-test | jq

# Test dynamic endpoint
curl http://localhost:8888/dynamic | jq

# Check headers with timestamp
curl -i http://localhost:8888/dynamic

# Or run the comprehensive test suite
./test_plugin.sh
```

## Expected Results

### /plugin-test endpoint:
```json
{
  "message": "Hello from plugin!",
  "timestamp": "2025-10-29 15:30:45 UTC",
  "uppercase": "HELLO WORLD",
  "reversed": "SPIF"
}
```

### /dynamic endpoint:
```json
{
  "generated_at": "2025-10-29 15:30:47 UTC",
  "request_id": "54321-QER"
}
```

With header:
```
X-Generated: 2025-10-29 15:30:47 UTC
```

## Plugin Functions

1. **{{Timestamp}}** - Returns current UTC timestamp
   - Args: Optional format string (default: "%Y-%m-%d %H:%M:%S UTC")
   
2. **{{Uppercase}}** - Converts string to uppercase
   - Args: String to convert
   
3. **{{Reverse}}** - Reverses a string
   - Args: String to reverse

## What This Tests

✅ Plugin loading with hyper 1.x upgrade
✅ Multiple plugins per rule (fixed implementation)
✅ Plugin function registration
✅ Placeholder replacement in response bodies
✅ Plugin arguments passing
✅ Multiple functions from same plugin library
✅ Frontend/backend plugin registry synchronization

## Troubleshooting

### Plugin not loading
- Check the path in `rule-plugin-test.nrule.yml` matches your OS (.dylib vs .so)
- Ensure plugin is built: `ls -la plugins/test_timestamp/target/release/libtest_timestamp.*`
- Check FIPS server logs for plugin loading errors

### Placeholders not replaced ({{Timestamp}} appears in response)
- Plugin functions may not be registered
- Check plugin is in the correct rule's `with.plugins` section
- Verify plugin path is absolute or relative from FIPS binary location

### Server won't start
- Check Rust version compatibility (plugin uses same RUSTC_VERSION as fips)
- Rebuild both fips and plugin with same toolchain

## Files Created

1. **Plugin source**: `plugins/test_timestamp/src/lib.rs`
2. **Plugin config**: `plugins/test_timestamp/Cargo.toml`
3. **Rule config**: `nconfig-test/rule-plugin-test.nrule.yml`
4. **Build script**: `build_plugin.sh`
5. **Test script**: `test_plugin.sh`
6. **Documentation**: `plugins/test_timestamp/README.md`

## Next Steps

After successful testing:
1. Create more complex plugins for your use cases
2. Add error handling and validation
3. Implement plugins with external API calls
4. Create plugins that transform request/response data
5. Build plugins with database access or file I/O

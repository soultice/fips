# 1.0.1 (2025-11-12)

### Features
- Add comprehensive request correlation ID system for easier log tracking
- Add detailed logging for rule evaluation with per-criterion decision tracking
- Add `stripPath` option for static file serving to strip directory paths
- Add parse error visibility in UI with file path and error details
- Display actual configured port and config paths in runtime messages and UI

### Improvements
- Enhanced logging output with correlation IDs across all request lifecycle stages
- Improved error handling to accumulate parse errors instead of failing fast
- Better visibility of configuration issues in terminal UI

### Bug Fixes
- Fixed runtime display messages to show actual configured values instead of placeholders

# 0.2.0

- implement new rule system
- change from packages to modules due to rust module system

# 0.0.1

- Instead of implicitly determining the modus operandi, rules now need to explicitly say what they want to do
- Add logging as a cargo feature
- Add UI mode as a cargo feature (default is enabled)
- Add possibility to generate json-schema for config files
- Add gradient to borders


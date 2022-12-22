# Fake Injecting Proxy Server - in short Fips - fake and proxy within seconds

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## About

Fips provides three different functionalities: It can function as a Fake data server, it can function as a simple proxy server, and it can be a mixture of both, manipulating responses on the fly - defined by your own rules. As such, Fips is best used if you wish to quickly setup an endpoint and test it in your application - the backend work is currently blocked? No problem. Start the application and host a mock endpoint while proxying the remainders of your endpoints to the actual backend.

## Installation

Due to Daimler internals, binaries can currently not be provided. If you wish to use Fips, build it from the sources:

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
<kdb>Tab</kdb> Go to next Tab
<kdb>Shift</kdb>Tab</kdb> Go to previous Tab
<kdb>c</kdb> clear the log output
<kdb>r</kdb> reload config files
<kdb>Esc</kdb> quit

## Usage

Fipss configuration is placed in `.yaml` or `.yml` files. They are loaded at startup from the `--config` directory.
For each request, Fips will check against the configuration files if any config object matches the current request URI.
If it does, one of the three modes do apply implicitly by the configuration given.

| Mode              | forwardUri | rules | serveStatic |
| -----             | :--------: | ----- | ----------- |
| Fips              |     ✔ ️     |   ✔   |     ❌      |
| proxy             |     ✔ ️     |   ❌  |     ❌      |
| mock              |     ❌     |   ✔   |     ❌      |
| static (fallback) |     ❌     |   ❌  |      ✔      | 

Meaning if you've set `forwardUri` but havent set any `rules`, then Fips will function as a proxy server.

❗ **Things to keep in mind for your config:**

- Fips uses regex to match against paths. `/foo/bar` in a config path will also match for `/foo/bar/baz`, so you need to be as explicit as possible if you care.
- If multiple rules match, only the first matching rule will apply.
- Rules are applied in the order they appear - so order matters!.
- The config file is not checked for spelling, the server will panic if it is unable to read a configuration file due to spelling errors.
- Object manipulation uses the [dotpath crate](dotpath). The syntax is noted below.
- If you wish to mock a request without any body (e.g. only the response status matters) you still need to provide the `rules` Array, however you can leave it empty:
  ```yaml
  path: ^/foo/$
  rules: []
  ```

## Example configuration in `config.yaml`

See also the `examples` directory for more example configurations.

1. Any request arriving at Fips with the URI `/foo/bar` will return `['this is a lot of fun']`

```yaml
- path: ^/foo/bar/$
  rules:
    - path: 
      item: ['this is a lot of fun']
```

2. Any request against `/foo/*anything*/bar/` will be proxied to the server at `localhost:4041`, the `Authorization` Header will be forwarded

```yaml
- path: ^/foo/.*/bar/$
  forwardUri: 'http://localhost:4041'
  forwardHeaders:
    - 'Authorization'
```

2. Any request against `/foo/*anything*/bar/` will be proxied to the server at `localhost:4041`, the `content-type` Header will be returned. Then we append another `user` to our response.

```yaml
- path: ^/foo/.*/bar/$
  forwardUri:
    "http://localhost:4041"
  backwardHeaders:
    - "content-type"
  rules:
    - path: ">>"
    {
      "firstname": "Morty",
      "lastname": "Smith",
      "status": "cloned himself"
    }
```

## All configuration parameters 

Main configuration:
```yaml
    # A a regex to match incoming requests. if a match is found, this rule will be applied
    path: String
    # This name will be displayed for debugging purposes
    name: String
    # Fips will change the response status to this value
    responseStatus: u16,
    # Sleep for ms
    sleep: u64
    # Forward any incoming request to this uri and return the response
    forwardUri: String
    # Forward matching headers on the request
    forwardHeaders: Vec<String>
    # Return these headers from the original response
    backwardHeaders: Vec<String>,
    # Add these headers to the response
    headers: HashMap<String, String>,
    # Only apply a rule if the method matches these
    matchMethods: Vec<String>
    # Only apply a rule with this probability. It's best to have a fallback rule defined
    matchProbability: Option<f32>
    # Only apply a rule if the request body contains the given string
    matchBodyContains: Option<String>
    # Transform the Response according to these rules
    rules: Vec<Rule>,
```
Rule:
```yaml
   # The json_dotpath (see more at Object manipulation on the response)
   path: String,
   # Any json serializeable item that is added to the response at the paths location
   item: Serde<Value>
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
    registrar.register_function("{{Name}}", Box::new(Random));
}
```

Above code registers the plugin on the Fips plugin registry.  The plugins `name` `{{Name}}` will be matched when a matching rule is found, the `json serializeable(!)` return value will be used to replace your pattern in the matching rule.

Example `config.yaml`

```yaml
- path: ^/randomname/$
  rules:
      item:
        foo: '{{Name}}'
```

Example output of `curl localhost:8888/randomname/ | jq`

```json
{
  "foo": {
    "bar": ["Ms. Destiney Metz"]
  }
}
```

Plugins can also be passed arguments via the configuration files. If you wish to do so, the plugin has to be configured as an object in your configuration yaml:

```yaml
- path: ^/randomname/$
  rules:
      item:
        foo:
          plugin: '{{Name}}',
          args: [ "foo", 1, "bar" ]
```

Example output of `curl localhost:8888/randomname/ | jq`

```json
{
  "foo": {
    "bar": ["Ms. Destiney Metz"]
  }
}
```

## Contributing 

Please read the [Contributing File](CONTRIBUTING.md)

## License 

This Project is Licensed under the [MIT License](LICENSE)

[cargo]: https://doc.rust-lang.org/cargo/getting-started/installation.html
[dotpath]: https://crates.io/crates/json_dotpath


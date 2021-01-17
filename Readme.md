# Moxy - mock and proxy within seconds

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## About

Moxy provides three different functionalities: It can function as a Mock server, it can function as a simple proxy server, and it can be a mixture of both, manipulating responses on the fly - defined by your own rules. As such, moxy is best used if you wish to quickly setup an endpoint and test it in your application - the backend work is currently blocked? No problem. Start the moxy application and host a mock endpoint while proxying the remainders of your endpoints to the actual backend.

## Installation

Binaries for Linux and Windows are attached to each release on the release page. If you wish you can also build it from the sources:

- Install [rust and cargo](cargo).
- Checkout this repo.
- Dir into it and run `cargo run` - or `cargo build` if you wish to produce an executable.

## Usage

Moxys configuration is placed alongside the executable in a `config.yaml` file.
For each request, moxy will check against the `config.yaml` if any config object matches the current request URI.
If it does, one of the three modes do apply implicitly by the configuration given.

| Mode  | forwardUri | rules |
| ----- | :--------: | ----- |
| moxy  |     ✔️     | ✔️    |
| proxy |     ✔️     | ❌    |
| mock  |     ❌     | ✔️    |

Meaning if you've set `forwardUri` but havent set any `rules`, then moxy will function as a proxy server.

❗ **Things to keep in mind for your config:**

- Moxy uses regex to match against paths. `/foo/bar` in a config path will also match for `/foo/bar/baz`, so you need to be as explicit as possible if you care.
- If multiple rules match, only the first rule will apply.
- Rules are applied in the order they appear - so order matters.
- The config file is not checked for spelling.
- Object manipulation uses the [dotpath crate](dotpath). The syntax is noted below.

## Example configuration in `config.yaml`

See also the `examples` directory for more example configurations.

1. Any request arriving at moxy with the URI `/foo/bar` will return `['this is a lot of fun']`

```yaml
- path: ^/foo/bar/$
  rules:
    - path: ''
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

## All configuration possibilities TBD

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

## Faker TBD

## Extension

One of moxys key features is its extension system. Moxy exports a rust macro `export_plugin`.
Your extension can make use of this macro to register a plugin.
The plugins name will be matched against your configuration. If a match occurs, the pattern will be replaced
with the output of your plugin. All plugins matching your OS in the `plugins` directory relative to the moxy binary will be loaded automatically at startup.

Example plugin implementation:

```rust
use moxy;
use moxy::{PluginRegistrar, Function, InvocationError};

pub struct Random;

impl Function for Random {
    fn call(&self, args: &[f64]) -> Result<String, InvocationError> {
        // do something here -e.g. make a request against an api to retrieve a value.
        Ok("{\"foo\" : [\"bar\"]}".to_owned())
    }
}

moxy::export_plugin!(register);

extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_function("{{UUID}}", Box::new(Random));
}
```
Above code registers the plugin on the moxy plugin registry.  The name `{{UUID}}` will be matched when a matching rule is found, the `json serializeable(!)` return value will be used to replace your pattern in the matching rule.

Example `config.yaml`

```yaml
- path: ^/final/$
  rules:
    - path: ''
      item:
        foo: '{{UUID}}'
```

Example output of `curl localhost:8888 | jq`

```json
{
  "foo": {
    "foo": ["bar"]
  }
}
```

## Contribution TBD

## License TBD

[cargo]: https://doc.rust-lang.org/cargo/getting-started/installation.html
[dotpath]: https://crates.io/crates/json_dotpath

use crate::plugin_registry::InvocationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Could not parse URI: {0}")]
    UriParseError(#[from] hyper::http::uri::InvalidUri),
    #[error("Could not format String: {0}")]
    StringFromatError(#[from] std::fmt::Error),
    #[error("Invalid Body in rule: {0}")]
    InvalidBodyError(#[from] http::Error),
    #[error("Config has no uri")]
    NoUriError,
    #[error("Config has no method")]
    NoMethodError,
    #[error("Not forwarding as per rule definition")]
    NotForwarding,
    #[error("Plugin invocation error: {0}")]
    PluginInvocation(#[from] InvocationError),
    #[error("Plugin not found error")]
    PluginNotFound,
    #[error("could not parse yaml")]
    YamlParse(#[from] serde_yaml::Error),
    #[error("could not parse json")]
    JsonParse(#[from] serde_json::Error),
    #[error("std error")]
    Std(#[from] std::io::Error),
    #[error("hyper lib error")]
    Hyper(#[from] hyper::Error),
    #[error("rule does not match")]
    RuleDoesNotMatch,
}

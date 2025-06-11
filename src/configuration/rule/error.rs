use crate::plugin_registry::InvocationError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigurationError {
    #[error("Could not parse URI: {0}")]
    UriParseError(#[from] hyper::http::uri::InvalidUri),
    #[error("Could not parse URI: {0}")]
    MalformedUri(String),
    #[error("Could not parse method: {0}")]
    MalformedMethod(String),
    #[error("Could not parse header: {0}")]
    MalformedHeader(String),
    #[error("Could not format String: {0}")]
    StringFormatError(#[from] std::fmt::Error),
    #[error("Invalid probability value: {0}")]
    InvalidProbability(f64),
    #[error("Invalid Body in rule: {0}")]
    InvalidBodyError(#[from] hyper::http::Error),
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
    #[error("Could not parse YAML: {0}")]
    YamlError(#[from] serde_yaml::Error),
    #[error("Could not parse JSON: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Hyper error: {0}")]
    HyperError(#[from] hyper::Error),
    #[error("Rule does not match")]
    RuleDoesNotMatch,
    #[error("Generic error: {0}")]
    Generic(String),
}

impl From<eyre::Report> for ConfigurationError {
    fn from(err: eyre::Report) -> Self {
        ConfigurationError::Generic(err.to_string())
    }
}

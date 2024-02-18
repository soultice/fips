pub mod plugin;
pub use plugin::ExternalFunctions;
use serde_json::Value;
use thiserror::Error;

pub trait Function {
    fn call(&self, args: Value) -> Result<String, InvocationError>;

    /// Help text that may be used to display information about this function.
    fn help(&self) -> Option<&str> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum InvocationError {
    #[error("Invalid argument count: expected {expected}, found {found}")]
    InvalidArgumentCount { expected: usize, found: usize },
    #[error("Plugin Error: {msg}")]
    Other { msg: String },
}

pub struct PluginDeclaration {
    pub rustc_version: &'static str,
    pub core_version: &'static str,
    pub register: unsafe extern "C" fn(&mut dyn PluginRegistrar),
}

pub trait PluginRegistrar {
    fn register_function(&mut self, name: &str, function: Box<dyn Function + Send>);
}


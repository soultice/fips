use super::{Function, InvocationError, PluginDeclaration};
use libloading::Library;

use serde_json::Value;
use std::collections::HashMap;
use std::ffi::OsStr;

use std::fmt::Debug;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::io;

pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

pub struct FunctionProxy {
    function: Arc<Box<dyn Function + Send>>,
    _lib: Arc<Library>,
}

impl Function for FunctionProxy {
    fn call(&self, args: Vec<Value>) -> Result<String, InvocationError> {
        self.function.call(args)
    }

    fn help(&self) -> Option<&str> {
        self.function.help()
    }
}

#[derive(Default, Clone)]
pub struct ExternalFunctions {
    pub functions: Arc<Mutex<HashMap<String, FunctionProxy>>>,
    libraries: Vec<Arc<Library>>,
}

unsafe impl Send for ExternalFunctions {}
unsafe impl Sync for ExternalFunctions {}

impl Debug for ExternalFunctions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExternalFunctions")
            .field("functions", &self.functions.lock().unwrap().keys())
            .finish()
    }
}

impl ExternalFunctions {
    pub fn new(path_to_plugin: &PathBuf) -> ExternalFunctions {
        let mut default = ExternalFunctions::default();
        default.load_from_file(path_to_plugin).unwrap();
        default
    }

    /// Load a plugin library and add all contained functions to the internal
    /// function table.
    ///
    /// # Safety
    ///
    /// A plugin library **must** be implemented using the
    /// [`plugins_core::plugin_declaration!()`] macro. Trying manually implement
    /// a plugin without going through that macro will result in undefined
    /// behaviour.
    pub unsafe fn load<P: AsRef<OsStr>>(
        &mut self,
        library_path: P,
    ) -> io::Result<()> {
        // load the library into memory
        let library = Arc::new(Library::new(library_path).unwrap()); //?);

        // get a pointer to the plugin_declaration symbol.
        let decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")
            .unwrap() //?
            .read();

        // version checks to prevent accidental ABI incompatibilities
        if decl.rustc_version != RUSTC_VERSION
            || decl.core_version != CORE_VERSION
        {
            return Err(io::Error::new(io::ErrorKind::Other, "Version mismatch"));
        }

        let mut registrar = PluginRegistrar::new(Arc::clone(&library));

        (decl.register)(&mut registrar);

        // add all loaded plugins to the functions map
        self.functions.lock().unwrap().extend(registrar.functions);
        // and make sure ExternalFunctions keeps a reference to the library
        self.libraries.push(library);

        Ok(())
    }

    pub fn load_from_file(&mut self, plugin_path: &PathBuf) -> io::Result<()> {
        #[cfg(feature = "enablelog")]
        log::info!("Loading plugin from {:?}", plugin_path);

        unsafe {
            self.load(plugin_path).expect("Function loading failed");
        }
        Ok(())
    }

    pub fn call(
        &self,
        function: &str,
        arguments: Vec<Value>,
    ) -> Result<String, InvocationError> {
        self.functions
            .lock().unwrap()
            .get(function)
            .ok_or_else(|| format!("\"{function}\" not found"))?
            .call(arguments)
    }

    pub fn has(&self, key: &str) -> bool {
        self.functions.lock().unwrap().contains_key(key)
    }

}

struct PluginRegistrar {
    functions: HashMap<String, FunctionProxy>,
    lib: Arc<Library>,
}

impl PluginRegistrar {
    fn new(lib: Arc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            lib,
            functions: HashMap::default(),
        }
    }
}

impl super::PluginRegistrar for PluginRegistrar {
    fn register_function(
        &mut self,
        name: &str,
        function: Box<dyn Function + Send>,
    ) {
        let proxy = FunctionProxy {
            function: function.into(),
            _lib: Arc::clone(&self.lib),
        };
        self.functions.insert(name.to_string(), proxy);
    }
}

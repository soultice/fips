use libloading::Library;
use moxy::{Function, InvocationError, PluginDeclaration};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::io;
use std::rc::Rc;
use std::sync::Arc;

pub struct FunctionProxy {
    function: Box<dyn Function + Send>,
    _lib: Arc<Library>,
}

impl Function for FunctionProxy {
    fn call(&self, args: &[f64]) -> Result<String, InvocationError> {
        self.function.call(args)
    }

    fn help(&self) -> Option<&str> {
        self.function.help()
    }
}

#[derive(Default)]
pub struct ExternalFunctions {
    functions: HashMap<String, FunctionProxy>,
    libraries: Vec<Arc<Library>>,
}

impl ExternalFunctions {
    pub fn new() -> ExternalFunctions {
        ExternalFunctions::default()
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
    pub unsafe fn load<P: AsRef<OsStr>>(&mut self, library_path: P) -> io::Result<()> {
        // load the library into memory
        let library = Arc::new(Library::new(library_path).unwrap()); //?);

        // get a pointer to the plugin_declaration symbol.
        let decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")
            .unwrap() //?
            .read();

        // version checks to prevent accidental ABI incompatibilities
        /*        if decl.rustc_version != plugins_core::RUSTC_VERSION
            || decl.core_version != plugins_core::CORE_VERSION
        {
            return Err(io::Error::new(io::ErrorKind::Other, "Version mismatch"));
        }*/

        let mut registrar = PluginRegistrar::new(Arc::clone(&library));

        (decl.register)(&mut registrar);

        // add all loaded plugins to the functions map
        self.functions.extend(registrar.functions);
        // and make sure ExternalFunctions keeps a reference to the library
        self.libraries.push(library);

        Ok(())
    }

    pub fn call(&self, function: &str, arguments: &[f64]) -> Result<String, InvocationError> {
        self.functions
            .get(function)
            .ok_or_else(|| format!("\"{}\" not found", function))?
            .call(arguments)
    }

    pub fn has(&self, key: &str) -> bool {
        self.functions.contains_key(key)
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

impl moxy::PluginRegistrar for PluginRegistrar {
    fn register_function(&mut self, name: &str, function: Box<dyn Function + Send>) {
        let proxy = FunctionProxy {
            function,
            _lib: Arc::clone(&self.lib),
        };
        self.functions.insert(name.to_string(), proxy);
    }
}
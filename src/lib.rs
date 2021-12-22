pub use plugin_registry::{PluginRegistrar, PluginDeclaration, InvocationError, Function};

#[macro_export]
macro_rules! export_plugin {
    ($register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static plugin_declaration: $crate::PluginDeclaration = $crate::PluginDeclaration {
            register: $register,
        };
    };
}

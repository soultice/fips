pub mod plugin_registry;
pub use plugin_registry::{InvocationError, PluginDeclaration, PluginRegistrar, Function};
pub use plugin_registry::plugin::{RUSTC_VERSION, CORE_VERSION};

#[macro_export]
macro_rules! export_plugin {
    ($register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static plugin_declaration: $crate::PluginDeclaration =
            $crate::PluginDeclaration {
                rustc_version: $crate::RUSTC_VERSION,
                core_version: $crate::CORE_VERSION,
                register: $register,
            };
    };
}

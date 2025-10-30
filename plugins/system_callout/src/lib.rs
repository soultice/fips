use fips::{export_plugin, Function, InvocationError, PluginRegistrar};
use serde_json::Value;
use std::process::Command;

/// ⚠️ WARNING: SECURITY CRITICAL PLUGIN ⚠️
/// 
/// This plugin demonstrates the power and danger of system callouts in FIPS plugins.
/// 
/// SECURITY CONSIDERATIONS:
/// 1. Arbitrary command execution - can run ANY system command
/// 2. No sandboxing - full access to file system and network
/// 3. Inherits server permissions - runs as same user as FIPS
/// 4. Can exfiltrate data - read files, make network requests
/// 5. Can modify system - write files, change configurations
/// 
/// USE ONLY IN CONTROLLED ENVIRONMENTS!
/// DO NOT expose to untrusted input or production systems without proper security review.

struct SystemCommand;

impl Function for SystemCommand {
    fn call(&self, args: Value) -> Result<String, InvocationError> {
        let args_array = args.as_array().ok_or(InvocationError::Other {
            msg: "Arguments must be an array [command, arg1, arg2, ...]".to_string(),
        })?;

        if args_array.is_empty() {
            return Err(InvocationError::Other {
                msg: "Command required as first argument".to_string(),
            });
        }

        let command = args_array[0].as_str().ok_or(InvocationError::Other {
            msg: "Command must be a string".to_string(),
        })?;

        let command_args: Vec<&str> = args_array[1..]
            .iter()
            .filter_map(|v| v.as_str())
            .collect();

        // ⚠️ DANGEROUS: Executing arbitrary system command
        let output = Command::new(command)
            .args(&command_args)
            .output()
            .map_err(|e| InvocationError::Other {
                msg: format!("Failed to execute command: {}", e),
            })?;

        if output.status.success() {
            String::from_utf8(output.stdout).map_err(|e| InvocationError::Other {
                msg: format!("Invalid UTF-8 in command output: {}", e),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(InvocationError::Other {
                msg: format!("Command failed: {}", stderr),
            })
        }
    }

    fn help(&self) -> Option<&str> {
        Some("⚠️ DANGEROUS: Executes system commands. Args: [command, arg1, arg2, ...]")
    }
}

struct GetEnvVar;

impl Function for GetEnvVar {
    fn call(&self, args: Value) -> Result<String, InvocationError> {
        let args_array = args.as_array().ok_or(InvocationError::Other {
            msg: "Arguments must be an array [var_name]".to_string(),
        })?;

        if args_array.is_empty() {
            return Err(InvocationError::Other {
                msg: "Environment variable name required".to_string(),
            });
        }

        let var_name = args_array[0].as_str().ok_or(InvocationError::Other {
            msg: "Variable name must be a string".to_string(),
        })?;

        // ⚠️ Can expose sensitive environment variables (API keys, secrets, etc.)
        std::env::var(var_name).map_err(|e| InvocationError::Other {
            msg: format!("Environment variable not found: {}", e),
        })
    }

    fn help(&self) -> Option<&str> {
        Some("⚠️ SENSITIVE: Reads environment variables. Args: [var_name]")
    }
}

struct ReadFile;

impl Function for ReadFile {
    fn call(&self, args: Value) -> Result<String, InvocationError> {
        let args_array = args.as_array().ok_or(InvocationError::Other {
            msg: "Arguments must be an array [file_path]".to_string(),
        })?;

        if args_array.is_empty() {
            return Err(InvocationError::Other {
                msg: "File path required".to_string(),
            });
        }

        let file_path = args_array[0].as_str().ok_or(InvocationError::Other {
            msg: "File path must be a string".to_string(),
        })?;

        // ⚠️ DANGEROUS: Can read ANY file the server has access to
        // Including: /etc/passwd, ~/.ssh/id_rsa, application secrets, etc.
        std::fs::read_to_string(file_path).map_err(|e| InvocationError::Other {
            msg: format!("Failed to read file: {}", e),
        })
    }

    fn help(&self) -> Option<&str> {
        Some("⚠️ DANGEROUS: Reads files from disk. Args: [file_path]")
    }
}

struct HttpRequest;

impl Function for HttpRequest {
    fn call(&self, args: Value) -> Result<String, InvocationError> {
        let args_array = args.as_array().ok_or(InvocationError::Other {
            msg: "Arguments must be an array [url]".to_string(),
        })?;

        if args_array.is_empty() {
            return Err(InvocationError::Other {
                msg: "URL required".to_string(),
            });
        }

        let url = args_array[0].as_str().ok_or(InvocationError::Other {
            msg: "URL must be a string".to_string(),
        })?;

        // ⚠️ DANGEROUS: Can make HTTP requests to internal services
        // Potential for SSRF (Server-Side Request Forgery) attacks
        // Can hit internal APIs, cloud metadata endpoints, etc.
        let output = Command::new("curl")
            .args(&["-s", url])
            .output()
            .map_err(|e| InvocationError::Other {
                msg: format!("Failed to execute curl: {}", e),
            })?;

        if output.status.success() {
            String::from_utf8(output.stdout).map_err(|e| InvocationError::Other {
                msg: format!("Invalid UTF-8 in response: {}", e),
            })
        } else {
            Err(InvocationError::Other {
                msg: "HTTP request failed".to_string(),
            })
        }
    }

    fn help(&self) -> Option<&str> {
        Some("⚠️ DANGEROUS: Makes HTTP requests (SSRF risk). Args: [url]")
    }
}

export_plugin!(register);

extern "C" fn register(registrar: &mut dyn PluginRegistrar) {
    registrar.register_function("SystemCommand", Box::new(SystemCommand));
    registrar.register_function("GetEnvVar", Box::new(GetEnvVar));
    registrar.register_function("ReadFile", Box::new(ReadFile));
    registrar.register_function("HttpRequest", Box::new(HttpRequest));
}

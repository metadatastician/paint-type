// SPDX-License-Identifier: AGPL-3.0-or-later

//! Error types for the paint-type plugin system.

use std::fmt;
use std::path::PathBuf;

use crate::manifest::{PluginId, PluginVersion};

/// Main error type for plugin operations.
#[derive(Debug)]
pub enum PluginError {
    /// Manifest validation failed
    ManifestValidation(String),
    /// Manifest signature verification failed
    SignatureVerificationFailed,
    /// Plugin not found in registry
    PluginNotFound(PluginId),
    /// Plugin already loaded
    PluginAlreadyLoaded(PluginId),
    /// WASM instantiation failed
    WasmInstantiationFailed(String),
    /// WASM execution failed
    WasmExecutionFailed(String),
    /// Type mismatch in plugin API
    TypeMismatch(&'static str),
    /// IO error (file not found, permission denied, etc.)
    IoError(PathBuf, String),
    /// Plugin returned an error
    PluginError(String),
    /// Unsupported plugin version
    UnsupportedVersion(PluginVersion),
    /// ML-DSA signing not available (feature not enabled)
    MlDsaNotAvailable,
}

impl fmt::Display for PluginError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginError::ManifestValidation(msg) => write!(f, "Manifest validation failed: {}", msg),
            PluginError::SignatureVerificationFailed => write!(f, "Signature verification failed"),
            PluginError::PluginNotFound(id) => write!(f, "Plugin not found: {}", id),
            PluginError::PluginAlreadyLoaded(id) => write!(f, "Plugin already loaded: {}", id),
            PluginError::WasmInstantiationFailed(msg) => {
                write!(f, "WASM instantiation failed: {}", msg)
            }
            PluginError::WasmExecutionFailed(msg) => write!(f, "WASM execution failed: {}", msg),
            PluginError::TypeMismatch(expected) => {
                write!(f, "Type mismatch: expected {}", expected)
            }
            PluginError::IoError(path, msg) => write!(f, "IO error for {}: {}", path.display(), msg),
            PluginError::PluginError(msg) => write!(f, "Plugin error: {}", msg),
            PluginError::UnsupportedVersion(v) => {
                write!(f, "Unsupported plugin version: {}", v)
            }
            PluginError::MlDsaNotAvailable => {
                write!(f, "ML-DSA signing not available (compile with --features ml-dsa)")
            }
        }
    }
}

impl std::error::Error for PluginError {}

impl PluginError {
    /// Create a manifest validation error
    pub fn manifest_validation(msg: impl Into<String>) -> Self {
        PluginError::ManifestValidation(msg.into())
    }

    /// Create an IO error
    pub fn io_error(path: impl Into<PathBuf>, msg: impl Into<String>) -> Self {
        PluginError::IoError(path.into(), msg.into())
    }

    /// Create a WASM instantiation error
    pub fn wasm_instantiation(msg: impl Into<String>) -> Self {
        PluginError::WasmInstantiationFailed(msg.into())
    }

    /// Create a WASM execution error
    pub fn wasm_execution(msg: impl Into<String>) -> Self {
        PluginError::WasmExecutionFailed(msg.into())
    }
}

/// Result type alias for plugin operations
pub type PluginResult<T> = Result<T, PluginError>;

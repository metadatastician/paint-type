// SPDX-License-Identifier: AGPL-3.0-or-later

//! WASM plugin sandbox.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::error::{PluginError, PluginResult};
use crate::manifest::{is_safe_relative_plugin_path, PluginCapability, PluginId, PluginManifest};

/// A sandboxed WASM plugin instance
pub struct WasmSandbox {
    id: PluginId,
    manifest: PluginManifest,
    granted_capabilities: Vec<PluginCapability>,
    state: HashMap<String, String>,
    loaded: bool,
}

impl WasmSandbox {
    /// Create a new sandbox for a plugin
    pub fn new(manifest: PluginManifest) -> Self {
        WasmSandbox {
            id: manifest.id.clone(),
            manifest,
            granted_capabilities: Vec::new(),
            state: HashMap::new(),
            loaded: false,
        }
    }

    /// Get the plugin ID
    pub fn id(&self) -> &PluginId {
        &self.id
    }

    /// Get the plugin manifest
    pub fn manifest(&self) -> &PluginManifest {
        &self.manifest
    }

    /// Grant a capability to the plugin
    pub fn grant_capability(&mut self, cap: PluginCapability) -> PluginResult<()> {
        if !self.manifest.has_capability(cap) {
            return Err(PluginError::manifest_validation(format!(
                "Cannot grant capability {:?} - not requested in manifest",
                cap
            )));
        }
        if !self.granted_capabilities.contains(&cap) {
            self.granted_capabilities.push(cap);
        }
        Ok(())
    }

    /// Check if the plugin has a specific capability
    pub fn has_capability(&self, cap: PluginCapability) -> bool {
        self.granted_capabilities.contains(&cap)
    }

    /// Mark the plugin loaded after checking path existence and validating the
    /// filename when it is UTF-8. Returns `ManifestValidation` for an invalid
    /// filename or `IoError` for a missing path. Does not read or execute WASM.
    pub fn load(&mut self, wasm_path: impl AsRef<Path>) -> PluginResult<()> {
        let path = wasm_path.as_ref();
        if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
            if !is_safe_relative_plugin_path(name) {
                return Err(PluginError::manifest_validation(
                    "Invalid WASM module filename",
                ));
            }
        }
        if !path.exists() {
            return Err(PluginError::io_error(path, "WASM file not found"));
        }
        self.loaded = true;
        Ok(())
    }

    /// Unload the plugin
    pub fn unload(&mut self) -> PluginResult<()> {
        if !self.loaded {
            return Ok(());
        }
        self.loaded = false;
        self.state.clear();
        Ok(())
    }

    /// Check if the plugin is loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Return a copy of `args` as a placeholder invocation result.
    ///
    /// Returns `PluginError::PluginError` if not loaded or `function` is empty.
    /// No WASM function is executed.
    pub fn invoke(&self, function: &str, args: &[u8]) -> PluginResult<Vec<u8>> {
        if !self.loaded {
            return Err(PluginError::PluginError("Plugin not loaded".to_string()));
        }
        if function.is_empty() {
            return Err(PluginError::PluginError(
                "Function name must not be empty".to_string(),
            ));
        }
        Ok(args.to_vec())
    }

    /// Get plugin state value
    pub fn get_state(&self, key: &str) -> Option<String> {
        self.state.get(key).cloned()
    }

    /// Set plugin state value
    pub fn set_state(&mut self, key: String, value: String) -> PluginResult<()> {
        self.state.insert(key, value);
        Ok(())
    }

    /// Clear plugin state
    pub fn clear_state(&mut self) {
        self.state.clear();
    }
}

impl std::fmt::Debug for WasmSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmSandbox")
            .field("id", &self.id)
            .field("loaded", &self.loaded)
            .field("granted_capabilities", &self.granted_capabilities)
            .field("state_keys", &self.state.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Thread-safe wrapper for WASM sandbox
#[derive(Clone)]
pub struct SafeWasmSandbox {
    inner: Arc<Mutex<WasmSandbox>>,
}

impl SafeWasmSandbox {
    /// Create a new thread-safe sandbox
    pub fn new(manifest: PluginManifest) -> Self {
        SafeWasmSandbox {
            inner: Arc::new(Mutex::new(WasmSandbox::new(manifest))),
        }
    }

    /// Get the plugin ID
    pub fn id(&self) -> PluginId {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .id
            .clone()
    }

    /// Load the plugin
    ///
    /// Delegates to [`WasmSandbox::load`], propagating its path errors.
    pub fn load(&self, wasm_path: impl AsRef<Path>) -> PluginResult<()> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .load(wasm_path)
    }

    /// Unload the plugin
    pub fn unload(&self) -> PluginResult<()> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unload()
    }

    /// Check if loaded
    pub fn is_loaded(&self) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_loaded()
    }

    /// Invoke a function
    ///
    /// Returns the placeholder byte echo from [`WasmSandbox::invoke`], including
    /// its unloaded-plugin and empty-function errors.
    pub fn invoke(&self, function: &str, args: &[u8]) -> PluginResult<Vec<u8>> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .invoke(function, args)
    }

    /// Grant a capability
    ///
    /// Returns `ManifestValidation` if the capability was not requested in the
    /// manifest; repeated grants succeed without adding duplicates.
    pub fn grant_capability(&self, cap: PluginCapability) -> PluginResult<()> {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .grant_capability(cap)
    }

    /// Check a capability
    pub fn has_capability(&self, cap: PluginCapability) -> bool {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .has_capability(cap)
    }
}

impl std::fmt::Debug for SafeWasmSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        f.debug_struct("SafeWasmSandbox")
            .field("id", &inner.id)
            .field("loaded", &inner.loaded)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_creation() {
        let manifest = PluginManifest {
            id: PluginId::new("com.test.plugin"),
            name: "Test Plugin".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            wasm_entry: "_start".to_string(),
            ..PluginManifest::default()
        };
        let sandbox = WasmSandbox::new(manifest);
        assert_eq!(sandbox.id().as_str(), "com.test.plugin");
        assert!(!sandbox.is_loaded());
    }

    #[test]
    fn test_sandbox_grant_capability() {
        let manifest = PluginManifest {
            id: PluginId::new("com.test.plugin"),
            name: "Test Plugin".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            wasm_entry: "_start".to_string(),
            capabilities: vec![PluginCapability::CanvasRead],
            ..PluginManifest::default()
        };
        let mut sandbox = WasmSandbox::new(manifest);
        assert!(!sandbox.has_capability(PluginCapability::CanvasRead));
        sandbox
            .grant_capability(PluginCapability::CanvasRead)
            .unwrap();
        assert!(sandbox.has_capability(PluginCapability::CanvasRead));
    }

    #[test]
    fn test_sandbox_grant_unrequested_capability() {
        let manifest = PluginManifest {
            id: PluginId::new("com.test.plugin"),
            name: "Test Plugin".to_string(),
            description: "Test".to_string(),
            author: "Test".to_string(),
            wasm_entry: "_start".to_string(),
            ..PluginManifest::default()
        };
        let mut sandbox = WasmSandbox::new(manifest);
        assert!(sandbox
            .grant_capability(PluginCapability::CanvasRead)
            .is_err());
    }
}

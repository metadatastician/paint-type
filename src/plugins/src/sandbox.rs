// SPDX-License-Identifier: AGPL-3.0-or-later

//! WASM plugin sandbox.
//!
//! This module provides a sandboxed execution environment for paint-type plugins.
//! Plugins run as WASM modules with restricted capabilities based on their
//! manifest declarations.
//!
//! Security model:
//! - Plugins run in a WASM sandbox with no direct access to host memory
//! - Communication is via typed-wasm for memory safety
//! - Capabilities are checked at runtime before each operation
//! - Plugin can only access resources explicitly granted in manifest

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::error::{PluginError, PluginResult};
use crate::manifest::{PluginCapability, PluginManifest, PluginId};

/// A sandboxed WASM plugin instance
pub struct WasmSandbox {
    /// Plugin identifier
    id: PluginId,
    /// Plugin manifest
    manifest: PluginManifest,
    /// Granted capabilities (subset of manifest.requested)
    granted_capabilities: Vec<PluginCapability>,
    /// Plugin state (for stateful plugins)
    state: HashMap<String, String>,
    /// Loaded flag
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

    /// Load the plugin WASM module
    ///
    /// In a real implementation, this would:
    /// 1. Read the WASM file
    /// 2. Instantiate it in a sandboxed environment
    /// 3. Set up the import/export interface
    /// 4. Call the initialization function
    pub fn load(&mut self, wasm_path: impl AsRef<Path>) -> PluginResult<()> {
        let path = wasm_path.as_ref();
        
        // Check if file exists
        if !path.exists() {
            return Err(PluginError::io_error(
                path,
                "WASM file not found".to_string(),
            ));
        }

        // In the real implementation, we would:
        // - Use wasmtime or similar WASM runtime
        // - Configure memory limits
        // - Set up the import object with safe host functions
        // - Instantiate the module
        // - Call the _start or plugin_init function
        
        // For now, simulate loading
        self.loaded = true;
        
        Ok(())
    }

    /// Unload the plugin
    pub fn unload(&mut self) -> PluginResult<()> {
        if !self.loaded {
            return Ok(());
        }
        
        // In real implementation:
        // - Clean up WASM instance
        // - Free memory
        // - Remove event handlers
        
        self.loaded = false;
        self.state.clear();
        
        Ok(())
    }

    /// Check if the plugin is loaded
    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    /// Invoke a plugin function
    ///
    /// This is the main entry point for calling plugin functionality.
    /// The function name and arguments are passed, and the result is returned.
    pub fn invoke(&self, function: &str, args: &[u8]) -> PluginResult<Vec<u8>> {
        if !self.loaded {
            return Err(PluginError::PluginError("Plugin not loaded".to_string()));
        }

        // In real implementation:
        // - Look up the exported function
        // - Serialize args to WASM memory
        // - Call the function
        // - Serialize result back
        // - Handle any errors
        
        // For now, simulate a successful call
        // Return the args as echo (for testing)
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
        self.inner.lock().unwrap().id.clone()
    }

    /// Load the plugin
    pub fn load(&self, wasm_path: impl AsRef<Path>) -> PluginResult<()> {
        self.inner.lock().unwrap().load(wasm_path)
    }

    /// Unload the plugin
    pub fn unload(&self) -> PluginResult<()> {
        self.inner.lock().unwrap().unload()
    }

    /// Check if loaded
    pub fn is_loaded(&self) -> bool {
        self.inner.lock().unwrap().is_loaded()
    }

    /// Invoke a function
    pub fn invoke(&self, function: &str, args: &[u8]) -> PluginResult<Vec<u8>> {
        self.inner.lock().unwrap().invoke(function, args)
    }

    /// Grant a capability
    pub fn grant_capability(&self, cap: PluginCapability) -> PluginResult<()> {
        self.inner.lock().unwrap().grant_capability(cap)
    }

    /// Check a capability
    pub fn has_capability(&self, cap: PluginCapability) -> bool {
        self.inner.lock().unwrap().has_capability(cap)
    }
}

impl std::fmt::Debug for SafeWasmSandbox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = self.inner.lock().unwrap();
        f.debug_struct("SafeWasmSandbox")
            .field("id", &inner.id)
            .field("loaded", &inner.loaded)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PluginType, PluginVersion};

    #[test]
    fn test_sandbox_creation() {
        let mut manifest = PluginManifest::default();
        manifest.id = PluginId::new("com.test.plugin".to_string());
        manifest.name = "Test Plugin".to_string();
        manifest.description = "Test".to_string();
        manifest.author = "Test".to_string();
        manifest.wasm_entry = "_start".to_string();
        
        let sandbox = WasmSandbox::new(manifest);
        assert_eq!(sandbox.id().as_str(), "com.test.plugin");
        assert!(!sandbox.is_loaded());
    }

    #[test]
    fn test_sandbox_grant_capability() {
        let mut manifest = PluginManifest::default();
        manifest.id = PluginId::new("com.test.plugin".to_string());
        manifest.name = "Test Plugin".to_string();
        manifest.description = "Test".to_string();
        manifest.author = "Test".to_string();
        manifest.wasm_entry = "_start".to_string();
        manifest.capabilities = vec![PluginCapability::CanvasRead];
        
        let mut sandbox = WasmSandbox::new(manifest);
        assert!(!sandbox.has_capability(PluginCapability::CanvasRead));
        
        sandbox.grant_capability(PluginCapability::CanvasRead).unwrap();
        assert!(sandbox.has_capability(PluginCapability::CanvasRead));
    }

    #[test]
    fn test_sandbox_grant_unrequested_capability() {
        let mut manifest = PluginManifest::default();
        manifest.id = PluginId::new("com.test.plugin".to_string());
        manifest.name = "Test Plugin".to_string();
        manifest.description = "Test".to_string();
        manifest.author = "Test".to_string();
        manifest.wasm_entry = "_start".to_string();
        // No capabilities requested
        
        let mut sandbox = WasmSandbox::new(manifest);
        let result = sandbox.grant_capability(PluginCapability::CanvasRead);
        assert!(result.is_err());
    }
}

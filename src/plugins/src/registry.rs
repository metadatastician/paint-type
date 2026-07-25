// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plugin registry - manages loaded plugins and their lifecycle.
//!
//! The registry:
//! - Maintains a collection of loaded plugins
//! - Handles plugin loading and unloading
//! - Manages plugin capabilities
//! - Provides lookup by ID
//! - Tracks plugin state

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use crate::error::{PluginError, PluginResult};
use crate::manifest::{PluginCapability, PluginId, PluginManifest, PluginType};
use crate::sandbox::SafeWasmSandbox;
use crate::effect::{EffectPlugin, WasmEffectPlugin, EffectConfig, BrightnessContrastEffect};
use crate::tool::{ToolPlugin, WasmToolPlugin, ToolConfig, BrushTool};

/// Plugin entry in the registry
#[derive(Debug, Clone)]
pub enum PluginEntry {
    /// Effect plugin
    Effect(Arc<dyn EffectPlugin + Send + Sync>),
    /// Tool plugin
    Tool(Arc<dyn ToolPlugin + Send + Sync>),
}

impl PluginEntry {
    /// Get the plugin ID
    pub fn id(&self) -> PluginId {
        match self {
            PluginEntry::Effect(p) => p.id().clone(),
            PluginEntry::Tool(p) => p.id().clone(),
        }
    }

    /// Get the plugin name
    pub fn name(&self) -> &str {
        match self {
            PluginEntry::Effect(p) => p.name(),
            PluginEntry::Tool(p) => p.name(),
        }
    }

    /// Get the plugin description
    pub fn description(&self) -> &str {
        match self {
            PluginEntry::Effect(p) => p.description(),
            PluginEntry::Tool(p) => p.description(),
        }
    }

    /// Get the plugin type
    pub fn plugin_type(&self) -> PluginType {
        match self {
            PluginEntry::Effect(_) => PluginType::Effect,
            PluginEntry::Tool(_) => PluginType::Tool,
        }
    }

    /// Check if the plugin has a specific capability
    pub fn has_capability(&self, cap: PluginCapability) -> bool {
        match self {
            PluginEntry::Effect(p) => p.required_capabilities().contains(&cap),
            PluginEntry::Tool(p) => p.required_capabilities().contains(&cap),
        }
    }

    /// Get required capabilities
    pub fn required_capabilities(&self) -> &[PluginCapability] {
        match self {
            PluginEntry::Effect(p) => p.required_capabilities(),
            PluginEntry::Tool(p) => p.required_capabilities(),
        }
    }
}

/// Plugin registry - manages all loaded plugins
pub struct PluginRegistry {
    /// Map from plugin ID to plugin entry
    plugins: HashMap<PluginId, PluginEntry>,
    /// Map from plugin ID to manifest
    manifests: HashMap<PluginId, PluginManifest>,
    /// Map from plugin ID to granted capabilities
    granted_caps: HashMap<PluginId, Vec<PluginCapability>>,
    /// Next plugin ID counter (for generated IDs)
    next_id: u64,
}

impl PluginRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        PluginRegistry {
            plugins: HashMap::new(),
            manifests: HashMap::new(),
            granted_caps: HashMap::new(),
            next_id: 1,
        }
    }

    /// Load an effect plugin from a directory
    pub fn load_effect_plugin(
        &mut self,
        manifest_path: impl AsRef<Path>,
        wasm_path: impl AsRef<Path>,
    ) -> PluginResult<PluginId> {
        let manifest = PluginManifest::from_json_file(&manifest_path)?;
        
        if manifest.plugin_type != PluginType::Effect {
            return Err(PluginError::manifest_validation(
                "Expected effect plugin type".to_string(),
            ));
        }

        let id = manifest.id.clone();
        
        // Check if already loaded
        if self.plugins.contains_key(&id) {
            return Err(PluginError::PluginAlreadyLoaded(id));
        }

        // Create the plugin
        let plugin = WasmEffectPlugin::new(manifest.clone(), EffectConfig::new(crate::effect::EffectType::Custom))?;
        plugin.load(&wasm_path)?;
        
        // Grant capabilities
        let mut granted = Vec::new();
        for cap in &manifest.capabilities {
            // In production, check if capability is allowed
            granted.push(*cap);
        }
        
        // Store the plugin
        self.manifests.insert(id.clone(), manifest);
        self.granted_caps.insert(id.clone(), granted);
        self.plugins.insert(id.clone(), PluginEntry::Effect(Arc::new(plugin)));
        
        Ok(id)
    }

    /// Load a tool plugin from a directory
    pub fn load_tool_plugin(
        &mut self,
        manifest_path: impl AsRef<Path>,
        wasm_path: impl AsRef<Path>,
    ) -> PluginResult<PluginId> {
        let manifest = PluginManifest::from_json_file(&manifest_path)?;
        
        if manifest.plugin_type != PluginType::Tool {
            return Err(PluginError::manifest_validation(
                "Expected tool plugin type".to_string(),
            ));
        }

        let id = manifest.id.clone();
        
        // Check if already loaded
        if self.plugins.contains_key(&id) {
            return Err(PluginError::PluginAlreadyLoaded(id));
        }

        // Create the plugin
        let plugin = WasmToolPlugin::new(manifest.clone(), ToolConfig::new(&manifest.name))?;
        plugin.load(&wasm_path)?;
        
        // Grant capabilities
        let mut granted = Vec::new();
        for cap in &manifest.capabilities {
            // In production, check if capability is allowed
            granted.push(*cap);
        }
        
        // Store the plugin
        self.manifests.insert(id.clone(), manifest);
        self.granted_caps.insert(id.clone(), granted);
        self.plugins.insert(id.clone(), PluginEntry::Tool(Arc::new(plugin)));
        
        Ok(id)
    }

    /// Register a built-in effect plugin
    pub fn register_builtin_effect(&mut self, effect: impl EffectPlugin + 'static) -> PluginId {
        let id = PluginId::new(format!("builtin.effect.{}", self.next_id));
        self.next_id += 1;
        
        let entry = PluginEntry::Effect(Arc::new(effect));
        self.plugins.insert(id.clone(), entry);
        
        id
    }

    /// Register a built-in tool plugin
    pub fn register_builtin_tool(&mut self, tool: impl ToolPlugin + 'static) -> PluginId {
        let id = PluginId::new(format!("builtin.tool.{}", self.next_id));
        self.next_id += 1;
        
        let entry = PluginEntry::Tool(Arc::new(tool));
        self.plugins.insert(id.clone(), entry);
        
        id
    }

    /// Unload a plugin by ID
    pub fn unload(&mut self, id: &PluginId) -> PluginResult<()> {
        self.plugins.remove(id);
        self.manifests.remove(id);
        self.granted_caps.remove(id);
        Ok(())
    }

    /// Get a plugin by ID (immutable)
    pub fn get(&self, id: &PluginId) -> Option<&PluginEntry> {
        self.plugins.get(id)
    }

    /// Get a plugin by ID (mutable) - for internal use only
    pub fn get_mut(&mut self, id: &PluginId) -> Option<&mut PluginEntry> {
        self.plugins.get_mut(id)
    }

    /// Get all loaded plugin IDs
    pub fn plugin_ids(&self) -> Vec<&PluginId> {
        self.plugins.keys().collect()
    }

    /// Get all loaded plugins
    pub fn plugins(&self) -> Vec<&PluginEntry> {
        self.plugins.values().collect()
    }

    /// Get plugins by type
    pub fn plugins_by_type(&self, plugin_type: PluginType) -> Vec<&PluginEntry> {
        self.plugins
            .values()
            .filter(|p| p.plugin_type() == plugin_type)
            .collect()
    }

    /// Get effect plugins
    pub fn effect_plugins(&self) -> Vec<&PluginEntry> {
        self.plugins_by_type(PluginType::Effect)
    }

    /// Get tool plugins
    pub fn tool_plugins(&self) -> Vec<&PluginEntry> {
        self.plugins_by_type(PluginType::Tool)
    }

    /// Get manifest for a plugin
    pub fn get_manifest(&self, id: &PluginId) -> Option<&PluginManifest> {
        self.manifests.get(id)
    }

    /// Get granted capabilities for a plugin
    pub fn get_granted_capabilities(&self, id: &PluginId) -> Option<&[PluginCapability]> {
        self.granted_caps.get(id).map(|v| v.as_slice())
    }

    /// Check if a plugin is loaded
    pub fn is_loaded(&self, id: &PluginId) -> bool {
        self.plugins.contains_key(id)
    }

    /// Grant a capability to a plugin
    pub fn grant_capability(
        &mut self,
        id: &PluginId,
        cap: PluginCapability,
    ) -> PluginResult<()> {
        if let Some(manifest) = self.manifests.get(id) {
            if !manifest.has_capability(cap) {
                return Err(PluginError::manifest_validation(format!(
                    "Cannot grant capability {:?} - not requested in manifest",
                    cap
                )));
            }
        }
        
        if let Some(caps) = self.granted_caps.get_mut(id) {
            if !caps.contains(&cap) {
                caps.push(cap);
            }
        } else {
            self.granted_caps.insert(id.clone(), vec![cap]);
        }
        
        Ok(())
    }

    /// Check if a plugin has a capability
    pub fn has_capability(&self, id: &PluginId, cap: PluginCapability) -> bool {
        self.get_granted_capabilities(id)
            .map(|caps| caps.contains(&cap))
            .unwrap_or(false)
    }

    /// Clear all plugins
    pub fn clear(&mut self) {
        self.plugins.clear();
        self.manifests.clear();
        self.granted_caps.clear();
    }

    /// Get the number of loaded plugins
    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    /// Check if there are no plugins loaded
    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe plugin registry
#[derive(Clone, Default)]
pub struct SafePluginRegistry {
    inner: Arc<RwLock<PluginRegistry>>,
}

impl SafePluginRegistry {
    /// Create a new thread-safe registry
    pub fn new() -> Self {
        SafePluginRegistry {
            inner: Arc::new(RwLock::new(PluginRegistry::new())),
        }
    }

    /// Load an effect plugin
    pub fn load_effect_plugin(
        &self,
        manifest_path: impl AsRef<Path>,
        wasm_path: impl AsRef<Path>,
    ) -> PluginResult<PluginId> {
        self.inner
            .write()
            .unwrap()
            .load_effect_plugin(manifest_path, wasm_path)
    }

    /// Load a tool plugin
    pub fn load_tool_plugin(
        &self,
        manifest_path: impl AsRef<Path>,
        wasm_path: impl AsRef<Path>,
    ) -> PluginResult<PluginId> {
        self.inner
            .write()
            .unwrap()
            .load_tool_plugin(manifest_path, wasm_path)
    }

    /// Register a built-in effect plugin
    pub fn register_builtin_effect(&self, effect: impl EffectPlugin + 'static) -> PluginId {
        self.inner
            .write()
            .unwrap()
            .register_builtin_effect(effect)
    }

    /// Register a built-in tool plugin
    pub fn register_builtin_tool(&self, tool: impl ToolPlugin + 'static) -> PluginId {
        self.inner
            .write()
            .unwrap()
            .register_builtin_tool(tool)
    }

    /// Unload a plugin
    pub fn unload(&self, id: &PluginId) -> PluginResult<()> {
        self.inner.write().unwrap().unload(id)
    }

    /// Get a plugin
    pub fn get(&self, id: &PluginId) -> Option<PluginEntry> {
        self.inner
            .read()
            .unwrap()
            .get(id)
            .cloned()
    }

    /// Get all plugin IDs
    pub fn plugin_ids(&self) -> Vec<PluginId> {
        self.inner
            .read()
            .unwrap()
            .plugin_ids()
            .into_iter()
            .cloned()
            .collect()
    }

    /// Get effect plugins
    pub fn effect_plugins(&self) -> Vec<PluginEntry> {
        self.inner
            .read()
            .unwrap()
            .effect_plugins()
            .into_iter()
            .cloned()
            .collect()
    }

    /// Get tool plugins
    pub fn tool_plugins(&self) -> Vec<PluginEntry> {
        self.inner
            .read()
            .unwrap()
            .tool_plugins()
            .into_iter()
            .cloned()
            .collect()
    }

    /// Check if a plugin is loaded
    pub fn is_loaded(&self, id: &PluginId) -> bool {
        self.inner.read().unwrap().is_loaded(id)
    }

    /// Get the number of plugins
    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.inner.read().unwrap().is_empty()
    }
}

impl SafePluginRegistry {
    /// Create a new registry with built-in plugins registered
    pub fn with_builtins() -> Self {
        let registry = SafePluginRegistry::new();
        
        // Register built-in effect plugins
        registry.register_builtin_effect(BrightnessContrastEffect::new(0.0, 1.0));
        
        // Register built-in tool plugins
        registry.register_builtin_tool(BrushTool::new(10.0, 1.0, [255, 0, 0, 255]));
        
        registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PluginType, PluginVersion};

    #[test]
    fn test_registry_creation() {
        let registry = PluginRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_register_builtin_effect() {
        let mut registry = PluginRegistry::new();
        let effect = BrightnessContrastEffect::new(0.0, 1.0);
        let id = registry.register_builtin_effect(effect);
        
        assert!(registry.is_loaded(&id));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_register_builtin_tool() {
        let mut registry = PluginRegistry::new();
        let tool = BrushTool::new(10.0, 1.0, [255, 0, 0, 255]);
        let id = registry.register_builtin_tool(tool);
        
        assert!(registry.is_loaded(&id));
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_get_plugin() {
        let mut registry = PluginRegistry::new();
        let effect = BrightnessContrastEffect::new(0.0, 1.0);
        let id = registry.register_builtin_effect(effect);
        
        let plugin = registry.get(&id).unwrap();
        assert_eq!(plugin.name(), "Brightness/Contrast");
        assert_eq!(plugin.plugin_type(), PluginType::Effect);
    }

    #[test]
    fn test_unload_plugin() {
        let mut registry = PluginRegistry::new();
        let effect = BrightnessContrastEffect::new(0.0, 1.0);
        let id = registry.register_builtin_effect(effect);
        
        assert!(registry.is_loaded(&id));
        
        registry.unload(&id).unwrap();
        assert!(!registry.is_loaded(&id));
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_safe_registry() {
        let registry = SafePluginRegistry::with_builtins();
        
        assert!(!registry.is_empty());
        assert!(registry.effect_plugins().len() >= 1);
        assert!(registry.tool_plugins().len() >= 1);
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plugin manifest schema and validation.
//!
//! A paint-type plugin consists of:
//! - A WASM module (.wasm file)
//! - A manifest file (plugin.toml or plugin.json)
//! - Optional signature file (.sig)
//!
//! The manifest declares:
//! - Plugin identity (id, version, name, description)
//! - Plugin type (Effect or Tool)
//! - Required capabilities
//! - Author and licensing information

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

use crate::error::{PluginError, PluginResult};

/// Unique plugin identifier (reverse domain notation, e.g., "com.example.myplugin")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PluginId(String);

impl PluginId {
    /// Create a new plugin ID
    pub fn new(id: impl Into<String>) -> Self {
        PluginId(id.into())
    }

    /// Get the plugin ID as a string slice
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PluginId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Plugin version following semver (major.minor.patch)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PluginVersion {
    /// Create a new version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        PluginVersion { major, minor, patch }
    }

    /// Parse a version string (e.g., "1.2.3")
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(PluginVersion {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }

    /// Minimum supported plugin API version
    pub const MIN_API_VERSION: u32 = 1;
    /// Current plugin API version
    pub const CURRENT_API_VERSION: u32 = 1;

    /// Check if this version is compatible with the current API
    pub fn is_compatible(&self) -> bool {
        self.major == Self::CURRENT_API_VERSION
    }
}

impl std::fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Plugin type - determines which API the plugin can use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Effect plugin - stateless transformations (filters, colour adjustments)
    Effect,
    /// Tool plugin - stateful tools (custom brushes, selection tools)
    Tool,
}

impl std::fmt::Display for PluginType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginType::Effect => write!(f, "effect"),
            PluginType::Tool => write!(f, "tool"),
        }
    }
}

/// Capability that a plugin may request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Read access to the canvas
    CanvasRead,
    /// Write access to the canvas
    CanvasWrite,
    /// Access to layer stack
    LayerAccess,
    /// Access to selection
    SelectionAccess,
    /// Access to file system (restricted paths)
    FileAccess,
    /// Network access (for fetching resources)
    NetworkAccess,
    /// User interface (modal dialogs, notifications)
    UserInterface,
    /// Persistent storage
    PersistentStorage,
}

/// Plugin manifest - the declarative metadata for a plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Plugin identifier (must be globally unique)
    pub id: PluginId,
    /// Plugin version
    pub version: PluginVersion,
    /// Human-readable name
    pub name: String,
    /// Short description
    pub description: String,
    /// Long description (markdown)
    #[serde(default)]
    pub long_description: Option<String>,
    /// Author name
    pub author: String,
    /// Author email
    #[serde(default)]
    pub author_email: Option<String>,
    /// Plugin type (Effect or Tool)
    pub plugin_type: PluginType,
    /// Minimum API version required
    #[serde(default = "default_api_version")]
    pub api_version: u32,
    /// List of capabilities this plugin requires
    #[serde(default)]
    pub capabilities: Vec<PluginCapability>,
    /// Main WASM module entry point
    pub wasm_entry: String,
    /// Icon URL or path
    #[serde(default)]
    pub icon: Option<String>,
    /// Plugin homepage
    #[serde(default)]
    pub homepage: Option<String>,
    /// License SPDX identifier
    pub license: String,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

impl PluginManifest {
    /// Load a manifest from a TOML file
    pub fn from_toml_file(path: impl AsRef<Path>) -> PluginResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| PluginError::io_error(path, e.to_string()))?;
        Self::from_toml(&content, path.parent().unwrap_or(path))
    }

    /// Load a manifest from a JSON file
    pub fn from_json_file(path: impl AsRef<Path>) -> PluginResult<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| PluginError::io_error(path, e.to_string()))?;
        serde_json::from_str(&content)
            .map_err(|e| PluginError::manifest_validation(e.to_string()))
    }

    /// Load a manifest from a TOML string
    pub fn from_toml(content: &str, base_path: &Path) -> PluginResult<Self> {
        // Simple TOML parsing - in production, use toml crate
        // For now, we'll use a basic approach
        let mut manifest = Self::default();
        
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }
            
            let key = parts[0].trim();
            let value = parts[1].trim().trim_matches('"');
            
            match key {
                "id" => manifest.id = PluginId::new(value.to_string()),
                "version" => {
                    if let Some(v) = PluginVersion::parse(value) {
                        manifest.version = v;
                    }
                }
                "name" => manifest.name = value.to_string(),
                "description" => manifest.description = value.to_string(),
                "author" => manifest.author = value.to_string(),
                "author_email" => manifest.author_email = Some(value.to_string()),
                "plugin_type" | "type" => {
                    manifest.plugin_type = match value.to_lowercase().as_str() {
                        "effect" => PluginType::Effect,
                        "tool" => PluginType::Tool,
                        _ => manifest.plugin_type,
                    };
                }
                "api_version" => {
                    if let Ok(v) = value.parse() {
                        manifest.api_version = v;
                    }
                }
                "wasm_entry" => manifest.wasm_entry = value.to_string(),
                "license" => manifest.license = value.to_string(),
                _ => {}
            }
        }
        
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest
    pub fn validate(&self) -> PluginResult<()> {
        if self.id.0.is_empty() {
            return Err(PluginError::manifest_validation("id is required"));
        }
        
        if !self.id.0.contains('.') {
            return Err(PluginError::manifest_validation(
                "id must be in reverse domain notation (e.g., com.example.plugin)",
            ));
        }
        
        if self.name.is_empty() {
            return Err(PluginError::manifest_validation("name is required"));
        }
        
        if self.description.is_empty() {
            return Err(PluginError::manifest_validation("description is required"));
        }
        
        if self.author.is_empty() {
            return Err(PluginError::manifest_validation("author is required"));
        }
        
        if self.wasm_entry.is_empty() {
            return Err(PluginError::manifest_validation("wasm_entry is required"));
        }
        
        if self.license.is_empty() {
            return Err(PluginError::manifest_validation("license is required"));
        }
        
        if self.api_version < PluginVersion::MIN_API_VERSION {
            return Err(PluginError::UnsupportedVersion(self.version.clone()));
        }
        
        if !self.version.is_compatible() {
            return Err(PluginError::UnsupportedVersion(self.version.clone()));
        }
        
        Ok(())
    }

    /// Get the set of required capabilities
    pub fn required_capabilities(&self) -> HashSet<PluginCapability> {
        self.capabilities.iter().cloned().collect()
    }

    /// Check if the plugin has a specific capability
    pub fn has_capability(&self, cap: PluginCapability) -> bool {
        self.capabilities.contains(&cap)
    }
}

impl Default for PluginManifest {
    fn default() -> Self {
        PluginManifest {
            id: PluginId::new("".to_string()),
            version: PluginVersion::new(1, 0, 0),
            name: "".to_string(),
            description: "".to_string(),
            long_description: None,
            author: "".to_string(),
            author_email: None,
            plugin_type: PluginType::Effect,
            api_version: PluginVersion::MIN_API_VERSION,
            capabilities: Vec::new(),
            wasm_entry: "_start".to_string(),
            icon: None,
            homepage: None,
            license: "AGPL-3.0-or-later".to_string(),
            tags: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = PluginVersion::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_version_compatibility() {
        let v = PluginVersion::parse("1.0.0").unwrap();
        assert!(v.is_compatible());
        
        let v = PluginVersion::parse("2.0.0").unwrap();
        assert!(!v.is_compatible());
    }

    #[test]
    fn test_manifest_validation() {
        let mut manifest = PluginManifest::default();
        manifest.id = PluginId::new("com.example.test".to_string());
        manifest.name = "Test Plugin".to_string();
        manifest.description = "A test plugin".to_string();
        manifest.author = "Test Author".to_string();
        manifest.wasm_entry = "_start".to_string();
        
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn test_manifest_validation_fails() {
        let manifest = PluginManifest::default();
        assert!(manifest.validate().is_err());
    }
}

fn default_api_version() -> u32 {
    PluginVersion::MIN_API_VERSION
}

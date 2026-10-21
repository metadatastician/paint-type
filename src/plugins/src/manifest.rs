// SPDX-License-Identifier: AGPL-3.0-or-later

//! Plugin manifest schema and validation.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path};

use crate::error::{PluginError, PluginResult};

/// Check that a relative plugin path contains no traversal or root components.
pub fn is_safe_relative_plugin_path(raw: &str) -> bool {
    if raw.is_empty() || raw.contains('\0') || raw.contains('\\') {
        return false;
    }
    let p = Path::new(raw);
    !p.is_absolute()
        && p.components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

/// Check that a plugin identifier contains only safe reverse-domain characters.
pub fn is_safe_plugin_id(raw: &str) -> bool {
    !raw.is_empty()
        && raw.contains('.')
        && !raw.starts_with('.')
        && !raw.ends_with('.')
        && !raw.contains("..")
        && raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_')
}

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
        PluginVersion {
            major,
            minor,
            patch,
        }
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
    #[serde(alias = "CanvasRead")]
    CanvasRead,
    /// Write access to the canvas
    #[serde(alias = "CanvasWrite")]
    CanvasWrite,
    /// Access to layer stack
    #[serde(alias = "LayerAccess")]
    LayerAccess,
    /// Access to selection
    #[serde(alias = "SelectionAccess")]
    SelectionAccess,
    /// Access to file system (restricted paths)
    #[serde(alias = "FileAccess")]
    FileAccess,
    /// Network access (for fetching resources)
    #[serde(alias = "NetworkAccess")]
    NetworkAccess,
    /// User interface (modal dialogs, notifications)
    #[serde(alias = "UserInterface")]
    UserInterface,
    /// Persistent storage
    #[serde(alias = "PersistentStorage")]
    PersistentStorage,
}

impl PluginCapability {
    /// Parse either snake_case (`canvas_read`) or PascalCase (`CanvasRead`).
    pub fn from_key(key: &str) -> Option<Self> {
        match key.trim() {
            "canvas_read" | "CanvasRead" => Some(Self::CanvasRead),
            "canvas_write" | "CanvasWrite" => Some(Self::CanvasWrite),
            "layer_access" | "LayerAccess" => Some(Self::LayerAccess),
            "selection_access" | "SelectionAccess" => Some(Self::SelectionAccess),
            "file_access" | "FileAccess" => Some(Self::FileAccess),
            "network_access" | "NetworkAccess" => Some(Self::NetworkAccess),
            "user_interface" | "UserInterface" => Some(Self::UserInterface),
            "persistent_storage" | "PersistentStorage" => Some(Self::PersistentStorage),
            _ => None,
        }
    }
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
    #[serde(alias = "type")]
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
        let manifest: Self = serde_json::from_str(&content)
            .map_err(|e| PluginError::manifest_validation(e.to_string()))?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Load a manifest from a TOML string
    pub fn from_toml(content: &str, _base_path: &Path) -> PluginResult<Self> {
        let mut manifest = Self::default();
        let mut in_capabilities_section = false;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                in_capabilities_section = line == "[capabilities]";
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim();
            let raw_val = parts[1].trim();
            let value = raw_val.trim_matches('"');

            if in_capabilities_section {
                if raw_val.eq_ignore_ascii_case("true") {
                    if let Some(cap) = PluginCapability::from_key(key) {
                        if !manifest.capabilities.contains(&cap) {
                            manifest.capabilities.push(cap);
                        }
                    }
                }
                continue;
            }

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
                "capabilities" => {
                    let inner = raw_val.trim_start_matches('[').trim_end_matches(']');
                    for item in inner.split(',') {
                        let cleaned = item.trim().trim_matches('"').trim_matches('\'');
                        if cleaned.is_empty() {
                            continue;
                        }
                        let cap = PluginCapability::from_key(cleaned).ok_or_else(|| {
                            PluginError::manifest_validation(format!(
                                "unknown capability '{cleaned}'"
                            ))
                        })?;
                        if !manifest.capabilities.contains(&cap) {
                            manifest.capabilities.push(cap);
                        }
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

    /// Validate the manifest (including path-traversal checks on `id` and `wasm_entry`).
    pub fn validate(&self) -> PluginResult<()> {
        if self.id.0.is_empty() {
            return Err(PluginError::manifest_validation("id is required"));
        }

        if !is_safe_plugin_id(&self.id.0) {
            return Err(PluginError::manifest_validation(
                "id must be safe reverse-domain notation without path traversal",
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

        if self.wasm_entry.is_empty() || !is_safe_relative_plugin_path(&self.wasm_entry) {
            return Err(PluginError::manifest_validation(
                "wasm_entry must be a non-empty relative path without traversal",
            ));
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
        self.capabilities.iter().copied().collect()
    }

    /// Check if the plugin has a specific capability
    pub fn has_capability(&self, cap: PluginCapability) -> bool {
        self.capabilities.contains(&cap)
    }
}

impl Default for PluginManifest {
    fn default() -> Self {
        PluginManifest {
            id: PluginId::new(String::new()),
            version: PluginVersion::new(1, 0, 0),
            name: String::new(),
            description: String::new(),
            long_description: None,
            author: String::new(),
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

fn default_api_version() -> u32 {
    PluginVersion::MIN_API_VERSION
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
    fn test_manifest_validation_and_traversal_rejection() {
        let mut manifest = PluginManifest {
            id: PluginId::new("com.example.test"),
            name: "Test Plugin".to_string(),
            description: "A test plugin".to_string(),
            author: "Test Author".to_string(),
            wasm_entry: "_start".to_string(),
            ..PluginManifest::default()
        };
        assert!(manifest.validate().is_ok());

        manifest.wasm_entry = "../escape.wasm".to_string();
        assert!(manifest.validate().is_err());

        manifest.wasm_entry = "_start".to_string();
        manifest.id = PluginId::new("com..traversal");
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn test_manifest_toml_parses_array_and_table_capabilities() {
        let toml_array = r#"
id = "com.example.filter"
version = "1.0.0"
name = "Filter"
description = "Test filter"
author = "Dev"
plugin_type = "effect"
wasm_entry = "plugin.wasm"
license = "AGPL-3.0-or-later"
capabilities = ["canvas_read", "CanvasWrite"]
"#;
        let parsed = PluginManifest::from_toml(toml_array, Path::new(".")).unwrap();
        assert!(parsed.has_capability(PluginCapability::CanvasRead));
        assert!(parsed.has_capability(PluginCapability::CanvasWrite));
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Plugin Manifest Types
//
// This module defines the plugin manifest types used by the provisioner.
// These mirror the types in paint-type-plugins crate for compatibility.

use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::path::Path;
use thiserror::Error;

/// Unique plugin identifier (reverse domain notation, e.g., "com.example.myplugin")
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct PluginId(String);

impl PluginId {
    pub fn new(id: impl Into<String>) -> Self {
        PluginId(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PluginId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for PluginId {
    fn from(s: String) -> Self {
        PluginId(s)
    }
}

impl From<&str> for PluginId {
    fn from(s: &str) -> Self {
        PluginId(s.to_string())
    }
}

/// Plugin version following semver (major.minor.patch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
pub struct PluginVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PluginVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        PluginVersion { major, minor, patch }
    }

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

    pub const MIN_API_VERSION: u32 = 1;
    pub const CURRENT_API_VERSION: u32 = 1;

    pub fn is_compatible(&self) -> bool {
        self.major == Self::CURRENT_API_VERSION
    }
}

/// Custom deserialization for PluginVersion to handle both string and struct formats
impl<'de> Deserialize<'de> for PluginVersion {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Try to deserialize as a struct first
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum VersionHelper {
            Struct { major: u32, minor: u32, patch: u32 },
            String(String),
        }
        
        let helper = VersionHelper::deserialize(deserializer)?;
        
        match helper {
            VersionHelper::Struct { major, minor, patch } => {
                Ok(PluginVersion { major, minor, patch })
            }
            VersionHelper::String(s) => {
                PluginVersion::parse(&s)
                    .ok_or_else(|| serde::de::Error::custom(format!("Invalid version string: {}", s)))
            }
        }
    }
}

impl fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Plugin type - determines which API the plugin can use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Effect plugin - stateless transformations (filters, colour adjustments)
    Effect,
    /// Tool plugin - stateful tools (custom brushes, selection tools)
    #[default]
    Tool,
}

impl fmt::Display for PluginType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginType::Effect => write!(f, "effect"),
            PluginType::Tool => write!(f, "tool"),
        }
    }
}

/// Capability that a plugin may request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// Read access to the canvas
    #[default]
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

impl fmt::Display for PluginCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginCapability::CanvasRead => write!(f, "CanvasRead"),
            PluginCapability::CanvasWrite => write!(f, "CanvasWrite"),
            PluginCapability::LayerAccess => write!(f, "LayerAccess"),
            PluginCapability::SelectionAccess => write!(f, "SelectionAccess"),
            PluginCapability::FileAccess => write!(f, "FileAccess"),
            PluginCapability::NetworkAccess => write!(f, "NetworkAccess"),
            PluginCapability::UserInterface => write!(f, "UserInterface"),
            PluginCapability::PersistentStorage => write!(f, "PersistentStorage"),
        }
    }
}

/// Plugin manifest - the declarative metadata for a plugin
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginManifest {
    /// Plugin identifier (must be globally unique)
    #[serde(default)]
    pub id: PluginId,
    /// Plugin version
    #[serde(default)]
    pub version: PluginVersion,
    /// Human-readable name
    #[serde(default)]
    pub name: String,
    /// Short description
    #[serde(default)]
    pub description: String,
    /// Long description (markdown)
    #[serde(default)]
    pub long_description: Option<String>,
    /// Author name
    #[serde(default)]
    pub author: String,
    /// Author email
    #[serde(default)]
    pub author_email: Option<String>,
    /// Plugin type (Effect or Tool)
    #[serde(default, rename = "type")]
    pub plugin_type: PluginType,
    /// Minimum API version required
    #[serde(default = "default_api_version")]
    pub api_version: u32,
    /// List of capabilities this plugin requires
    #[serde(default)]
    pub capabilities: Vec<PluginCapability>,
    /// Main WASM module entry point
    #[serde(default)]
    pub wasm_entry: String,
    /// Icon URL or path
    #[serde(default)]
    pub icon: Option<String>,
    /// Plugin homepage
    #[serde(default)]
    pub homepage: Option<String>,
    /// License SPDX identifier
    #[serde(default)]
    pub license: String,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_api_version() -> u32 {
    PluginVersion::MIN_API_VERSION
}

/// Error type for plugin operations
#[derive(Error, Debug)]
pub enum PluginError {
    #[error("IO error: {0}: {1}")]
    IoError(String, String),

    #[error("Manifest validation failed: {0}")]
    ManifestValidation(String),

    #[error("Unsupported plugin type: {0}")]
    UnsupportedType(String),

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(PluginVersion),

    #[error("Plugin already loaded: {0}")]
    PluginAlreadyLoaded(PluginId),

    #[error("Plugin not found: {0}")]
    PluginNotFound(PluginId),

    #[error("Capability error: {0}")]
    CapabilityError(String),
}

impl PluginManifest {
    /// Load a manifest from a TOML file
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| PluginError::IoError(path.display().to_string(), e.to_string()))?;
        Self::from_toml(&content, path.parent().unwrap_or(path))
    }

    /// Load a manifest from a JSON file
    pub fn from_json_file(path: impl AsRef<Path>) -> Result<Self, PluginError> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| PluginError::IoError(path.display().to_string(), e.to_string()))?;
        serde_json::from_str(&content)
            .map_err(|e| PluginError::ManifestValidation(e.to_string()))
    }

    /// Load a manifest from a TOML string
    pub fn from_toml(content: &str, _base_path: &Path) -> Result<Self, PluginError> {
        // Use toml crate for parsing
        let mut manifest: Self = toml::from_str(content)
            .map_err(|e| PluginError::ManifestValidation(e.to_string()))?;
        
        // Apply defaults for missing fields
        if manifest.id.0.is_empty() {
            manifest.id = PluginId::new("unknown".to_string());
        }
        if manifest.version.major == 0 && manifest.version.minor == 0 && manifest.version.patch == 0 {
            manifest.version = PluginVersion::new(1, 0, 0);
        }
        if manifest.wasm_entry.is_empty() {
            manifest.wasm_entry = "_start".to_string();
        }
        if manifest.license.is_empty() {
            manifest.license = "AGPL-3.0-or-later".to_string();
        }
        if manifest.api_version == 0 {
            manifest.api_version = PluginVersion::MIN_API_VERSION;
        }
        
        manifest.validate()?;
        Ok(manifest)
    }

    /// Validate the manifest
    pub fn validate(&self) -> Result<(), PluginError> {
        if self.id.0.is_empty() {
            return Err(PluginError::ManifestValidation(String::from("id is required")));
        }

        if !self.id.0.contains('.') {
            return Err(PluginError::ManifestValidation(
                String::from("id must be in reverse domain notation (e.g., com.example.plugin)"),
            ));
        }

        if self.name.is_empty() {
            return Err(PluginError::ManifestValidation(String::from("name is required")));
        }

        if self.description.is_empty() {
            return Err(PluginError::ManifestValidation(String::from("description is required")));
        }

        if self.author.is_empty() {
            return Err(PluginError::ManifestValidation(String::from("author is required")));
        }

        if self.wasm_entry.is_empty() {
            return Err(PluginError::ManifestValidation(String::from("wasm_entry is required")));
        }

        if self.license.is_empty() {
            return Err(PluginError::ManifestValidation(String::from("license is required")));
        }

        if self.api_version < PluginVersion::MIN_API_VERSION {
            return Err(PluginError::UnsupportedVersion(self.version));
        }

        if !self.version.is_compatible() {
            return Err(PluginError::UnsupportedVersion(self.version));
        }

        Ok(())
    }

    /// Get the set of required capabilities
    pub fn required_capabilities(&self) -> std::collections::HashSet<PluginCapability> {
        self.capabilities.iter().cloned().collect()
    }

    /// Check if the plugin has a specific capability
    pub fn has_capability(&self, cap: PluginCapability) -> bool {
        self.capabilities.contains(&cap)
    }
}

pub type PluginResult<T> = Result<T, PluginError>;

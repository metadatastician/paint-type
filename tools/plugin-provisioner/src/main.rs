// SPDX-License-Identifier: AGPL-3.0-or-later
//
// paint-type Plugin Provisioner
//
// A tool for deploying plugins and setting up their runtime environment.
// Inspired by:
// - boJ-server/tools/cartridge-provisioner/provisioner.js
// - panll/contracts/provisioner.toml
// - panll/src/core/provisioner_engine.affine
//
// This provisioner models plugin deployment as an affine state machine,
// ensuring each provisioning step is explicit and traceable (linear logic).

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, ValueEnum};
use hex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

mod manifest;
use manifest::{PluginCapability, PluginId, PluginManifest, PluginVersion};

/// Provisioning phase - models the affine state transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ProvisioningPhase {
    /// Waiting to start
    Idle,
    /// Resolving plugin dependencies
    ResolvingDependencies,
    /// Downloading plugin artifacts
    Downloading,
    /// Validating integrity
    Validating,
    /// Installing to target directory
    Installing,
    /// Configuring plugin
    Configuring,
    /// Provisioning complete
    Complete,
    /// Provisioning failed
    Failed,
}

/// Provisioning state - affine state machine
#[derive(Debug, Clone)]
pub struct ProvisioningState {
    pub phase: ProvisioningPhase,
    pub plugin_id: Option<PluginId>,
    pub resolved_dependencies: HashMap<String, PluginManifest>,
    pub downloaded_plugins: HashSet<String>,
    pub validated_plugins: HashSet<String>,
    pub installed_plugins: HashSet<String>,
    pub configured_plugins: HashSet<String>,
    pub provisioning_log: Vec<String>,
    pub errors: Vec<String>,
}

impl ProvisioningState {
    pub fn new() -> Self {
        Self {
            phase: ProvisioningPhase::Idle,
            plugin_id: None,
            resolved_dependencies: HashMap::new(),
            downloaded_plugins: HashSet::new(),
            validated_plugins: HashSet::new(),
            installed_plugins: HashSet::new(),
            configured_plugins: HashSet::new(),
            provisioning_log: Vec::new(),
            errors: Vec::new(),
        }
    }

    pub fn log(&mut self, message: &str) {
        self.provisioning_log.push(message.to_string());
        info!("{}", message);
    }

    pub fn error(&mut self, message: &str) {
        self.errors.push(message.to_string());
        error!("{}", message);
    }

    pub fn is_complete(&self) -> bool {
        self.phase == ProvisioningPhase::Complete
    }

    pub fn is_failed(&self) -> bool {
        self.phase == ProvisioningPhase::Failed
    }
}

/// Plugin configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginConfig {
    pub id: PluginId,
    pub enabled: bool,
    pub capabilities: Vec<PluginCapability>,
    pub granted_capabilities: Vec<PluginCapability>,
    pub priority: u32,
}

/// Deployment result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentResult {
    pub plugin_id: PluginId,
    pub plugin_name: String,
    pub version: PluginVersion,
    pub target_directory: PathBuf,
    pub capabilities_granted: Vec<PluginCapability>,
    pub dependencies_resolved: Vec<String>,
    pub checksum: String,
    pub timestamp: String,
}

impl DeploymentResult {
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize deployment result")
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let json = self.to_json()?;
        fs::write(path, json).context("Failed to write deployment result")
    }
}

/// Compute SHA-256 checksum of a file
fn compute_checksum(file_path: &Path) -> Result<String> {
    let mut file = File::open(file_path).context("Failed to open file for checksum")?;
    let mut hasher = Sha256::new();
    let _ = std::io::copy(&mut file, &mut hasher).context("Failed to read file for checksum")?;
    Ok(hex::encode(hasher.finalize()))
}

/// Validate plugin manifest
fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
    manifest.validate().map_err(|e| anyhow::anyhow!("Manifest validation failed: {}", e))?;
    Ok(())
}

/// Load manifest from a directory
fn load_manifest_from_dir(dir: &Path) -> Result<PluginManifest> {
    let toml_path = dir.join("plugin.toml");
    let json_path = dir.join("plugin.json");

    if toml_path.exists() {
        Ok(PluginManifest::from_toml_file(toml_path)?)
    } else if json_path.exists() {
        Ok(PluginManifest::from_json_file(json_path)?)
    } else {
        anyhow::bail!("No plugin manifest found in directory: {}", dir.display());
    }
}

/// Get standard plugin directory
fn get_plugin_dir() -> PathBuf {
    PathBuf::from("src/plugins/available")
}

/// Ensure directory exists
fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).context("Failed to create directory")?;
    }
    Ok(())
}

/// Copy file with checksum verification
fn copy_file_with_checksum(src: &Path, dst: &Path) -> Result<String> {
    let checksum = compute_checksum(src)?;
    fs::copy(src, dst).context("Failed to copy file")?;
    let dst_checksum = compute_checksum(dst)?;
    if checksum != dst_checksum {
        anyhow::bail!(
            "Checksum mismatch after copy: src={}, dst={}",
            checksum, dst_checksum
        );
    }
    Ok(checksum)
}

/// Get WASM entry point file
fn get_wasm_file(dir: &Path, manifest: &PluginManifest) -> PathBuf {
    dir.join(&manifest.wasm_entry)
}

/// Provision a single plugin
fn provision_plugin(
    source_dir: &Path,
    target_dir: &Path,
    state: &mut ProvisioningState,
) -> Result<DeploymentResult> {
    state.log(&format!("Starting provisioning from: {}", source_dir.display()));
    state.phase = ProvisioningPhase::ResolvingDependencies;

    // Load manifest
    let manifest = load_manifest_from_dir(source_dir)
        .context("Failed to load plugin manifest")?;
    state.log(&format!("Loaded manifest for plugin: {}", manifest.id));

    // Validate manifest
    state.phase = ProvisioningPhase::Validating;
    validate_manifest(&manifest).context("Manifest validation failed")?;
    state.log("Manifest validated successfully");

    // Create target directory
    let plugin_target = target_dir.join(manifest.id.as_str());
    ensure_dir_exists(&plugin_target).context("Failed to create plugin target directory")?;

    // Copy plugin files
    state.phase = ProvisioningPhase::Installing;
    state.log("Copying plugin files...");

    // Copy manifest
    let manifest_src = source_dir.join("plugin.toml");
    let manifest_dst = plugin_target.join("plugin.toml");
    if manifest_src.exists() {
        copy_file_with_checksum(&manifest_src, &manifest_dst)?;
    }

    let manifest_json_src = source_dir.join("plugin.json");
    let manifest_json_dst = plugin_target.join("plugin.json");
    if manifest_json_src.exists() {
        copy_file_with_checksum(&manifest_json_src, &manifest_json_dst)?;
    }

    // Copy WASM module
    let wasm_src = get_wasm_file(source_dir, &manifest);
    if !wasm_src.exists() {
        anyhow::bail!(
            "WASM entry point not found: {}",
            wasm_src.display()
        );
    }
    let wasm_dst = plugin_target.join(wasm_src.file_name().unwrap_or_default());
    let checksum = copy_file_with_checksum(&wasm_src, &wasm_dst)
        .context("Failed to copy WASM module")?;
    state.log(&format!("WASM module copied: {}", wasm_dst.display()));

    // Copy other files (README, LICENSE, etc.)
    for entry in WalkDir::new(source_dir) {
        let entry = entry.context("Failed to walk source directory")?;
        let rel_path = entry.path().strip_prefix(source_dir).unwrap_or(entry.path());
        let dst_path = plugin_target.join(rel_path);

        // Skip manifest files (already copied)
        if rel_path == Path::new("plugin.toml") || rel_path == Path::new("plugin.json") {
            continue;
        }

        // Skip WASM file (already copied)
        if rel_path == Path::new(&manifest.wasm_entry) {
            continue;
        }

        if entry.file_type().is_file() {
            if let Some(parent) = dst_path.parent() {
                ensure_dir_exists(parent)?;
            }
            fs::copy(entry.path(), &dst_path).context("Failed to copy plugin file")?;
            debug!("Copied: {}", rel_path.display());
        }
    }

    // Create plugin config
    state.phase = ProvisioningPhase::Configuring;
    state.log("Creating plugin configuration...");

    let plugin_config = PluginConfig {
        id: manifest.id.clone(),
        enabled: true,
        capabilities: manifest.capabilities.clone(),
        granted_capabilities: manifest.capabilities.clone(), // Grant all requested by default
        priority: 100,
    };

    let config_path = plugin_target.join("plugin.config.toml");
    let config_toml = toml::to_string(&plugin_config)
        .context("Failed to serialize plugin config")?;
    fs::write(config_path, config_toml).context("Failed to write plugin config")?;

    // Generate deployment result
    state.phase = ProvisioningPhase::Complete;
    state.log("Provisioning complete");

    let result = DeploymentResult {
        plugin_id: manifest.id.clone(),
        plugin_name: manifest.name.clone(),
        version: manifest.version.clone(),
        target_directory: plugin_target.clone(),
        capabilities_granted: manifest.capabilities.clone(),
        dependencies_resolved: Vec::new(), // TODO: Implement dependency resolution
        checksum,
        timestamp: Utc::now().to_rfc3339(),
    };

    Ok(result)
}

/// List deployed plugins
fn list_plugins(target_dir: &Path, detailed: bool) -> Result<()> {
    if !target_dir.exists() {
        warn!("Target directory does not exist: {}", target_dir.display());
        return Ok(());
    }

    info!("Deployed plugins in: {}", target_dir.display());
    println!("\nDeployed Plugins:");
    println!("{}", "=".repeat(60));

    let mut plugins: Vec<(PathBuf, PluginManifest)> = Vec::new();

    for entry in WalkDir::new(target_dir) {
        let entry = entry.context("Failed to walk target directory")?;
        if !entry.file_type().is_dir() {
            continue;
        }

        let manifest_path = entry.path().join("plugin.toml");
        if manifest_path.exists() {
            if let Ok(manifest) = PluginManifest::from_toml_file(&manifest_path) {
                plugins.push((entry.path().to_path_buf(), manifest));
            }
        }

        let manifest_path = entry.path().join("plugin.json");
        if manifest_path.exists() {
            if let Ok(manifest) = PluginManifest::from_json_file(&manifest_path) {
                plugins.push((entry.path().to_path_buf(), manifest));
            }
        }
    }

    if plugins.is_empty() {
        println!("No plugins found");
        return Ok(());
    }

    for (dir, manifest) in &plugins {
        println!("\nID: {}", manifest.id);
        println!("  Name: {}", manifest.name);
        println!("  Version: {}", manifest.version);
        println!("  Type: {}", manifest.plugin_type);
        println!("  Author: {}", manifest.author);
        println!("  Directory: {}", dir.display());

        if detailed {
            println!("  Description: {}", manifest.description);
            println!("  WASM Entry: {}", manifest.wasm_entry);
            println!("  Capabilities: {:?}", manifest.capabilities);
            println!("  License: {}", manifest.license);
            if let Some(homepage) = &manifest.homepage {
                println!("  Homepage: {}", homepage);
            }
        }
    }

    println!("\n{}", "=".repeat(60));
    println!("Total: {} plugin(s)", plugins.len());

    Ok(())
}

/// Validate a plugin
fn validate_plugin(path: &Path) -> Result<()> {
    let manifest = if path.is_file() {
        if path.extension().map_or(false, |ext| ext == "toml") {
            PluginManifest::from_toml_file(path)?
        } else if path.extension().map_or(false, |ext| ext == "json") {
            PluginManifest::from_json_file(path)?
        } else {
            anyhow::bail!("Unsupported manifest file: {}", path.display());
        }
    } else {
        load_manifest_from_dir(path)?
    };

    validate_manifest(&manifest)?;
    info!("Plugin manifest is valid");
    println!("✓ Plugin validation passed");
    println!("  ID: {}", manifest.id);
    println!("  Name: {}", manifest.name);
    println!("  Version: {}", manifest.version);
    println!("  Type: {}", manifest.plugin_type);

    Ok(())
}

/// Remove a plugin
fn remove_plugin(plugin_id_or_path: &str, target_dir: &Path) -> Result<()> {
    let plugin_path = if Path::new(plugin_id_or_path).is_absolute() {
        PathBuf::from(plugin_id_or_path)
    } else {
        target_dir.join(plugin_id_or_path)
    };

    if !plugin_path.exists() {
        anyhow::bail!("Plugin not found: {}", plugin_path.display());
    }

    // Check if it's a directory with a plugin manifest
    let manifest_toml = plugin_path.join("plugin.toml");
    let manifest_json = plugin_path.join("plugin.json");

    if !manifest_toml.exists() && !manifest_json.exists() {
        anyhow::bail!(
            "Not a valid plugin directory (no manifest found): {}",
            plugin_path.display()
        );
    }

    // Load manifest to get plugin ID
    let manifest = if manifest_toml.exists() {
        PluginManifest::from_toml_file(&manifest_toml)?
    } else {
        PluginManifest::from_json_file(&manifest_json)?
    };

    info!("Removing plugin: {}", manifest.id);
    fs::remove_dir_all(&plugin_path).context("Failed to remove plugin directory")?;
    println!("✓ Plugin removed: {}", manifest.id);

    Ok(())
}

/// paint-type Plugin Provisioner CLI
#[derive(Parser, Debug)]
#[command(name = "plugin-provisioner")]
#[command(author = "paint.type team")]
#[command(version = "0.1.0")]
#[command(about = "Deploy and manage paint.type plugins", long_about = None)]
struct Args {
    /// Command to execute
    #[command(subcommand)]
    command: Command,
}

#[derive(Parser, Debug)]
enum Command {
    /// Deploy a plugin from a source directory
    Deploy {
        /// Path to plugin source directory
        #[arg(short, long)]
        source: PathBuf,

        /// Target directory to deploy to (default: standard plugin directory)
        #[arg(short, long)]
        target: Option<PathBuf>,

        /// Skip dependency resolution
        #[arg(long)]
        skip_deps: bool,

        /// Skip integrity validation
        #[arg(long)]
        skip_validation: bool,

        /// Force overwrite existing installation
        #[arg(long)]
        force: bool,
    },

    /// Install a plugin from the registry (not yet implemented - requires network feature)
    Install {
        /// Plugin ID to install (e.g., com.example.myplugin)
        #[arg(short, long)]
        plugin_id: String,

        /// Version to install (default: latest)
        #[arg(short, long)]
        version: Option<String>,

        /// Target directory
        #[arg(short, long)]
        target: Option<PathBuf>,

        /// Registry URL (default: official paint.type registry)
        #[arg(long)]
        registry: Option<String>,
    },

    /// Remove a deployed plugin
    Remove {
        /// Plugin ID or path to remove
        #[arg(short, long)]
        plugin_id: String,

        /// Target directory (where plugin is installed)
        #[arg(short, long)]
        target: Option<PathBuf>,
    },

    /// List deployed plugins
    List {
        /// Target directory to list (default: standard plugin directory)
        #[arg(short, long)]
        target: Option<PathBuf>,

        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Validate a plugin manifest
    Validate {
        /// Path to plugin directory or manifest file
        #[arg(short, long)]
        path: PathBuf,
    },

    /// Resolve dependencies for a plugin
    Resolve {
        /// Path to plugin manifest
        #[arg(short, long)]
        manifest: PathBuf,

        /// Registry URL
        #[arg(long)]
        registry: Option<String>,
    },
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("plugin_provisioner=info".parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    match args.command {
        Command::Deploy {
            source,
            target,
            skip_deps: _,
            skip_validation,
            force: _,
        } => {
            let target_dir = target.unwrap_or_else(get_plugin_dir);
            ensure_dir_exists(&target_dir)?;

            let mut state = ProvisioningState::new();

            if !skip_validation {
                // Validate source
                validate_plugin(&source)?;
            }

            let result = provision_plugin(&source, &target_dir, &mut state)?;

            // Save deployment result
            let result_path = target_dir.join("_deployment_result.json");
            result.save(&result_path)?;
            info!("Deployment result saved to: {}", result_path.display());

            println!("\n✓ Plugin deployed successfully!");
            println!("  Plugin: {}", result.plugin_id);
            println!("  Version: {}", result.version);
            println!("  Target: {}", result.target_directory.display());
            println!("  Checksum: {}", result.checksum);
        }

        Command::Install {
            plugin_id: _,
            version: _,
            target: _,
            registry: _,
        } => {
            warn!("Network installation not yet implemented");
            warn!("Use 'deploy' for local plugins");
            warn!("To enable network installation, build with --features network");
        }

        Command::Remove {
            plugin_id,
            target,
        } => {
            let target_dir = target.unwrap_or_else(get_plugin_dir);
            remove_plugin(&plugin_id, &target_dir)?;
        }

        Command::List {
            target,
            detailed,
        } => {
            let target_dir = target.unwrap_or_else(get_plugin_dir);
            list_plugins(&target_dir, detailed)?;
        }

        Command::Validate { path } => {
            validate_plugin(&path)?;
        }

        Command::Resolve {
            manifest,
            registry: _,
        } => {
            let manifest = load_manifest_from_dir(manifest.parent().unwrap_or(&manifest))?;
            info!("Manifest loaded: {}", manifest.id);

            // Display the capabilities (dependencies)
            println!("Plugin: {}", manifest.id);
            println!("Type: {}", manifest.plugin_type);
            
            if !manifest.capabilities.is_empty() {
                println!("\nRequired capabilities:");
                for cap in &manifest.capabilities {
                    println!("  - {}", cap);
                }
            } else {
                println!("No capabilities required");
            }
        }
    }

    Ok(())
}

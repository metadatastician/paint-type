// SPDX-License-Identifier: AGPL-3.0-or-later
//
// plugin-harness -- Tests and integrates paint.type plugins.
//
// Usage:
//   cargo run --manifest-path tools/plugin-harness/Cargo.toml -- <plugin-dir> [OPTIONS]
//
// Inspired by:
//   - boJ-server/tools/panel-harness/harness.js
//   - panll/schema/panll-harness-v2.schema.json
//   - panll/generated/k9iser/panel-harness.k9

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Harness configuration from harness.toml or command-line
#[derive(Debug, Deserialize, Clone)]
struct HarnessConfig {
    // Plugin to test
    #[serde(default)]
    plugin_path: Option<String>,
    
    // Test configuration
    #[serde(default)]
    test_timeout_secs: u64,
    #[serde(default)]
    test_memory_limit_mb: usize,
    
    // Runtime target
    #[serde(default = "default_target")]
    target: String,
    
    // Registration options
    #[serde(default)]
    register: bool,
    #[serde(default)]
    registry_path: Option<String>,
    
    // Health check configuration
    #[serde(default)]
    health_check: Option<HealthCheckConfig>,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        HarnessConfig {
            plugin_path: None,
            test_timeout_secs: 30,
            test_memory_limit_mb: 256,
            target: default_target(),
            register: false,
            registry_path: None,
            health_check: None,
        }
    }
}

fn default_target() -> String {
    "paint-type".to_string()
}

/// Health check configuration
#[derive(Debug, Deserialize, Serialize, Clone)]
struct HealthCheckConfig {
    #[serde(default = "default_health_path")]
    path: String,
    #[serde(default = "default_health_interval")]
    interval_ms: u64,
    #[serde(default = "default_health_timeout")]
    timeout_ms: u64,
    #[serde(default = "default_health_threshold")]
    unhealthy_threshold: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        HealthCheckConfig {
            path: default_health_path(),
            interval_ms: default_health_interval(),
            timeout_ms: default_health_timeout(),
            unhealthy_threshold: default_health_threshold(),
        }
    }
}

fn default_health_path() -> String {
    "/health".to_string()
}

fn default_health_interval() -> u64 {
    5000
}

fn default_health_timeout() -> u64 {
    1000
}

fn default_health_threshold() -> u32 {
    3
}

/// Plugin manifest (matches the manifest in src/plugins/src/manifest.rs)
#[derive(Debug, Deserialize, Serialize, Clone)]
struct PluginManifest {
    pub id: String,
    pub version: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub long_description: Option<String>,
    pub author: String,
    #[serde(default)]
    pub author_email: Option<String>,
    pub plugin_type: String,
    #[serde(default = "default_api_version")]
    pub api_version: u32,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub wasm_entry: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_api_version() -> u32 {
    1
}

/// Runtime-specific endpoint configuration
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct RuntimeEndpoints {
    #[serde(default)]
    pub paint_type: Option<String>,
    #[serde(default)]
    pub wasm: Option<String>,
    #[serde(default)]
    pub http: Option<String>,
}

/// Harness manifest for a plugin
#[derive(Debug, Deserialize, Serialize, Clone)]
struct HarnessManifest {
    #[serde(default = "default_schema")]
    pub schema: String,
    pub service_id: String,
    pub plugin_id: String,
    pub default_endpoint: String,
    #[serde(default)]
    pub runtime_endpoints: RuntimeEndpoints,
    #[serde(default)]
    pub health_check: Option<HealthCheckConfig>,
    #[serde(default)]
    pub data_sources: Vec<DataSource>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub metadata: HarnessMetadata,
}

impl Default for HarnessManifest {
    fn default() -> Self {
        HarnessManifest {
            schema: default_schema(),
            service_id: String::new(),
            plugin_id: String::new(),
            default_endpoint: String::new(),
            runtime_endpoints: RuntimeEndpoints::default(),
            health_check: None,
            data_sources: Vec::new(),
            capabilities: Vec::new(),
            metadata: HarnessMetadata::default(),
        }
    }
}

fn default_schema() -> String {
    "paint-type-harness/v1".to_string()
}

/// Data source definition
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct DataSource {
    pub name: String,
    pub path: String,
    pub method: String,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
    pub returns: String,
    #[serde(default)]
    pub description: Option<String>,
}

/// Harness metadata
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
struct HarnessMetadata {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
}

/// Test result for a plugin
#[derive(Debug, Deserialize, Serialize, Clone)]
struct TestResult {
    pub plugin_id: String,
    pub passed: bool,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub duration_ms: u64,
}

/// Validate a plugin manifest
fn validate_manifest(manifest: &PluginManifest) -> Result<()> {
    let mut errors = Vec::new();
    
    if manifest.id.is_empty() {
        errors.push("missing 'id' field");
    } else if !manifest.id.contains('.') {
        errors.push("'id' must be in reverse domain notation (e.g., com.example.plugin)");
    }
    
    if manifest.name.is_empty() {
        errors.push("missing 'name' field");
    }
    
    if manifest.description.is_empty() {
        errors.push("missing 'description' field");
    }
    
    if manifest.author.is_empty() {
        errors.push("missing 'author' field");
    }
    
    if manifest.plugin_type.is_empty() {
        errors.push("missing 'plugin_type' field");
    } else if manifest.plugin_type != "effect" && manifest.plugin_type != "tool" {
        errors.push("'plugin_type' must be 'effect' or 'tool'");
    }
    
    if manifest.wasm_entry.is_empty() {
        errors.push("missing 'wasm_entry' field");
    }
    
    if manifest.license.is_empty() {
        errors.push("missing 'license' field");
    }
    
    if !errors.is_empty() {
        anyhow::bail!("manifest validation failed:\n  - {}", errors.join("\n  - "));
    }
    
    Ok(())
}

/// Load a plugin manifest from a TOML file
fn load_plugin_manifest(path: &Path) -> Result<PluginManifest> {
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read manifest: {}", path.display()))?;
    
    let manifest: PluginManifest = toml::from_str(&content)
        .with_context(|| format!("failed to parse manifest: {}", path.display()))?;
    
    validate_manifest(&manifest)?;
    
    Ok(manifest)
}

/// Generate a harness manifest from a plugin manifest
fn generate_harness_manifest(plugin_manifest: &PluginManifest, target: &str) -> HarnessManifest {
    let plugin_id_normalized = plugin_manifest.id.replace('.', "-");
    let mut service_id = String::with_capacity(20 + plugin_id_normalized.len());
    service_id.push_str("paint-type-plugin-");
    service_id.push_str(&plugin_id_normalized);
    
    let mut default_endpoint = String::with_capacity(15 + target.len() + plugin_manifest.id.len());
    default_endpoint.push_str("paint-type://");
    default_endpoint.push_str(target);
    default_endpoint.push('/');
    default_endpoint.push_str(&plugin_manifest.id);
    
    let mut paint_type_endpoint = String::with_capacity(15 + target.len() + plugin_manifest.id.len());
    paint_type_endpoint.push_str("paint-type://");
    paint_type_endpoint.push_str(target);
    paint_type_endpoint.push('/');
    paint_type_endpoint.push_str(&plugin_manifest.id);
    
    let mut wasm_endpoint = String::with_capacity(7 + plugin_manifest.wasm_entry.len());
    wasm_endpoint.push_str("wasm://");
    wasm_endpoint.push_str(&plugin_manifest.wasm_entry);
    
    HarnessManifest {
        schema: "paint-type-harness/v1".to_string(),
        service_id,
        plugin_id: plugin_manifest.id.clone(),
        default_endpoint,
        runtime_endpoints: RuntimeEndpoints {
            paint_type: Some(paint_type_endpoint),
            wasm: Some(wasm_endpoint),
            http: None,
        },
        health_check: Some(HealthCheckConfig {
            path: "/health".to_string(),
            interval_ms: 5000,
            timeout_ms: 1000,
            unhealthy_threshold: 3,
        }),
        data_sources: Vec::new(),
        capabilities: plugin_manifest.capabilities.clone(),
        metadata: HarnessMetadata {
            name: Some(plugin_manifest.name.clone()),
            description: Some(plugin_manifest.description.clone()),
            version: Some(plugin_manifest.version.clone()),
            author: Some(plugin_manifest.author.clone()),
            license: Some(plugin_manifest.license.clone()),
        },
    }
}

/// Test a plugin by loading its manifest and validating it
fn test_plugin(plugin_dir: &Path, config: &HarnessConfig) -> Result<TestResult> {
    use std::time::Instant;
    
    let start = Instant::now();
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    
    // Find plugin.toml
    let manifest_path = plugin_dir.join("plugin.toml");
    
    if !manifest_path.exists() {
        errors.push(format!("plugin.toml not found in {}", plugin_dir.display()));
        return Ok(TestResult {
            plugin_id: String::new(),
            passed: false,
            errors,
            warnings,
            duration_ms: start.elapsed().as_millis() as u64,
        });
    }
    
    // Load the manifest
    let manifest = match load_plugin_manifest(&manifest_path) {
        Ok(m) => m,
        Err(e) => {
            errors.push(format!("failed to load manifest: {}", e));
            return Ok(TestResult {
                plugin_id: String::new(),
                passed: false,
                errors,
                warnings,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
    };
    
    // Check for WASM file
    let mut wasm_filename = manifest.id.replace('.', "-");
    wasm_filename.push_str(".wasm");
    let wasm_path = plugin_dir.join("target").join("wasm32-unknown-unknown").join("release").join(wasm_filename);
    if !wasm_path.exists() {
        warnings.push(format!("WASM file not found at {}", wasm_path.display()));
        warnings.push("Run 'cargo build --release' to build the plugin".to_string());
    }
    
    // Generate harness manifest
    let harness_manifest = generate_harness_manifest(&manifest, &config.target);
    
    // If registration is enabled, save the harness manifest
    if config.register {
        let registry_dir = config.registry_path.as_deref()
            .map(Path::new)
            .unwrap_or_else(|| plugin_dir.parent().unwrap_or(plugin_dir));
        
        let harness_path = registry_dir.join(format!("{}-harness.toml", manifest.id));
        let harness_toml = toml::to_string(&harness_manifest)
            .map_err(|e| anyhow::anyhow!("failed to serialize harness manifest: {}", e))?;
        
        fs::write(&harness_path, harness_toml)
            .with_context(|| format!("failed to write harness manifest: {}", harness_path.display()))?;
        
        println!("✓ Harness manifest written: {}", harness_path.display());
    }
    
    let passed = errors.is_empty();
    
    Ok(TestResult {
        plugin_id: manifest.id.clone(),
        passed,
        errors,
        warnings,
        duration_ms: start.elapsed().as_millis() as u64,
    })
}

/// Register a plugin with the paint-type system
fn register_plugin(plugin_dir: &Path, harness_config: &HarnessConfig) -> Result<()> {
    let manifest = load_plugin_manifest(&plugin_dir.join("plugin.toml"))?;
    
    println!("Registering plugin: {}", manifest.name);
    println!("  ID: {}", manifest.id);
    println!("  Version: {}", manifest.version);
    println!("  Type: {}", manifest.plugin_type);
    println!("  Author: {}", manifest.author);
    println!("  Capabilities: {:?}", manifest.capabilities);
    
    // Generate and save harness manifest
    let harness_manifest = generate_harness_manifest(&manifest, &harness_config.target);
    
    let registry_dir = harness_config.registry_path.as_deref()
        .map(Path::new)
        .unwrap_or_else(|| plugin_dir.parent().unwrap_or(plugin_dir));
    
    fs::create_dir_all(registry_dir)?;
    
    let harness_path = registry_dir.join(format!("{}-harness.toml", manifest.id));
    let harness_toml = toml::to_string(&harness_manifest)?;
    
    fs::write(&harness_path, harness_toml)?;
    
    println!("✓ Registered plugin: {}", manifest.id);
    println!("  Harness manifest: {}", harness_path.display());
    println!("  Endpoint: {}", harness_manifest.default_endpoint);
    
    Ok(())
}

/// Print harness manifest as JSON (for integration with other tools)
fn print_harness_json(plugin_dir: &Path, target: &str) -> Result<()> {
    let manifest = load_plugin_manifest(&plugin_dir.join("plugin.toml"))?;
    let harness_manifest = generate_harness_manifest(&manifest, target);
    
    let json = serde_json::to_string_pretty(&harness_manifest)?;
    println!("{}", json);
    
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        println!("Usage: plugin-harness <plugin-dir> [OPTIONS]");
        println!();
        println!("Tests and integrates paint.type plugins.");
        println!();
        println!("Options:");
        println!("  --test              Run plugin tests");
        println!("  --register          Register plugin with the system");
        println!("  --target <name>     Target runtime (default: paint-type)");
        println!("  --registry <path>   Path to plugin registry");
        println!("  --harness-json      Output harness manifest as JSON");
        println!("  --timeout <secs>    Test timeout in seconds (default: 30)");
        println!("  --help, -h          Show this help");
        println!();
        println!("Examples:");
        println!("  # Test a plugin");
        println!("  cargo run --manifest-path tools/plugin-harness/Cargo.toml -- src/plugins/available/my-plugin --test");
        println!();
        println!("  # Register a plugin");
        println!("  cargo run --manifest-path tools/plugin-harness/Cargo.toml -- src/plugins/available/my-plugin --register");
        println!();
        println!("  # Get harness manifest as JSON");
        println!("  cargo run --manifest-path tools/plugin-harness/Cargo.toml -- src/plugins/available/my-plugin --harness-json");
        std::process::exit(if args.len() < 2 { 1 } else { 0 });
    }
    
    let plugin_dir = Path::new(&args[1]);
    
    if !plugin_dir.exists() {
        anyhow::bail!("plugin directory not found: {}", plugin_dir.display());
    }
    
    // Parse command-line options
    let mut config = HarnessConfig::default();
    let mut test_mode = false;
    let mut register_mode = false;
    let mut harness_json_mode = false;
    
    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--test" => test_mode = true,
            "--register" => register_mode = true,
            "--harness-json" => harness_json_mode = true,
            "--target" => {
                if i + 1 < args.len() {
                    config.target = args[i + 1].clone();
                    i += 1;
                }
            }
            "--registry" => {
                if i + 1 < args.len() {
                    config.registry_path = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--timeout" => {
                if i + 1 < args.len() {
                    config.test_timeout_secs = args[i + 1].parse()?;
                    i += 1;
                }
            }
            arg if arg.starts_with("--") => {
                eprintln!("Unknown option: {}", arg);
                std::process::exit(1);
            }
            _ => {
                eprintln!("Unexpected argument: {}", args[i]);
                std::process::exit(1);
            }
        }
        i += 1;
    }
    
    // Handle mutually exclusive modes
    if harness_json_mode {
        print_harness_json(plugin_dir, &config.target)?;
        return Ok(());
    }
    
    if test_mode {
        let result = test_plugin(plugin_dir, &config)?;
        
        println!("\n=== Test Results ===");
        println!("Plugin: {}", result.plugin_id);
        println!("Passed: {}", result.passed);
        println!("Duration: {}ms", result.duration_ms);
        
        if !result.errors.is_empty() {
            println!("\nErrors:");
            for error in &result.errors {
                println!("  ❌ {}", error);
            }
        }
        
        if !result.warnings.is_empty() {
            println!("\nWarnings:");
            for warning in &result.warnings {
                println!("  ⚠ {}", warning);
            }
        }
        
        if !result.passed {
            std::process::exit(1);
        }
        
        return Ok(());
    }
    
    if register_mode {
        register_plugin(plugin_dir, &config)?;
        return Ok(());
    }
    
    // Default: test and register
    let result = test_plugin(plugin_dir, &config)?;
    
    if result.passed {
        println!("\n✓ Plugin passed validation");
        register_plugin(plugin_dir, &config)?;
    } else {
        println!("\n❌ Plugin validation failed");
        for error in &result.errors {
            println!("  {}", error);
        }
        std::process::exit(1);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_validate_manifest_valid() {
        let manifest = PluginManifest {
            id: "com.example.test".to_string(),
            version: "1.0.0".to_string(),
            name: "Test Plugin".to_string(),
            description: "A test plugin".to_string(),
            long_description: None,
            author: "Test Author".to_string(),
            author_email: None,
            plugin_type: "effect".to_string(),
            api_version: 1,
            capabilities: vec!["CanvasRead".to_string()],
            wasm_entry: "_start".to_string(),
            icon: None,
            homepage: None,
            license: "AGPL-3.0-or-later".to_string(),
            tags: vec!["test".to_string()],
        };
        
        assert!(validate_manifest(&manifest).is_ok());
    }
    
    #[test]
    fn test_validate_manifest_missing_id() {
        let manifest = PluginManifest {
            id: String::new(),
            version: "1.0.0".to_string(),
            name: "Test Plugin".to_string(),
            description: "A test plugin".to_string(),
            long_description: None,
            author: "Test Author".to_string(),
            author_email: None,
            plugin_type: "effect".to_string(),
            api_version: 1,
            capabilities: Vec::new(),
            wasm_entry: "_start".to_string(),
            icon: None,
            homepage: None,
            license: "AGPL-3.0-or-later".to_string(),
            tags: Vec::new(),
        };
        
        assert!(validate_manifest(&manifest).is_err());
    }
    
    #[test]
    fn test_generate_harness_manifest() {
        let manifest = PluginManifest {
            id: "com.example.test".to_string(),
            version: "1.0.0".to_string(),
            name: "Test Plugin".to_string(),
            description: "A test plugin".to_string(),
            long_description: None,
            author: "Test Author".to_string(),
            author_email: None,
            plugin_type: "effect".to_string(),
            api_version: 1,
            capabilities: vec!["CanvasRead".to_string()],
            wasm_entry: "_start".to_string(),
            icon: None,
            homepage: None,
            license: "AGPL-3.0-or-later".to_string(),
            tags: vec!["test".to_string()],
        };
        
        let harness = generate_harness_manifest(&manifest, "paint-type");
        
        assert_eq!(harness.schema, "paint-type-harness/v1");
        assert_eq!(harness.plugin_id, "com.example.test");
        assert!(harness.service_id.contains("paint-type-plugin-com-example-test"));
        assert!(!harness.default_endpoint.is_empty());
    }
}

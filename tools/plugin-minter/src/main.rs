// SPDX-License-Identifier: AGPL-3.0-or-later
//
// plugin-minter -- Scaffolds a new paint.type plugin from a minter.toml config.
//
// Usage:
//   cargo run --manifest-path tools/plugin-minter/Cargo.toml -- <minter.toml> [--dest <path>]
//
// Inspired by:
//   - boJ-server/cartridges/tools/cartridge-minter/mint.ts
//   - panll/contracts/minter.toml

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Minter configuration from minter.toml
#[derive(Debug, Deserialize, Clone)]
struct MinterConfig {
    // Required fields
    name: String,
    description: String,
    version: String,
    id: String,
    author: String,
    plugin_type: String,
    wasm_entry: String,
    license: String,
    
    // Optional fields
    #[serde(default)]
    author_email: Option<String>,
    #[serde(default)]
    homepage: Option<String>,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default = "default_api_version")]
    api_version: u32,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    long_description: Option<String>,
}

impl Default for MinterConfig {
    fn default() -> Self {
        MinterConfig {
            name: String::new(),
            description: String::new(),
            version: "0.1.0".to_string(),
            id: String::new(),
            author: String::new(),
            plugin_type: "effect".to_string(),
            wasm_entry: "_start".to_string(),
            license: "AGPL-3.0-or-later".to_string(),
            author_email: None,
            homepage: None,
            icon: None,
            api_version: 1,
            capabilities: Vec::new(),
            tags: Vec::new(),
            long_description: None,
        }
    }
}

fn default_api_version() -> u32 {
    1
}

/// Represents the plugin manifest to be written
#[derive(Debug, Clone)]
struct PluginManifest {
    id: String,
    version: String,
    name: String,
    description: String,
    author: String,
    author_email: Option<String>,
    plugin_type: String,
    wasm_entry: String,
    api_version: u32,
    license: String,
    homepage: Option<String>,
    icon: Option<String>,
    capabilities: Vec<String>,
    tags: Vec<String>,
    long_description: Option<String>,
}

/// Convert domain notation to crate name (e.g., com.example.plugin -> paint-type-plugin)
fn id_to_crate_name(id: &str) -> String {
    id.replace('.', "-")
        .to_lowercase()
        .trim_matches('-')
        .to_string()
}

/// Convert plugin name to CamelCase
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = true;
    
    for c in s.chars() {
        if c == '_' || c == '-' || c == '.' || c == ' ' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c.to_ascii_lowercase());
        }
    }
    
    result
}

/// Validate the minter configuration
fn validate_config(cfg: &MinterConfig) -> Result<()> {
    let mut errors = Vec::new();
    
    if cfg.name.is_empty() {
        errors.push("missing 'name' field");
    }
    if cfg.description.is_empty() {
        errors.push("missing 'description' field");
    }
    if cfg.version.is_empty() {
        errors.push("missing 'version' field");
    }
    if cfg.id.is_empty() {
        errors.push("missing 'id' field");
    } else if !cfg.id.contains('.') {
        errors.push("'id' must be in reverse domain notation (e.g., com.example.plugin)");
    }
    if cfg.author.is_empty() {
        errors.push("missing 'author' field");
    }
    if cfg.plugin_type.is_empty() {
        errors.push("missing 'plugin_type' field");
    } else if cfg.plugin_type != "effect" && cfg.plugin_type != "tool" {
        errors.push("'plugin_type' must be 'effect' or 'tool'");
    }
    if cfg.wasm_entry.is_empty() {
        errors.push("missing 'wasm_entry' field");
    }
    if cfg.license.is_empty() {
        errors.push("missing 'license' field");
    }
    
    if !errors.is_empty() {
        anyhow::bail!("minter.toml validation failed:\n  - {}", errors.join("\n  - "));
    }
    
    Ok(())
}

/// Determine the destination path for the plugin
fn destination_for(cfg: &MinterConfig, explicit_dest: Option<&str>) -> PathBuf {
    let dest = explicit_dest.map(PathBuf::from);
    
    if let Some(d) = dest {
        return d;
    }
    
    // Default: src/plugins/available/<plugin_id>/
    let repo_root = Path::new("tools/plugin-minter")
        .ancestors()
        .nth(2)
        .unwrap_or(Path::new(""));
    
    repo_root.join("src").join("plugins").join("available").join(&cfg.id)
}

/// Copy a directory tree recursively
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(&entry.path(), dst.join(entry.file_name()))?;
        }
    }
    
    Ok(())
}

/// Replace placeholders in a file with actual values
fn replace_placeholders(content: &str, cfg: &MinterConfig, crate_name: &str) -> String {
    let camel_name = to_camel_case(&cfg.name);
    
    content
        .replace("PLUGIN_ID", &cfg.id)
        .replace("PLUGIN_VERSION", &cfg.version)
        .replace("PLUGIN_NAME", &cfg.name)
        .replace("PLUGIN_DESCRIPTION", &cfg.description)
        .replace("PLUGIN_AUTHOR", &cfg.author)
        .replace("PLUGIN_TYPE", &cfg.plugin_type)
        .replace("PLUGIN_WASM_ENTRY", &cfg.wasm_entry)
        .replace("PLUGIN_API_VERSION", &cfg.api_version.to_string())
        .replace("PLUGIN_LICENSE", &cfg.license)
        .replace("PLUGIN_CRATE_NAME", crate_name)
        .replace("PLUGIN_CAMEL_NAME", &camel_name)
        .replace("PLUGIN_AUTHOR_EMAIL", cfg.author_email.as_deref().unwrap_or(""))
        .replace("PLUGIN_HOMEPAGE", cfg.homepage.as_deref().unwrap_or(""))
        .replace("PLUGIN_ICON", cfg.icon.as_deref().unwrap_or(""))
}

/// Mint a new plugin from configuration
fn mint(config_path: &Path, explicit_dest: Option<&str>) -> Result<()> {
    // Read and parse the minter.toml
    let toml_content = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config file: {}", config_path.display()))?;
    
    let cfg: MinterConfig = toml::from_str(&toml_content)
        .with_context(|| format!("failed to parse config file: {}", config_path.display()))?;
    
    validate_config(&cfg)?;
    
    let crate_name = id_to_crate_name(&cfg.id);
    let dest = destination_for(&cfg, explicit_dest);
    
    // Check if destination already exists
    if dest.exists() {
        anyhow::bail!("destination already exists: {}", dest.display());
    }
    
    // Create the template directory path
    let template_dir = Path::new("tools/plugin-minter/templates/plugin_template");
    
    // Copy the template tree
    copy_dir_all(template_dir, &dest)?;
    
    // Rename the template directory from plugin_template to the plugin name
    let temp_template_dir = dest.join("plugin_template");
    let final_dest = dest.join(&cfg.name);
    if temp_template_dir.exists() {
        fs::rename(&temp_template_dir, &final_dest)?;
    }
    
    let actual_dest = if final_dest.exists() { final_dest } else { dest.clone() };
    
    // Now replace placeholders in all files recursively
    let mut stack = vec![actual_dest.clone()];
    
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if entry.file_type()?.is_dir() {
                stack.push(path);
                continue;
            }
            
            // Read the file
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            
            // Check if content has any placeholders
            if content.contains("PLUGIN_") {
                let replaced = replace_placeholders(&content, &cfg, &crate_name);
                fs::write(&path, replaced)?;
            }
        }
    }
    
    // Copy the original minter.toml to the plugin directory for reproducibility
    let minter_dest = dest.join("minter.toml");
    fs::copy(config_path, minter_dest)?;
    
    println!("✓ Minted {} → {}", cfg.name, dest.display());
    println!("  Plugin ID: {}", cfg.id);
    println!("  Type: {}", cfg.plugin_type);
    println!("  Version: {}", cfg.version);
    println!("\nNext steps:");
    println!("  1. Edit Cargo.toml to add dependencies");
    println!("  2. Edit src/lib.rs to implement your plugin");
    println!("  3. cargo build --release");
    
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        println!("Usage: plugin-minter <minter.toml> [--dest <path>]");
        println!();
        println!("Scaffolds a new paint.type plugin by copying templates/");
        println!("and customising the manifest based on the minter.toml config.");
        println!();
        println!("Example:");
        println!("  cargo run --manifest-path tools/plugin-minter/Cargo.toml -- minter.toml");
        println!("  cargo run --manifest-path tools/plugin-minter/Cargo.toml -- minter.toml --dest ./my-plugin");
        std::process::exit(if args.len() < 2 { 1 } else { 0 });
    }
    
    let config_path = Path::new(&args[1]);
    
    // Find --dest argument
    let mut explicit_dest: Option<&str> = None;
    for (i, arg) in args.iter().skip(2).enumerate() {
        if arg == "--dest" && i + 1 < args.len() - 2 {
            explicit_dest = Some(&args[i + 2]);
            break;
        }
    }
    
    // Change to repo root
    let minter_path = Path::new("tools/plugin-minter").canonicalize()?;
    let repo_root = minter_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("Could not determine repository root"))?;
    std::env::set_current_dir(repo_root)?;
    
    mint(config_path, explicit_dest)?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_id_to_crate_name() {
        assert_eq!(id_to_crate_name("com.example.plugin"), "com-example-plugin");
        assert_eq!(id_to_crate_name("org.paint.type.blur"), "org-paint-type-blur");
    }
    
    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("blur_effect"), "BlurEffect");
        assert_eq!(to_camel_case("gaussian-blur"), "GaussianBlur");
        assert_eq!(to_camel_case("My Plugin"), "MyPlugin");
    }
}

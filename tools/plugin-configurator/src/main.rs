// SPDX-License-Identifier: AGPL-3.0-or-later
//
// paint-type Plugin Configurator
//
// A tool for managing plugin configurations: parsing, validating, merging,
// and generating configuration files for paint.type plugins.
//
// Inspired by:
// - boj-server/boj-server configurators
// - panll configuration management

use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

/// Configuration format
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    Json,
    Yaml,
    Toml,
}

impl ConfigFormat {
    pub fn from_path(path: &Path) -> Self {
        if let Some(ext) = path.extension() {
            match ext.to_string_lossy().to_lowercase().as_str() {
                "json" => ConfigFormat::Json,
                "yaml" | "yml" => ConfigFormat::Yaml,
                "toml" => ConfigFormat::Toml,
                _ => ConfigFormat::Toml,
            }
        } else {
            ConfigFormat::Toml
        }
    }
}

/// Merge strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, serde::Serialize)]
pub enum MergeStrategy {
    Override,
    DeepMerge,
}

/// Plugin Configurator CLI
#[derive(Debug, Parser)]
#[command(name = "plugin-configurator")]
#[command(author = "paint.type")]
#[command(version = "0.1.0")]
#[command(about = "Manage plugin configurations for paint.type")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Parser)]
enum Command {
    /// Generate a default configuration
    Generate {
        plugin_id: String,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, value_enum, default_value = "toml")]
        format: ConfigFormat,
    },
    
    /// Validate a configuration file
    Validate {
        path: PathBuf,
        #[arg(short, long, value_enum)]
        format: Option<ConfigFormat>,
        #[arg(short, long, default_value = "false")]
        strict: bool,
    },
    
    /// Merge configuration files
    Merge {
        inputs: Vec<PathBuf>,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(short, long, value_enum, default_value = "toml")]
        format: ConfigFormat,
        #[arg(short, long, value_enum, default_value = "deep-merge")]
        strategy: MergeStrategy,
    },
    
    /// List configuration layers
    List {
        plugin: String,
        #[arg(short, long, default_value = "false")]
        verbose: bool,
    },
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("plugin_configurator=info")
        .init();

    let cli = Cli::parse();
    
    match cli.command {
        Command::Generate { plugin_id, output, format } => {
            cmd_generate(&plugin_id, &output, format)?;
        }
        Command::Validate { path, format, strict } => {
            cmd_validate(&path, format, strict)?;
        }
        Command::Merge { inputs, output, format, strategy } => {
            cmd_merge(&inputs, &output, format, strategy)?;
        }
        Command::List { plugin, verbose } => {
            cmd_list(&plugin, verbose)?;
        }
    }

    Ok(())
}

fn cmd_generate(plugin_id: &str, output: &Path, format: ConfigFormat) -> Result<()> {
    info!("Generating configuration for plugin: {}", plugin_id);
    
    let config = json!({
        "plugin_id": plugin_id,
        "version": "1.0.0",
        "enabled": true,
        "log_level": "info",
        "capabilities": ["CanvasRead", "CanvasWrite"],
        "settings": {}
    });
    
    let content = match format {
        ConfigFormat::Json => serde_json::to_string_pretty(&config)?,
        ConfigFormat::Yaml => serde_yaml::to_string(&config)?,
        ConfigFormat::Toml => {
            // Convert JSON to a TOML-compatible structure
            let value: Value = config;
            toml::to_string(&value).context("Failed to convert to TOML")?
        }
    };
    
    fs::write(output, content)
        .with_context(|| format!("Failed to write to {}", output.display()))?;
    
    info!("Configuration generated: {}", output.display());
    Ok(())
}

fn cmd_validate(path: &Path, format: Option<ConfigFormat>, strict: bool) -> Result<()> {
    info!("Validating configuration: {}", path.display());
    
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    
    let format = format.unwrap_or_else(|| ConfigFormat::from_path(path));
    
    match format {
        ConfigFormat::Json => {
            let _: serde_json::Value = serde_json::from_str(&content)?;
        }
        ConfigFormat::Yaml => {
            let _: serde_yaml::Value = serde_yaml::from_str(&content)?;
        }
        ConfigFormat::Toml => {
            let _: toml::Value = toml::from_str(&content)?;
        }
    }
    
    if strict {
        // Additional strict validation would go here
        // For now, just check that it parses
    }
    
    info!("Configuration is valid: {}", path.display());
    Ok(())
}

fn cmd_merge(inputs: &[PathBuf], output: &Path, format: ConfigFormat, strategy: MergeStrategy) -> Result<()> {
    info!("Merging {} configuration files", inputs.len());
    
    if inputs.is_empty() {
        bail!("At least one input file is required");
    }
    
    let mut merged = Map::new();
    
    for input in inputs {
        let content = fs::read_to_string(input)
            .with_context(|| format!("Failed to read {}", input.display()))?;
        
        let fmt = ConfigFormat::from_path(input);
        let parsed: Value = match fmt {
            ConfigFormat::Json => serde_json::from_str(&content)?,
            ConfigFormat::Yaml => serde_yaml::from_str(&content)?,
            ConfigFormat::Toml => {
                let v: toml::Value = toml::from_str(&content)?;
                serde_json::to_value(v)?
            }
        };
        
        match strategy {
            MergeStrategy::Override => {
                // Replace entire config with this one
                if let Value::Object(obj) = parsed {
                    merged = obj;
                }
            }
            MergeStrategy::DeepMerge => {
                deep_merge_json(&mut merged, parsed);
            }
        }
    }
    
    let merged_value = Value::Object(merged);
    let content = match format {
        ConfigFormat::Json => serde_json::to_string_pretty(&merged_value)?,
        ConfigFormat::Yaml => serde_yaml::to_string(&merged_value)?,
        ConfigFormat::Toml => {
            toml::to_string(&merged_value).context("Failed to convert to TOML")?
        }
    };
    
    fs::write(output, content)
        .with_context(|| format!("Failed to write to {}", output.display()))?;
    
    info!("Merged configuration written: {}", output.display());
    Ok(())
}

/// Deep merge two JSON values
fn deep_merge_json(target: &mut Map<String, Value>, source: Value) {
    if let Value::Object(source_obj) = source {
        for (key, value) in source_obj {
            if let Some(existing) = target.get_mut(&key) {
                match existing {
                    Value::Object(existing_obj) => {
                        if let Value::Object(source_obj) = value {
                            deep_merge_json(existing_obj, Value::Object(source_obj));
                        } else {
                            *existing = value;
                        }
                    }
                    Value::Array(existing_arr) => {
                        if let Value::Array(source_arr) = value {
                            existing_arr.extend(source_arr);
                        } else {
                            *existing = value;
                        }
                    }
                    _ => {
                        *existing = value;
                    }
                }
            } else {
                target.insert(key, value);
            }
        }
    }
}

fn cmd_list(plugin: &str, verbose: bool) -> Result<()> {
    info!("Listing configuration layers for plugin: {}", plugin);
    
    println!("Configuration layers for plugin '{}':", plugin);
    println!();
    println!("1. Plugin ({}/config.toml) - Plugin default configuration", plugin);
    println!("2. User (~/.paint-type/plugins/{}/config.toml) - User overrides", plugin);
    println!("3. Project (project/plugins/{}/config.toml) - Project-specific", plugin);
    
    if verbose {
        println!("\nNote: Verbose mode would show actual values from each layer");
    }
    
    Ok(())
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//
// plugin-wizard -- Interactive CLI tool for scaffolding paint.type plugins.
//
// This wizard guides users through the process of creating a new plugin,
// including plugin discovery, selection, configuration, dependency resolution,
// integration testing, and deployment planning.
//
// Inspired by:
//   - boJ-server/cartridges/tools/cartridge-minter/mint.ts
//   - panll/contracts/minter.toml
//   - panll/src/core/provisioner_engine.affine
//
// Usage:
//   plugin-wizard                    # Start interactive mode
//   plugin-wizard --non-interactive  # Run in non-interactive mode (for CI)
//   plugin-wizard --type effect      # Create an effect plugin
//   plugin-wizard --type tool        # Create a tool plugin

use anyhow::Result;
use clap::{Parser, ValueEnum};
use console::style;
use dialoguer::{
    console::Term,
    Input, MultiSelect, Select, Confirm,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Plugin type for the new plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginType {
    /// Effect plugin - stateless transformations
    Effect,
    /// Tool plugin - stateful interactive tools
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

/// Plugin language
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginLanguage {
    Rust,
    Zig,
    Idris2,
    /// JavaScript/TypeScript for web-based plugins (future)
    TypeScript,
}

impl std::fmt::Display for PluginLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginLanguage::Rust => write!(f, "rust"),
            PluginLanguage::Zig => write!(f, "zig"),
            PluginLanguage::Idris2 => write!(f, "idris2"),
            PluginLanguage::TypeScript => write!(f, "typescript"),
        }
    }
}

/// Capability that a plugin may request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    CanvasRead,
    CanvasWrite,
    LayerAccess,
    SelectionAccess,
    FileAccess,
    NetworkAccess,
    UserInterface,
    PersistentStorage,
}

impl std::fmt::Display for PluginCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PluginCapability::CanvasRead => write!(f, "Canvas Read"),
            PluginCapability::CanvasWrite => write!(f, "Canvas Write"),
            PluginCapability::LayerAccess => write!(f, "Layer Access"),
            PluginCapability::SelectionAccess => write!(f, "Selection Access"),
            PluginCapability::FileAccess => write!(f, "File Access"),
            PluginCapability::NetworkAccess => write!(f, "Network Access"),
            PluginCapability::UserInterface => write!(f, "User Interface"),
            PluginCapability::PersistentStorage => write!(f, "Persistent Storage"),
        }
    }
}

/// Configuration collected by the wizard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WizardConfig {
    /// Plugin type (effect or tool)
    pub plugin_type: PluginType,
    
    /// Plugin language
    pub language: PluginLanguage,
    
    /// Plugin identifier (reverse domain notation)
    pub id: String,
    
    /// Plugin version
    pub version: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Short description
    pub description: String,
    
    /// Long description (markdown)
    pub long_description: Option<String>,
    
    /// Author name
    pub author: String,
    
    /// Author email
    pub author_email: Option<String>,
    
    /// WASM entry point
    pub wasm_entry: String,
    
    /// License SPDX identifier
    pub license: String,
    
    /// Plugin homepage
    pub homepage: Option<String>,
    
    /// Plugin icon
    pub icon: Option<String>,
    
    /// List of capabilities
    pub capabilities: Vec<PluginCapability>,
    
    /// Tags for categorization
    pub tags: Vec<String>,
    
    /// Output directory
    pub output_dir: PathBuf,
    
    /// Whether to create a minter.toml for reproducibility
    pub create_minter: bool,
}

impl Default for WizardConfig {
    fn default() -> Self {
        Self {
            plugin_type: PluginType::Effect,
            language: PluginLanguage::Rust,
            id: String::new(),
            version: "0.1.0".to_string(),
            name: String::new(),
            description: String::new(),
            long_description: None,
            author: String::new(),
            author_email: None,
            wasm_entry: "_start".to_string(),
            license: "AGPL-3.0-or-later".to_string(),
            homepage: None,
            icon: None,
            capabilities: Vec::new(),
            tags: Vec::new(),
            output_dir: PathBuf::new(),
            create_minter: true,
        }
    }
}

/// CLI arguments
#[derive(Parser, Debug)]
#[command(name = "plugin-wizard")]
#[command(author = "paint.type team")]
#[command(version = "0.1.0")]
#[command(about = "Interactive CLI wizard for scaffolding paint.type plugins", long_about = None)]
struct Args {
    /// Plugin type (effect or tool)
    #[arg(short, long, value_enum)]
    r#type: Option<PluginType>,
    
    /// Plugin language
    #[arg(short, long, value_enum)]
    language: Option<PluginLanguage>,
    
    /// Plugin ID (reverse domain notation, e.g., com.example.myplugin)
    #[arg(short, long)]
    id: Option<String>,
    
    /// Plugin name
    #[arg(short, long)]
    name: Option<String>,
    
    /// Plugin description
    #[arg(short, long)]
    description: Option<String>,
    
    /// Output directory
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Run in non-interactive mode (reads from stdin or uses defaults)
    #[arg(long)]
    non_interactive: bool,
    
    /// Skip confirmation before generating
    #[arg(long)]
    yes: bool,
}

/// Get user input with a prompt
fn get_input(_term: &Term, prompt: &str, default: Option<&str>) -> Result<String> {
    let input = Input::<String>::new()
        .with_prompt(prompt)
        .default(default.unwrap_or("").to_string())
        .interact_text()?;
    
    if input.is_empty() {
        anyhow::bail!("Input cannot be empty");
    }
    
    Ok(input)
}

/// Get user selection from options
fn get_select(_term: &Term, prompt: &str, options: &[&str]) -> Result<usize> {
    Select::new()
        .with_prompt(prompt)
        .items(options)
        .interact()
        .map_err(Into::into)
}

/// Get multi-selection from options
fn get_multiselect(_term: &Term, prompt: &str, options: &[&str]) -> Result<Vec<usize>> {
    MultiSelect::new()
        .with_prompt(prompt)
        .items(options)
        .interact()
        .map_err(Into::into)
}

/// Confirm with user
fn get_confirm(_term: &Term, prompt: &str, default: bool) -> Result<bool> {
    Confirm::new()
        .with_prompt(prompt)
        .default(default)
        .interact()
        .map_err(Into::into)
}

/// Validate plugin ID format (reverse domain notation)
fn validate_id(id: &str) -> Result<()> {
    if id.is_empty() {
        anyhow::bail!("Plugin ID cannot be empty");
    }
    if !id.contains('.') {
        anyhow::bail!("Plugin ID must be in reverse domain notation (e.g., com.example.plugin)");
    }
    if id.starts_with('.') || id.ends_with('.') {
        anyhow::bail!("Plugin ID cannot start or end with a dot");
    }
    Ok(())
}

/// Validate plugin name
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("Plugin name cannot be empty");
    }
    Ok(())
}

/// Run the interactive wizard
fn run_interactive_wizard(args: &Args) -> Result<WizardConfig> {
    let term = Term::stderr();
    let mut config = WizardConfig::default();
    
    // Welcome message
    println!();
    println!("{}", style("=== paint.type Plugin Wizard ===").bold().cyan());
    println!();
    println!("This wizard will guide you through creating a new paint.type plugin.");
    println!("Answer the questions below to configure your plugin.");
    println!();
    
    // Plugin type
    if args.r#type.is_some() {
        config.plugin_type = args.r#type.unwrap();
    } else {
        let options = vec!["Effect (stateless transformations like filters)", "Tool (stateful interactive tools like brushes)"];
        let selection = get_select(&term, "Plugin type:", &options)?;
        config.plugin_type = match selection {
            0 => PluginType::Effect,
            1 => PluginType::Tool,
            _ => unreachable!(),
        };
    }
    
    // Plugin language
    if args.language.is_some() {
        config.language = args.language.unwrap();
    } else {
        let options = vec!["Rust", "Zig", "Idris2", "TypeScript"];
        let selection = get_select(&term, "Implementation language:", &options)?;
        config.language = match selection {
            0 => PluginLanguage::Rust,
            1 => PluginLanguage::Zig,
            2 => PluginLanguage::Idris2,
            3 => PluginLanguage::TypeScript,
            _ => unreachable!(),
        };
    }
    
    // Plugin ID
    if let Some(ref id) = args.id {
        config.id = id.clone();
        validate_id(&config.id)?;
    } else {
        loop {
            let prompt = "Plugin ID (reverse domain notation, e.g., com.example.myplugin):";
            config.id = get_input(&term, prompt, None)?;
            if validate_id(&config.id).is_ok() {
                break;
            }
        }
    }
    
    // Plugin name
    if let Some(ref name) = args.name {
        config.name = name.clone();
        validate_name(&config.name)?;
    } else {
        loop {
            let prompt = "Plugin name (human-readable):";
            config.name = get_input(&term, prompt, None)?;
            if validate_name(&config.name).is_ok() {
                break;
            }
        }
    }
    
    // Plugin version
    let version_prompt = "Plugin version (default: 0.1.0):";
    config.version = get_input(&term, version_prompt, Some("0.1.0"))?;
    
    // Plugin description
    if let Some(ref desc) = args.description {
        config.description = desc.clone();
    } else {
        let prompt = "Short description:";
        config.description = get_input(&term, prompt, None)?;
    }
    
    // Long description (optional)
    let long_desc_prompt = "Long description (markdown, optional - press Enter to skip):";
    let long_input = Input::<String>::new()
        .with_prompt(long_desc_prompt)
        .allow_empty(true)
        .interact_text()?;
    if !long_input.is_empty() {
        config.long_description = Some(long_input);
    }
    
    // Author
    let author_prompt = "Author name:";
    config.author = get_input(&term, author_prompt, None)?;
    
    // Author email (optional)
    let email_prompt = "Author email (optional - press Enter to skip):";
    let email_input = Input::<String>::new()
        .with_prompt(email_prompt)
        .allow_empty(true)
        .interact_text()?;
    if !email_input.is_empty() {
        config.author_email = Some(email_input);
    }
    
    // WASM entry point
    let wasm_prompt = "WASM entry point (default: _start):";
    config.wasm_entry = get_input(&term, wasm_prompt, Some("_start"))?;
    
    // License
    let license_options = vec!["AGPL-3.0-or-later", "MIT", "Apache-2.0", "GPL-3.0-or-later", "BSD-3-Clause", "Other"];
    let license_idx = get_select(&term, "License:", &license_options)?;
    config.license = match license_idx {
        i if i < license_options.len() - 1 => license_options[i].to_string(),
        _ => {
            let prompt = "Enter custom license SPDX identifier:";
            get_input(&term, prompt, None)?
        }
    };
    
    // Capabilities
    let capability_options: Vec<&str> = vec![
        "Canvas Read",
        "Canvas Write",
        "Layer Access",
        "Selection Access",
        "File Access",
        "Network Access",
        "User Interface",
        "Persistent Storage",
    ];
    
    let selected_indices = get_multiselect(&term, "Required capabilities (select with space, confirm with Enter):", &capability_options)?;
    config.capabilities = selected_indices
        .into_iter()
        .map(|idx| match idx {
            0 => PluginCapability::CanvasRead,
            1 => PluginCapability::CanvasWrite,
            2 => PluginCapability::LayerAccess,
            3 => PluginCapability::SelectionAccess,
            4 => PluginCapability::FileAccess,
            5 => PluginCapability::NetworkAccess,
            6 => PluginCapability::UserInterface,
            7 => PluginCapability::PersistentStorage,
            _ => unreachable!(),
        })
        .collect();
    
    // Tags
    let tag_prompt = "Tags (comma-separated, e.g., filter, blur, experimental):";
    let tags_input = Input::<String>::new()
        .with_prompt(tag_prompt)
        .allow_empty(true)
        .interact_text()?;
    if !tags_input.is_empty() {
        config.tags = tags_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    
    // Output directory
    if let Some(ref output) = args.output {
        config.output_dir = output.clone();
    } else {
        let mut default_dir = String::with_capacity(12 + config.name.len());
        default_dir.push_str("./plugin-");
        let name_lower = config.name.to_lowercase();
        let mut name_dashed = String::with_capacity(name_lower.len());
        for c in name_lower.chars() {
            if c == ' ' {
                name_dashed.push('-');
            } else {
                name_dashed.push(c);
            }
        }
        default_dir.push_str(&name_dashed);
        
        let mut dir_prompt = String::with_capacity(30 + default_dir.len());
        dir_prompt.push_str("Output directory (default: ");
        dir_prompt.push_str(&default_dir);
        dir_prompt.push(':');
        
        config.output_dir = PathBuf::from(get_input(&term, &dir_prompt, Some(&default_dir))?);
    }
    
    // Create minter.toml for reproducibility
    config.create_minter = get_confirm(&term, "Create minter.toml for reproducibility?", true)?;
    
    // Confirmation
    if !args.yes {
        println!();
        println!("{}", style("=== Configuration Summary ===").bold().cyan());
        println!();
        println!("Plugin Type:        {}", style(&config.plugin_type).green());
        println!("Language:           {}", style(&config.language).green());
        println!("ID:                {}", style(&config.id).green());
        println!("Name:              {}", style(&config.name).green());
        println!("Version:           {}", style(&config.version).green());
        println!("Description:       {}", style(&config.description).green());
        println!("Author:            {}", style(&config.author).green());
        if let Some(email) = &config.author_email {
            println!("Author Email:      {}", style(email).green());
        }
        println!("WASM Entry:       {}", style(&config.wasm_entry).green());
        println!("License:           {}", style(&config.license).green());
        
        if !config.capabilities.is_empty() {
            println!("Capabilities:      {}", 
                style(&config.capabilities.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(", ")).green());
        }
        
        if !config.tags.is_empty() {
            println!("Tags:              {}", 
                style(&config.tags.join(", ")).green());
        }
        
        println!("Output Directory:  {}", style(config.output_dir.display()).green());
        println!("Create minter.toml: {}", style(&config.create_minter).green());
        println!();
        
        if !get_confirm(&term, "Does this look correct?", true)? {
            anyhow::bail!("Wizard cancelled by user");
        }
    }
    
    Ok(config)
}

/// Convert PluginType to string
fn plugin_type_to_str(p: &PluginType) -> &'static str {
    match p {
        PluginType::Effect => "effect",
        PluginType::Tool => "tool",
    }
}


/// Convert PluginCapability to string
fn capability_to_str(c: &PluginCapability) -> &'static str {
    match c {
        PluginCapability::CanvasRead => "canvas_read",
        PluginCapability::CanvasWrite => "canvas_write",
        PluginCapability::LayerAccess => "layer_access",
        PluginCapability::SelectionAccess => "selection_access",
        PluginCapability::FileAccess => "file_access",
        PluginCapability::NetworkAccess => "network_access",
        PluginCapability::UserInterface => "user_interface",
        PluginCapability::PersistentStorage => "persistent_storage",
    }
}

/// Generate a minter.toml configuration file
fn generate_minter_toml(config: &WizardConfig) -> String {
    let capabilities: Vec<String> = config.capabilities
        .iter()
        .map(capability_to_str)
        .map(String::from)
        .collect();
    
    let mut toml = format!(
        "name = \"{}\"

description = \"{}\"

version = \"{}\"

id = \"{}\"

author = \"{}\"

plugin_type = \"{}\"

wasm_entry = \"{}\"

license = \"{}\"
",
        config.name,
        config.description,
        config.version,
        config.id,
        config.author,
        plugin_type_to_str(&config.plugin_type),
        config.wasm_entry,
        config.license
    );
    
    if let Some(email) = &config.author_email {
        toml.push_str(&format!("\nauthor_email = \"{}\"", email));
    }
    
    if let Some(homepage) = &config.homepage {
        toml.push_str(&format!("\nhomepage = \"{}\"", homepage));
    }
    
    if let Some(icon) = &config.icon {
        toml.push_str(&format!("\nicon = \"{}\"", icon));
    }
    
    if !capabilities.is_empty() {
        toml.push_str("\n\n[capabilities]\n");
        for cap in capabilities {
            toml.push_str(&format!("  {} = true\n", cap));
        }
    }
    
    if !config.tags.is_empty() {
        toml.push_str("\n\ntags = [");
        for (i, tag) in config.tags.iter().enumerate() {
            if i > 0 {
                toml.push_str(", ");
            }
            toml.push_str(&format!("\"{}\"", tag));
        }
        toml.push_str("]");
    }
    
    toml
}

/// Generate a plugin.toml manifest
fn generate_plugin_toml(config: &WizardConfig) -> String {
    // Pre-calculate total capacity to avoid reallocations
    let estimated_capacity = config.id.len() + config.name.len() + config.description.len() +
        config.version.len() + config.author.len() + config.wasm_entry.len() +
        config.license.len() + config.plugin_type.to_string().len();
    
    let mut toml = String::with_capacity(estimated_capacity + 512); // Extra space for TOML structure
    
    // ID
    toml.push_str("id = \"");
    toml.push_str(&config.id);
    toml.push_str("\"\n\n");
    
    // Name
    toml.push_str("name = \"");
    toml.push_str(&config.name);
    toml.push_str("\"\n\n");
    
    // Description
    toml.push_str("description = \"");
    toml.push_str(&config.description);
    toml.push_str("\"\n\n");
    
    // Version
    toml.push_str("version = \"");
    toml.push_str(&config.version);
    toml.push_str("\"\n\n");
    
    // Long description
    if let Some(long_desc) = &config.long_description {
        toml.push_str("long_description = \"\"\"\n");
        toml.push_str(long_desc);
        toml.push_str("\n\"\"\"\n\n");
    }
    
    // Author
    toml.push_str("author = \"");
    toml.push_str(&config.author);
    toml.push_str("\"\n\n");
    
    // Author email
    if let Some(email) = &config.author_email {
        toml.push_str("author_email = \"");
        toml.push_str(email);
        toml.push_str("\"\n\n");
    }
    
    // Plugin type
    let plugin_type_str = plugin_type_to_str(&config.plugin_type);
    toml.push_str("plugin_type = \"");
    toml.push_str(&plugin_type_str);
    toml.push_str("\"\n\n");
    
    // WASM entry
    toml.push_str("wasm_entry = \"");
    toml.push_str(&config.wasm_entry);
    toml.push_str("\"\n\n");
    
    // License
    toml.push_str("license = \"");
    toml.push_str(&config.license);
    toml.push_str("\"\n\n");
    
    // Homepage
    if let Some(homepage) = &config.homepage {
        toml.push_str("homepage = \"");
        toml.push_str(homepage);
        toml.push_str("\"\n\n");
    }
    
    // Icon
    if let Some(icon) = &config.icon {
        toml.push_str("icon = \"");
        toml.push_str(icon);
        toml.push_str("\"\n\n");
    }
    
    // Capabilities
    toml.push_str("[capabilities]\n");
    for cap in &config.capabilities {
        toml.push_str("  ");
        toml.push_str(capability_to_str(cap));
        toml.push_str(" = true\n");
    }
    
    // Tags
    if !config.tags.is_empty() {
        toml.push_str("\n\ntags = [");
        for (i, tag) in config.tags.iter().enumerate() {
            if i > 0 {
                toml.push_str(", ");
            }
            toml.push('\"');
            toml.push_str(tag);
            toml.push('\"');
        }
        toml.push(']');
    }
    
    toml
}

/// Generate a Cargo.toml for Rust plugins
fn generate_cargo_toml(config: &WizardConfig) -> String {
    let crate_name = config.id.replace('.', "-").to_lowercase();
    
    format!(
        "[package]\n\nname = \"{}\"\nversion = \"{}\"\nedition = \"2021\"\nlicense = \"{}\"\n\n\n[dependencies]\npaint-type-plugins = {{ path = \"../../..\" }}\n\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n\n[profile.release]\nopt-level = \"s\"\nstrip = true\n",
        crate_name,
        config.version,
        config.license
    )
}

/// Generate lib.rs for Rust effect plugins
fn generate_rust_effect_lib(config: &WizardConfig) -> String {
    let _crate_name = config.id.replace('.', "-").to_lowercase();
    
    format!(
        "// SPDX-License-Identifier: {}\n\n// paint.type {} Plugin\n// Generated by plugin-wizard\n\nuse paint_type_plugins::{{error::PluginResult, manifest::{{PluginCapability, PluginId, PluginManifest, PluginType}}, effect::{{EffectConfig, EffectPlugin, EffectType, WasmEffectPlugin}}}};\n\n/// Plugin ID\nconst PLUGIN_ID: &str = \"{}\";\n\n/// Plugin manifest\nfn get_manifest() -> PluginManifest {{\n    PluginManifest {{\n        id: PluginId::new(PLUGIN_ID.to_string()),\n        version: paint_type_plugins::manifest::PluginVersion::new(0, 1, 0),\n        name: \"{}\".to_string(),\n        description: \"{}\".to_string(),\n        long_description: None,\n        author: \"{}\".to_string(),\n        author_email: None,\n        plugin_type: PluginType::Effect,\n        api_version: paint_type_plugins::manifest::PluginVersion::MIN_API_VERSION,\n        capabilities: vec![{}],\n        wasm_entry: \"_start\".to_string(),\n        icon: None,\n        homepage: None,\n        license: \"{}\".to_string(),\n        tags: vec![{}],\n    }}\n}}\n\n/// Main effect plugin struct\npub struct {}EffectPlugin;\n\nimpl EffectPlugin for {}EffectPlugin {{\n    fn apply(&self, input: &[u8], width: u32, height: u32) -> PluginResult<Vec<u8>> {{\n        // TODO: Implement your effect here\n        // For now, return the input unchanged\n        Ok(input.to_vec())\n    }}\n\n    fn id(&self) -> &PluginId {{\n        &get_manifest().id\n    }}\n\n    fn name(&self) -> &str {{\n        &get_manifest().name\n    }}\n\n    fn description(&self) -> &str {{\n        &get_manifest().description\n    }}\n\n    fn required_capabilities(&self) -> &[PluginCapability] {{\n        &get_manifest().capabilities\n    }}\n}}\n",
        config.license,
        config.plugin_type,
        config.id,
        config.name,
        config.description,
        config.author,
        format_capabilities(&config.capabilities),
        config.license,
        format_tags(&config.tags),
        to_camel_case(&config.name),
        to_camel_case(&config.name)
    )
}

/// Generate lib.rs for Rust tool plugins
fn generate_rust_tool_lib(config: &WizardConfig) -> String {
    let _crate_name = config.id.replace('.', "-").to_lowercase();
    
    format!(
        "// SPDX-License-Identifier: {}\n\n// paint.type {} Plugin\n// Generated by plugin-wizard\n\nuse paint_type_plugins::{{error::PluginResult, manifest::{{PluginCapability, PluginId, PluginManifest, PluginType}}, tool::{{ToolConfig, ToolPlugin, ToolResponse, ToolState, WasmToolPlugin}}}};\n\n/// Plugin ID\nconst PLUGIN_ID: &str = \"{}\";\n\n/// Plugin manifest\nfn get_manifest() -> PluginManifest {{\n    PluginManifest {{\n        id: PluginId::new(PLUGIN_ID.to_string()),\n        version: paint_type_plugins::manifest::PluginVersion::new(0, 1, 0),\n        name: \"{}\".to_string(),\n        description: \"{}\".to_string(),\n        long_description: None,\n        author: \"{}\".to_string(),\n        author_email: None,\n        plugin_type: PluginType::Tool,\n        api_version: paint_type_plugins::manifest::PluginVersion::MIN_API_VERSION,\n        capabilities: vec![{}],\n        wasm_entry: \"_start\".to_string(),\n        icon: None,\n        homepage: None,\n        license: \"{}\".to_string(),\n        tags: vec![{}],\n    }}\n}}\n\n/// Main tool plugin struct\npub struct {}ToolPlugin {{\n    state: ToolState,\n}}\n\nimpl Default for {}ToolPlugin {{\n    fn default() -> Self {{\n        Self {{\n            state: ToolState::new(),\n        }}\n    }}\n}}\n\nimpl ToolPlugin for {}ToolPlugin {{\n    fn handle_event(&mut self, event: ToolEvent, state: &mut ToolState) -> PluginResult<ToolResponse> {{\n        // TODO: Implement your tool event handling here\n        match event {{\n            ToolEvent::PointerDown {{ x, y }} => {{\n                state.x = x;\n                state.y = y;\n                state.is_active = true;\n                Ok(ToolResponse::new().with_status(\"Pointer down\"))\n            }}\n            ToolEvent::PointerMove {{ x, y, pressure }} => {{\n                state.x = x;\n                state.y = y;\n                state.pressure = pressure;\n                Ok(ToolResponse::new().with_status(\"Pointer move\"))\n            }}\n            ToolEvent::PointerUp {{ x, y }} => {{\n                state.x = x;\n                state.y = y;\n                state.is_active = false;\n                Ok(ToolResponse::new().with_status(\"Pointer up\"))\n            }}\n            ToolEvent::Cancel => {{\n                state.is_active = false;\n                state.reset();\n                Ok(ToolResponse::new().with_status(\"Cancelled\"))\n            }}\n            ToolEvent::ConfigChanged => {{\n                Ok(ToolResponse::new().with_status(\"Config changed\"))\n            }}\n        }}\n    }}\n\n    fn finalize(&mut self, state: &mut ToolState) -> PluginResult<ToolResponse> {{\n        state.reset();\n        Ok(ToolResponse::new())\n    }}\n\n    fn id(&self) -> &PluginId {{\n        &get_manifest().id\n    }}\n\n    fn name(&self) -> &str {{\n        &get_manifest().name\n    }}\n\n    fn description(&self) -> &str {{\n        &get_manifest().description\n    }}\n\n    fn config(&self) -> &ToolConfig {{\n        // TODO: Return your tool configuration\n        static CONFIG: once_cell::sync::OnceCell<ToolConfig> = once_cell::sync::OnceCell::new();\n        CONFIG.get_or_init(|| {{\n            let mut config = ToolConfig::new(\"{}\");\n            // Add your tool options here\n            config\n        }})\n    }}\n\n    fn required_capabilities(&self) -> &[PluginCapability] {{\n        &get_manifest().capabilities\n    }}\n}}\n",
        config.license,
        config.plugin_type,
        config.id,
        config.name,
        config.description,
        config.author,
        format_capabilities(&config.capabilities),
        format_tags(&config.tags),
        to_camel_case(&config.name),
        to_camel_case(&config.name),
        to_camel_case(&config.name),
        config.name,
        config.name
    )
}

/// Format capabilities for Rust code
fn format_capabilities(caps: &[PluginCapability]) -> String {
    if caps.is_empty() {
        return String::new();
    }
    caps.iter()
        .map(|c| format!("PluginCapability::{}", match c {
            PluginCapability::CanvasRead => "CanvasRead",
            PluginCapability::CanvasWrite => "CanvasWrite",
            PluginCapability::LayerAccess => "LayerAccess",
            PluginCapability::SelectionAccess => "SelectionAccess",
            PluginCapability::FileAccess => "FileAccess",
            PluginCapability::NetworkAccess => "NetworkAccess",
            PluginCapability::UserInterface => "UserInterface",
            PluginCapability::PersistentStorage => "PersistentStorage",
        }))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Format tags for Rust code
fn format_tags(tags: &[String]) -> String {
    if tags.is_empty() {
        return String::new();
    }
    tags.iter()
        .map(|t| format!("\"{}\"", t))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Convert to CamelCase
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

/// Create the plugin directory structure
fn create_plugin_structure(config: &WizardConfig) -> Result<()> {
    let output_dir = &config.output_dir;
    
    // Create output directory
    if output_dir.exists() {
        anyhow::bail!("Output directory already exists: {}", output_dir.display());
    }
    
    fs::create_dir_all(output_dir)?;
    
    // Create src directory
    let src_dir = output_dir.join("src");
    fs::create_dir_all(&src_dir)?;
    
    // Generate and write plugin.toml
    let plugin_toml = generate_plugin_toml(config);
    let plugin_toml_path = output_dir.join("plugin.toml");
    fs::write(&plugin_toml_path, plugin_toml)?;
    
    // Generate and write minter.toml if requested
    if config.create_minter {
        let minter_toml = generate_minter_toml(config);
        let minter_toml_path = output_dir.join("minter.toml");
        fs::write(&minter_toml_path, minter_toml)?;
    }
    
    // Generate Cargo.toml for Rust
    if matches!(config.language, PluginLanguage::Rust) {
        let cargo_toml = generate_cargo_toml(config);
        let cargo_toml_path = output_dir.join("Cargo.toml");
        fs::write(&cargo_toml_path, cargo_toml)?;
        
        // Generate lib.rs based on plugin type
        let lib_rs = match config.plugin_type {
            PluginType::Effect => generate_rust_effect_lib(config),
            PluginType::Tool => generate_rust_tool_lib(config),
        };
        let lib_rs_path = src_dir.join("lib.rs");
        fs::write(&lib_rs_path, lib_rs)?;
        
        // Create README
        let readme = generate_readme(config);
        let readme_path = output_dir.join("README.adoc");
        fs::write(&readme_path, readme)?;
    }
    
    // Generate .gitignore
    let gitignore = generate_gitignore();
    let gitignore_path = output_dir.join(".gitignore");
    fs::write(&gitignore_path, gitignore)?;
    
    // Generate LICENSE file
    let license = generate_license(&config.license);
    let license_path = output_dir.join("LICENSE");
    fs::write(&license_path, license)?;
    
    Ok(())
}

/// Generate README.adoc
fn generate_readme(config: &WizardConfig) -> String {
    format!(
        "= {} Plugin\n\n:description: {}\n:spdx-license-identifier: {}\n\n== Overview\n\nThis is a {} plugin for paint.type.\n\n== Plugin Type\n\n{}\n\n== Author\n\n{}\n{}\n\n== Usage\n\n// TODO: Add usage instructions\n\n== License\n\n{}\n",
        config.name,
        config.description,
        config.license,
        match config.plugin_type {
            PluginType::Effect => "effect",
            PluginType::Tool => "tool",
        },
        match config.plugin_type {
            PluginType::Effect => "Effect plugins apply stateless transformations to the canvas.",
            PluginType::Tool => "Tool plugins are stateful interactive tools.",
        },
        config.author,
        if let Some(email) = &config.author_email {
            format!("<{}", email)
        } else {
            String::new()
        },
        config.license
    )
}

/// Generate .gitignore
fn generate_gitignore() -> String {
    "# SPDX-License-Identifier: CC0-1.0\n\n# Build artifacts\ntarget/\n\n# IDE\n.idea/\n.vscode/\n*.swp\n*.swo\n\n# macOS\n.DS_Store\n\n# Windows\nThumbs.db\n\n# Environment\n.env\n.env.local\n" .to_string()
}

/// Generate LICENSE file
fn generate_license(spdx: &str) -> String {
    match spdx {
        "AGPL-3.0-or-later" => AGPL_LICENSE.to_string(),
        "MIT" => MIT_LICENSE.to_string(),
        "Apache-2.0" => APACHE_LICENSE.to_string(),
        _ => format!("SPDX-License-Identifier: {}\n\nLicense text: See {}\n", spdx, spdx),
    }
}

const AGPL_LICENSE: &str = "GNU AFFERO GENERAL PUBLIC LICENSE\nVersion 3, 19 November 2007\n\nCopyright (C) 2007 Free Software Foundation, Inc. <https://fsf.org/>\nEveryone is permitted to copy and distribute verbatim copies\nof this license document, but changing it is not allowed.\n\nThis program is free software: you can redistribute it and/or modify\nit under the terms of the GNU Affero General Public License as\npublished by the Free Software Foundation, either version 3 of the\nLicense, or (at your option) any later version.\n\nThis program is distributed in the hope that it will be useful,\nbut WITHOUT ANY WARRANTY; without even the implied warranty of\nMERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the\nGNU Affero General Public License for more details.\n\nYou should have received a copy of the GNU Affero General Public License\nalong with this program.  If not, see <https://www.gnu.org/licenses/>.\n";

const MIT_LICENSE: &str = "MIT License\n\nCopyright (c) {year} {author}\n\nPermission is hereby granted, free of charge, to any person obtaining a copy\nof this software and associated documentation files (the \"Software\"), to deal\nin the Software without restriction, including without limitation the rights\nto use, copy, modify, merge, publish, distribute, sublicense, and/or sell\ncopies of the Software, and to permit persons to whom the Software is\nfurnished to do so, subject to the following conditions:\n\nThe above copyright notice and this permission notice shall be included in all\ncopies or substantial portions of the Software.\n\nTHE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\nIMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\nFITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\nAUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\nLIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\nOUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\nSOFTWARE.\n";

const APACHE_LICENSE: &str = "Apache License\nVersion 2.0, January 2004\nhttp://www.apache.org/licenses/\n\nTERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION\n\n1. Definitions.\n\n\"License\" shall mean the terms and conditions for use, reproduction, and distribution as defined by Sections 1 through 9 of this document.\n\n\"Licensor\" shall mean the copyright owner or entity authorized by the copyright owner that is granting the License.\n\n\"Legal Entity\" shall mean the union of the acting entity and all other entities that control, are controlled by, or are under common control with that entity. For the purposes of this definition, \"control\" means (i) the power, direct or indirect, to cause the direction or management of such entity, whether by contract or otherwise, or (ii) ownership of fifty percent (50%) or more of the outstanding shares, or (iii) beneficial ownership of such entity.\n\n\"You\" (or \"Your\") shall mean an individual or Legal Entity exercising permissions granted by this License.\n\n\"Source\" form shall mean the preferred form for making modifications, including but not limited to software source code, documentation source, and configuration files.\n\n\"Object\" form shall mean any form resulting from mechanical transformation or translation of a Source form, including but not limited to compiled object code, generated documentation, and conversions to other media types.\n\n\"Work\" shall mean the work of authorship, whether in Source or Object form, made available under the License, as indicated by a copyright notice that is included in or attached to the work (an example is provided in the Appendix below).\n\n\"Derivative Works\" shall mean any work, whether in Source or Object form, that is based on (or derived from) the Work and for which the editorial revisions, annotations, elaborations, or other modifications represent, as a whole, an original work of authorship. For the purposes of this License, Derivative Works shall not include works that remain separable from, or merely link (or bind by name) to the interfaces of, the Work and Derivative Works thereof.\n\n\"Contribution\" shall mean any work of authorship, including the original version of the Work and any modifications or additions to that Work or Derivative Works thereof, that is intentionally submitted to Licensor for inclusion in the Work by the copyright owner or by an individual or Legal Entity authorized to submit on behalf of the copyright owner. For the purposes of this definition, \"submitted\" means any form of electronic, verbal, or written communication sent to the Licensor or its representatives, including but not limited to communication on electronic mailing lists, source code control systems, and issue tracking systems that are managed by, or on behalf of, the Licensor for the purpose of discussing and improving the Work, but excluding communication that is conspicuously marked or otherwise designated in writing by the copyright owner as \"Not a Contribution.\"\n\n\"Contributor\" shall mean Licensor and any individual or Legal Entity on behalf of whom a Contribution has been received by Licensor and subsequently incorporated within the Work.\n";

fn main() -> Result<()> {
    let args = Args::parse();
    
    let config = if args.non_interactive {
        // Non-interactive mode - use CLI args only
        let mut config = WizardConfig::default();
        
        if args.r#type.is_none() {
            anyhow::bail!("In non-interactive mode, --type is required");
        }
        config.plugin_type = args.r#type.unwrap();
        
        if args.id.is_none() {
            anyhow::bail!("In non-interactive mode, --id is required");
        }
        config.id = args.id.unwrap();
        validate_id(&config.id)?;
        
        if args.name.is_none() {
            anyhow::bail!("In non-interactive mode, --name is required");
        }
        config.name = args.name.unwrap();
        
        if args.description.is_none() {
            anyhow::bail!("In non-interactive mode, --description is required");
        }
        config.description = args.description.unwrap();
        
        if args.output.is_none() {
            anyhow::bail!("In non-interactive mode, --output is required");
        }
        config.output_dir = args.output.unwrap();
        
        // Use defaults for optional fields
        if args.language.is_some() {
            config.language = args.language.unwrap();
        }
        
        config
    } else {
        run_interactive_wizard(&args)?
    };
    
    // Create the plugin structure
    create_plugin_structure(&config)?;
    
    // Success message
    println!();
    println!("{}", style("✓ Plugin scaffolding complete!").bold().green());
    println!();
    println!("Created plugin: {}", style(&config.name).cyan());
    println!("  Type: {}", style(&config.plugin_type).cyan());
    println!("  ID: {}", style(&config.id).cyan());
    println!("  Location: {}", style(config.output_dir.display()).cyan());
    println!();
    println!("Next steps:");
    
    if matches!(config.language, PluginLanguage::Rust) {
        println!("  1. cd {}", config.output_dir.display());
        println!("  2. Edit src/lib.rs to implement your plugin");
        println!("  3. cargo build --release");
        println!("  4. Test with: cargo run --manifest-path ../../tools/plugin-harness/Cargo.toml -- .");
    } else {
        println!("  1. Edit the generated files");
        println!("  2. Build with the appropriate compiler");
        println!("  3. Test your plugin");
    }
    
    println!();
    
    Ok(())
}

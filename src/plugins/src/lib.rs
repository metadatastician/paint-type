// SPDX-License-Identifier: AGPL-3.0-or-later
//
// paint-type Plugin System (v0.4.0)
//
// This crate provides the plugin infrastructure for paint.type, including:
// - Plugin manifest schema and validation
// - WASM plugin sandboxing
// - Effect plugin API (colour adjustments, filters)
// - Tool plugin API (custom brush behaviours)
//
// Architecture:
//   - Plugins are WASM modules that run in a sandboxed environment
//   - Communication via typed-wasm for memory safety
//   - Manifests are signed with ML-DSA (cerro-torre integration)
//   - Two plugin tiers: Effect (stateless) and Tool (stateful)

#![forbid(unsafe_code)]

pub mod error;
pub mod manifest;
pub mod sandbox;
pub mod effect;
pub mod tool;
pub mod registry;

// Re-export main types
pub use error::PluginError;
pub use manifest::{PluginManifest, PluginId, PluginVersion, PluginType};
pub use sandbox::WasmSandbox;
pub use effect::EffectPlugin;
pub use tool::ToolPlugin;
pub use registry::PluginRegistry;

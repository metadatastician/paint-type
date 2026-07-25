// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tool Plugin API.
//!
//! Tool plugins are stateful interactive tools that users can use to paint or
//! manipulate the canvas. They include:
//! - Custom brushes
//! - Selection tools
//! - Transform tools
//! - Fill tools
//!
//! Tool plugins:
//! - Are stateful (maintain state between mouse events)
//! - Receive mouse/pointer events (down, move, up)
//! - Can request CanvasWrite and LayerAccess capabilities
//! - Can maintain a preview state during dragging

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::error::{PluginError, PluginResult};
use crate::manifest::{PluginCapability, PluginId, PluginManifest, PluginType};
use crate::sandbox::WasmSandbox;

/// Tool event types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToolEvent {
    /// Pointer down (mouse button pressed or touch started)
    PointerDown { x: f32, y: f32 },
    /// Pointer move (mouse moved or touch moved)
    PointerMove { x: f32, y: f32, pressure: f32 },
    /// Pointer up (mouse button released or touch ended)
    PointerUp { x: f32, y: f32 },
    /// Cancel (e.g., escape key pressed)
    Cancel,
    /// Configuration changed
    ConfigChanged,
}

/// Tool state (maintained between events)
#[derive(Debug, Clone, Default)]
pub struct ToolState {
    /// Current X position
    pub x: f32,
    /// Current Y position
    pub y: f32,
    /// Pressure (0.0 to 1.0)
    pub pressure: f32,
    /// Whether the tool is currently active (pointer down)
    pub is_active: bool,
    /// Whether to show a preview
    pub show_preview: bool,
    /// Custom state values
    pub custom: HashMap<String, String>,
}

impl ToolState {
    /// Create a new tool state
    pub fn new() -> Self {
        ToolState::default()
    }

    /// Reset the state
    pub fn reset(&mut self) {
        self.x = 0.0;
        self.y = 0.0;
        self.pressure = 0.5;
        self.is_active = false;
        self.show_preview = false;
        self.custom.clear();
    }

    /// Set a custom state value
    pub fn set_custom(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.custom.insert(key.into(), value.into());
    }

    /// Get a custom state value
    pub fn get_custom(&self, key: &str) -> Option<&str> {
        self.custom.get(key).map(|s| s.as_str())
    }
}

/// Tool configuration
#[derive(Debug, Clone)]
pub struct ToolConfig {
    /// Tool name
    pub name: String,
    /// Tool cursor (CSS cursor value)
    pub cursor: String,
    /// Tool icon
    pub icon: Option<String>,
    /// Tool options
    pub options: HashMap<String, ToolOption>,
}

impl ToolConfig {
    /// Create a new tool configuration
    pub fn new(name: impl Into<String>) -> Self {
        ToolConfig {
            name: name.into(),
            cursor: "crosshair".to_string(),
            icon: None,
            options: HashMap::new(),
        }
    }

    /// Set the cursor
    pub fn set_cursor(&mut self, cursor: impl Into<String>) {
        self.cursor = cursor.into();
    }

    /// Add an option
    pub fn add_option(&mut self, name: impl Into<String>, option: ToolOption) {
        self.options.insert(name.into(), option);
    }

    /// Get an option
    pub fn get_option(&self, name: &str) -> Option<&ToolOption> {
        self.options.get(name)
    }
}

/// Tool option definition
#[derive(Debug, Clone)]
pub struct ToolOption {
    /// Display name
    pub label: String,
    /// Option type
    pub option_type: ToolOptionType,
    /// Default value
    pub default_value: String,
    /// Minimum value (for numeric options)
    pub min: Option<f32>,
    /// Maximum value (for numeric options)
    pub max: Option<f32>,
    /// Step value (for numeric options)
    pub step: Option<f32>,
    /// List of allowed values (for enum options)
    pub allowed_values: Option<Vec<String>>,
}

/// Tool option types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolOptionType {
    Boolean,
    Integer,
    Float,
    String,
    Enum,
    Color,
}

/// Tool plugin trait
pub trait ToolPlugin {
    /// Get the plugin ID
    fn id(&self) -> &PluginId;

    /// Handle a tool event
    fn handle_event(&mut self, event: ToolEvent, state: &mut ToolState) -> PluginResult<ToolResponse>;

    /// Get the tool name
    fn name(&self) -> &str;

    /// Get the tool description
    fn description(&self) -> &str;

    /// Get the tool configuration
    fn config(&self) -> &ToolConfig;

    /// Get required capabilities
    fn required_capabilities(&self) -> &[PluginCapability];

    /// Finalize the tool operation (called after pointer up)
    fn finalize(&mut self, state: &mut ToolState) -> PluginResult<ToolResponse>;
}

/// Tool response
#[derive(Debug, Clone)]
pub struct ToolResponse {
    /// Whether the canvas was modified
    pub canvas_modified: bool,
    /// Region that was modified (for repainting)
    pub dirty_rect: Option<(u32, u32, u32, u32)>, // (x, y, width, height)
    /// Preview data (optional)
    pub preview: Option<Vec<u8>>,
    /// Status message (optional)
    pub status: Option<String>,
}

impl Default for ToolResponse {
    fn default() -> Self {
        ToolResponse {
            canvas_modified: false,
            dirty_rect: None,
            preview: None,
            status: None,
        }
    }
}

impl ToolResponse {
    /// Create a new response
    pub fn new() -> Self {
        ToolResponse::default()
    }

    /// Mark canvas as modified
    pub fn with_modified(mut self, dirty_rect: Option<(u32, u32, u32, u32)>) -> Self {
        self.canvas_modified = true;
        self.dirty_rect = dirty_rect;
        self
    }

    /// Add preview data
    pub fn with_preview(mut self, preview: Vec<u8>) -> Self {
        self.preview = Some(preview);
        self
    }

    /// Add status message
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }
}

/// A WASM-based tool plugin
pub struct WasmToolPlugin {
    sandbox: WasmSandbox,
    config: ToolConfig,
    state: ToolState,
}

impl WasmToolPlugin {
    /// Create a new WASM tool plugin
    pub fn new(manifest: PluginManifest, config: ToolConfig) -> PluginResult<Self> {
        if manifest.plugin_type != PluginType::Tool {
            return Err(PluginError::manifest_validation(
                "Expected tool plugin type".to_string(),
            ));
        }
        
        Ok(WasmToolPlugin {
            sandbox: WasmSandbox::new(manifest),
            config,
            state: ToolState::new(),
        })
    }

    /// Load the plugin from a WASM file
    pub fn load(&mut self, wasm_path: impl AsRef<std::path::Path>) -> PluginResult<()> {
        self.sandbox.load(wasm_path)
    }

    /// Get the plugin ID
    pub fn id(&self) -> &crate::manifest::PluginId {
        self.sandbox.id()
    }

    /// Get the tool configuration
    pub fn config(&self) -> &ToolConfig {
        &self.config
    }

    /// Get mutable access to the configuration
    pub fn config_mut(&mut self) -> &mut ToolConfig {
        &mut self.config
    }

    /// Get the current tool state
    pub fn state(&self) -> &ToolState {
        &self.state
    }

    /// Get mutable access to the tool state
    pub fn state_mut(&mut self) -> &mut ToolState {
        &mut self.state
    }
}

impl ToolPlugin for WasmToolPlugin {
    fn handle_event(&mut self, event: ToolEvent, state: &mut ToolState) -> PluginResult<ToolResponse> {
        if !self.sandbox.is_loaded() {
            return Err(PluginError::PluginError("Plugin not loaded".to_string()));
        }
        
        // In real implementation, this would call into WASM
        // For now, simulate a simple brush tool
        match event {
            ToolEvent::PointerDown { x, y } => {
                state.x = x;
                state.y = y;
                state.is_active = true;
                // Simulate painting a dot
                Ok(ToolResponse::new().with_modified(Some((x as u32, y as u32, 1, 1))))
            }
            ToolEvent::PointerMove { x, y, pressure } => {
                state.x = x;
                state.y = y;
                state.pressure = pressure;
                // Simulate painting a line from previous to current position
                Ok(ToolResponse::new().with_modified(Some((
                    state.x as u32,
                    state.y as u32,
                    1,
                    1,
                ))))
            }
            ToolEvent::PointerUp { x, y } => {
                state.x = x;
                state.y = y;
                state.is_active = false;
                Ok(ToolResponse::new().with_modified(Some((x as u32, y as u32, 1, 1))))
            }
            ToolEvent::Cancel => {
                state.is_active = false;
                state.reset();
                Ok(ToolResponse::new())
            }
            ToolEvent::ConfigChanged => {
                Ok(ToolResponse::new())
            }
        }
    }

    fn name(&self) -> &str {
        &self.sandbox.manifest().name
    }

    fn description(&self) -> &str {
        &self.sandbox.manifest().description
    }

    fn config(&self) -> &ToolConfig {
        &self.config
    }

    fn required_capabilities(&self) -> &[PluginCapability] {
        &self.sandbox.manifest().capabilities
    }

    fn finalize(&mut self, state: &mut ToolState) -> PluginResult<ToolResponse> {
        // Default finalize does nothing
        // Plugins can override this to commit changes
        state.reset();
        Ok(ToolResponse::new())
    }
}

/// Built-in brush tool (non-WASM, for testing)
pub struct BrushTool {
    id: PluginId,
    size: f32,
    hardness: f32,
    color: [u8; 4],
}

impl BrushTool {
    pub fn new(size: f32, hardness: f32, color: [u8; 4]) -> Self {
        BrushTool {
            id: PluginId::new("com.paint-type.tools.brush"),
            size,
            hardness,
            color,
        }
    }
}

impl ToolPlugin for BrushTool {
    fn handle_event(&mut self, event: ToolEvent, state: &mut ToolState) -> PluginResult<ToolResponse> {
        match event {
            ToolEvent::PointerDown { x, y } => {
                state.x = x;
                state.y = y;
                state.is_active = true;
                // Paint a dot
                Ok(ToolResponse::new().with_modified(Some((x as u32, y as u32, 1, 1))))
            }
            ToolEvent::PointerMove { x, y, pressure } => {
                if !state.is_active {
                    return Ok(ToolResponse::new());
                }
                
                state.x = x;
                state.y = y;
                state.pressure = pressure;
                
                // Simulate painting with the brush
                // In real implementation, this would use the brush engine
                Ok(ToolResponse::new().with_modified(Some((
                    x as u32,
                    y as u32,
                    1,
                    1,
                ))))
            }
            ToolEvent::PointerUp { x, y } => {
                state.x = x;
                state.y = y;
                state.is_active = false;
                Ok(ToolResponse::new().with_modified(Some((x as u32, y as u32, 1, 1))))
            }
            ToolEvent::Cancel => {
                state.is_active = false;
                state.reset();
                Ok(ToolResponse::new())
            }
            ToolEvent::ConfigChanged => {
                Ok(ToolResponse::new())
            }
        }
    }

    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Brush"
    }

    fn description(&self) -> &str {
        "Paint with a customizable brush"
    }

    fn config(&self) -> &ToolConfig {
        // Build config dynamically using OnceLock for thread-safe lazy initialization
        static CONFIG: OnceLock<ToolConfig> = OnceLock::new();
        
        CONFIG.get_or_init(|| {
            let mut config = ToolConfig::new("Brush");
            config.set_cursor("crosshair".to_string());
            
            let size_option = ToolOption {
                label: "Size".to_string(),
                option_type: ToolOptionType::Float,
                default_value: "10.0".to_string(),
                min: Some(1.0),
                max: Some(100.0),
                step: Some(0.5),
                allowed_values: None,
            };
            config.add_option("size", size_option);
            
            let hardness_option = ToolOption {
                label: "Hardness".to_string(),
                option_type: ToolOptionType::Float,
                default_value: "1.0".to_string(),
                min: Some(0.0),
                max: Some(1.0),
                step: Some(0.05),
                allowed_values: None,
            };
            config.add_option("hardness", hardness_option);
            
            config
        })
    }

    fn required_capabilities(&self) -> &[PluginCapability] {
        &[PluginCapability::CanvasWrite]
    }

    fn finalize(&mut self, state: &mut ToolState) -> PluginResult<ToolResponse> {
        state.reset();
        Ok(ToolResponse::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_state() {
        let mut state = ToolState::new();
        assert!(!state.is_active);
        
        state.x = 100.0;
        state.y = 200.0;
        state.is_active = true;
        state.set_custom("test", "value");
        
        assert_eq!(state.x, 100.0);
        assert_eq!(state.y, 200.0);
        assert!(state.is_active);
        assert_eq!(state.get_custom("test"), Some("value"));
    }

    #[test]
    fn test_tool_response() {
        let mut response = ToolResponse::new();
        assert!(!response.canvas_modified);
        
        response = response.with_modified(Some((0, 0, 10, 10)));
        assert!(response.canvas_modified);
        assert_eq!(response.dirty_rect, Some((0, 0, 10, 10)));
    }
}

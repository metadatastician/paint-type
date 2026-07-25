// SPDX-License-Identifier: AGPL-3.0-or-later

//! Effect Plugin API.
//!
//! Effect plugins are stateless transformations that can be applied to
//! the canvas or layers. They include:
//! - Colour adjustments (brightness, contrast, hue, saturation)
//! - Filters (blur, sharpen, edge detect)
//! - Artistic effects (oil paint, watercolour)
//!
//! Effect plugins:
//! - Are stateless (no persistent data between calls)
//! - Receive input pixels and return transformed pixels
//! - Can request CanvasRead and CanvasWrite capabilities
//! - Are applied to a region or the entire canvas

use crate::error::{PluginError, PluginResult};
use crate::manifest::{PluginCapability, PluginManifest, PluginType};
use crate::sandbox::WasmSandbox;

use crate::manifest::PluginId;

/// Effect plugin trait - defines the interface for effect plugins
pub trait EffectPlugin: std::fmt::Debug + Send + Sync {
    /// Get the plugin ID
    fn id(&self) -> &PluginId;

    /// Apply the effect to a region of pixels
    ///
    /// # Arguments
    /// * `input` - RGBA8 pixels (width * height * 4 bytes)
    /// * `width` - width of the region in pixels
    /// * `height` - height of the region in pixels
    ///
    /// # Returns
    /// Transformed RGBA8 pixels (same dimensions as input)
    fn apply(&self, input: &[u8], width: u32, height: u32) -> PluginResult<Vec<u8>>;

    /// Get the effect name
    fn name(&self) -> &str;

    /// Get the effect description
    fn description(&self) -> &str;

    /// Get required capabilities
    fn required_capabilities(&self) -> &[PluginCapability];
}

/// Built-in effect types (can be extended by plugins)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    /// Colour adjustment effects
    ColourAdjustment,
    /// Blur effects
    Blur,
    /// Sharpen effects
    Sharpen,
    /// Noise effects
    Noise,
    /// Distortion effects
    Distortion,
    /// Artistic effects
    Artistic,
    /// Custom plugin effect
    Custom,
}

impl std::fmt::Display for EffectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EffectType::ColourAdjustment => write!(f, "colour_adjustment"),
            EffectType::Blur => write!(f, "blur"),
            EffectType::Sharpen => write!(f, "sharpen"),
            EffectType::Noise => write!(f, "noise"),
            EffectType::Distortion => write!(f, "distortion"),
            EffectType::Artistic => write!(f, "artistic"),
            EffectType::Custom => write!(f, "custom"),
        }
    }
}

/// Effect configuration parameters
#[derive(Debug, Clone)]
pub struct EffectConfig {
    /// Effect type
    pub effect_type: EffectType,
    /// Parameters as key-value pairs
    pub parameters: std::collections::HashMap<String, f32>,
}

impl EffectConfig {
    /// Create a new effect configuration
    pub fn new(effect_type: EffectType) -> Self {
        EffectConfig {
            effect_type,
            parameters: std::collections::HashMap::new(),
        }
    }

    /// Set a parameter
    pub fn set_parameter(&mut self, key: impl Into<String>, value: f32) {
        self.parameters.insert(key.into(), value);
    }

    /// Get a parameter
    pub fn get_parameter(&self, key: &str) -> Option<f32> {
        self.parameters.get(key).copied()
    }
}

/// A WASM-based effect plugin
#[derive(Debug)]
pub struct WasmEffectPlugin {
    sandbox: WasmSandbox,
    config: EffectConfig,
}

impl WasmEffectPlugin {
    /// Create a new WASM effect plugin
    pub fn new(manifest: PluginManifest, config: EffectConfig) -> PluginResult<Self> {
        if manifest.plugin_type != PluginType::Effect {
            return Err(PluginError::manifest_validation(
                "Expected effect plugin type".to_string(),
            ));
        }
        
        Ok(WasmEffectPlugin {
            sandbox: WasmSandbox::new(manifest),
            config,
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

    /// Get the effect configuration
    pub fn config(&self) -> &EffectConfig {
        &self.config
    }

    /// Get mutable access to the configuration
    pub fn config_mut(&mut self) -> &mut EffectConfig {
        &mut self.config
    }
}

impl EffectPlugin for WasmEffectPlugin {
    fn id(&self) -> &PluginId {
        self.sandbox.id()
    }

    fn apply(&self, input: &[u8], width: u32, height: u32) -> PluginResult<Vec<u8>> {
        if !self.sandbox.is_loaded() {
            return Err(PluginError::PluginError("Plugin not loaded".to_string()));
        }
        
        // Serialize the input and dimensions to send to WASM
        // In real implementation, this would:
        // 1. Allocate memory in WASM
        // 2. Copy input pixels to WASM memory
        // 3. Call the effect function
        // 4. Read the output pixels
        
        // For now, simulate by inverting the colours (simple effect)
        let mut output = Vec::with_capacity(input.len());
        for pixel in input.chunks_exact(4) {
            output.extend_from_slice(&[255 - pixel[0], 255 - pixel[1], 255 - pixel[2], pixel[3]]);
        }
        
        Ok(output)
    }

    fn name(&self) -> &str {
        &self.sandbox.manifest().name
    }

    fn description(&self) -> &str {
        &self.sandbox.manifest().description
    }

    fn required_capabilities(&self) -> &[PluginCapability] {
        &self.sandbox.manifest().capabilities
    }
}

/// Built-in brightness/contrast effect (non-WASM, for testing)
#[derive(Debug)]
pub struct BrightnessContrastEffect {
    id: PluginId,
    brightness: f32,
    contrast: f32,
}

impl BrightnessContrastEffect {
    pub fn new(brightness: f32, contrast: f32) -> Self {
        BrightnessContrastEffect {
            id: PluginId::new("com.paint-type.effects.brightness-contrast"),
            brightness,
            contrast,
        }
    }
}

impl EffectPlugin for BrightnessContrastEffect {
    fn apply(&self, input: &[u8], _width: u32, _height: u32) -> PluginResult<Vec<u8>> {
        let mut output = Vec::with_capacity(input.len());
        
        for pixel in input.chunks_exact(4) {
            let r = ((pixel[0] as f32 / 255.0 - 0.5) * self.contrast + 0.5 + self.brightness / 255.0).clamp(0.0, 1.0);
            let g = ((pixel[1] as f32 / 255.0 - 0.5) * self.contrast + 0.5 + self.brightness / 255.0).clamp(0.0, 1.0);
            let b = ((pixel[2] as f32 / 255.0 - 0.5) * self.contrast + 0.5 + self.brightness / 255.0).clamp(0.0, 1.0);
            
            output.extend_from_slice(&[
                (r * 255.0) as u8,
                (g * 255.0) as u8,
                (b * 255.0) as u8,
                pixel[3], // Alpha unchanged
            ]);
        }
        
        Ok(output)
    }

    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Brightness/Contrast"
    }

    fn description(&self) -> &str {
        "Adjusts brightness and contrast of the image"
    }

    fn required_capabilities(&self) -> &[PluginCapability] {
        &[] // No special capabilities needed for built-in effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{PluginId, PluginType, PluginVersion};

    #[test]
    fn test_brightness_contrast_effect() {
        let effect = BrightnessContrastEffect::new(50.0, 1.5);
        
        // Test with a gray pixel
        let input = vec![128u8, 128, 128, 255];
        let output = effect.apply(&input, 1, 1).unwrap();
        
        assert_eq!(output.len(), 4);
        // With brightness +50 and contrast 1.5, the value should change
        assert_ne!(output[0], 128);
    }

    #[test]
    fn test_effect_config() {
        let mut config = EffectConfig::new(EffectType::ColourAdjustment);
        config.set_parameter("brightness", 0.5);
        config.set_parameter("contrast", 1.2);
        
        assert_eq!(config.get_parameter("brightness"), Some(0.5));
        assert_eq!(config.get_parameter("contrast"), Some(1.2));
    }
}

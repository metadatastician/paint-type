// SPDX-License-Identifier: AGPL-3.0-or-later

//! Effect Plugin API.

use crate::error::{PluginError, PluginResult};
use crate::manifest::{PluginCapability, PluginId, PluginManifest, PluginType};
use crate::sandbox::WasmSandbox;

/// Require exactly `width * height * 4` RGBA8 bytes, allowing empty regions.
///
/// Returns `PluginError::PluginError` for size overflow or a length mismatch.
fn check_rgba8_dimensions(input: &[u8], width: u32, height: u32) -> PluginResult<()> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(4))
        .ok_or_else(|| PluginError::PluginError("region dimensions overflow".to_string()))?;
    if input.len() != expected {
        return Err(PluginError::PluginError(format!(
            "RGBA8 buffer length {} does not match {}x{}x4 = {}",
            input.len(),
            width,
            height,
            expected
        )));
    }
    Ok(())
}

/// Effect plugin trait - defines the interface for effect plugins
pub trait EffectPlugin: std::fmt::Debug + Send + Sync {
    /// Get the plugin ID
    fn id(&self) -> &PluginId;

    /// Apply the effect to a region of pixels (`width * height * 4` RGBA8 bytes)
    fn apply(&self, input: &[u8], width: u32, height: u32) -> PluginResult<Vec<u8>>;

    /// Get the effect name
    fn name(&self) -> &str;

    /// Get the effect description
    fn description(&self) -> &str;

    /// Get required capabilities
    fn required_capabilities(&self) -> &[PluginCapability];
}

/// Built-in effect categories
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectType {
    ColourAdjustment,
    Blur,
    Sharpen,
    Noise,
    Distortion,
    Artistic,
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
    pub effect_type: EffectType,
    pub parameters: std::collections::HashMap<String, f32>,
}

impl EffectConfig {
    pub fn new(effect_type: EffectType) -> Self {
        EffectConfig {
            effect_type,
            parameters: std::collections::HashMap::new(),
        }
    }

    pub fn set_parameter(&mut self, key: impl Into<String>, value: f32) {
        self.parameters.insert(key.into(), value);
    }

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

    pub fn load(&mut self, wasm_path: impl AsRef<std::path::Path>) -> PluginResult<()> {
        self.sandbox.load(wasm_path)
    }

    pub fn id(&self) -> &PluginId {
        self.sandbox.id()
    }

    pub fn config(&self) -> &EffectConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut EffectConfig {
        &mut self.config
    }
}

impl EffectPlugin for WasmEffectPlugin {
    fn id(&self) -> &PluginId {
        self.sandbox.id()
    }

    /// Invert RGB channels while preserving alpha, including RGB of transparent pixels.
    ///
    /// Returns `PluginError::PluginError` if not loaded, if dimensions overflow,
    /// or if the input length differs from `width * height * 4`. This placeholder
    /// returns transformed bytes without executing WASM.
    fn apply(&self, input: &[u8], width: u32, height: u32) -> PluginResult<Vec<u8>> {
        if !self.sandbox.is_loaded() {
            return Err(PluginError::PluginError("Plugin not loaded".to_string()));
        }
        check_rgba8_dimensions(input, width, height)?;

        let mut output = Vec::with_capacity(input.len());
        for pixel in input.as_chunks::<4>().0 {
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

/// Built-in brightness/contrast effect
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
    /// Adjust RGB contrast around midgray and add brightness in 8-bit channel units.
    ///
    /// Clamps the result to RGBA8 and preserves alpha. Returns
    /// `PluginError::PluginError` if dimensions overflow or the input length
    /// differs from `width * height * 4`; empty regions are accepted.
    fn apply(&self, input: &[u8], width: u32, height: u32) -> PluginResult<Vec<u8>> {
        check_rgba8_dimensions(input, width, height)?;
        let mut output = Vec::with_capacity(input.len());

        for pixel in input.as_chunks::<4>().0 {
            let r = ((pixel[0] as f32 / 255.0 - 0.5) * self.contrast
                + 0.5
                + self.brightness / 255.0)
                .clamp(0.0, 1.0);
            let g = ((pixel[1] as f32 / 255.0 - 0.5) * self.contrast
                + 0.5
                + self.brightness / 255.0)
                .clamp(0.0, 1.0);
            let b = ((pixel[2] as f32 / 255.0 - 0.5) * self.contrast
                + 0.5
                + self.brightness / 255.0)
                .clamp(0.0, 1.0);

            output.extend_from_slice(&[
                (r * 255.0).round() as u8,
                (g * 255.0).round() as u8,
                (b * 255.0).round() as u8,
                pixel[3],
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
        "Adjusts brightness and contrast of the active layer"
    }

    fn required_capabilities(&self) -> &[PluginCapability] {
        &[]
    }
}

/// Built-in RGB colour inversion effect
#[derive(Debug)]
pub struct InvertColourEffect {
    id: PluginId,
}

impl InvertColourEffect {
    /// Create the built-in RGB inversion effect with its stable plugin identifier.
    pub fn new() -> Self {
        Self {
            id: PluginId::new("com.paint-type.effects.invert"),
        }
    }
}

impl Default for InvertColourEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectPlugin for InvertColourEffect {
    /// Invert RGB, preserving alpha and clearing RGB of fully transparent pixels.
    ///
    /// Returns `PluginError::PluginError` if dimensions overflow or the input
    /// length differs from `width * height * 4`; empty regions are accepted.
    fn apply(&self, input: &[u8], width: u32, height: u32) -> PluginResult<Vec<u8>> {
        check_rgba8_dimensions(input, width, height)?;
        let mut output = Vec::with_capacity(input.len());
        for px in input.as_chunks::<4>().0 {
            if px[3] == 0 {
                output.extend_from_slice(&[0, 0, 0, 0]);
            } else {
                output.extend_from_slice(&[255 - px[0], 255 - px[1], 255 - px[2], px[3]]);
            }
        }
        Ok(output)
    }

    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Invert Colours"
    }

    fn description(&self) -> &str {
        "Inverts RGB channels while preserving pixel alpha"
    }

    fn required_capabilities(&self) -> &[PluginCapability] {
        &[]
    }
}

/// Built-in Rec.709 luminance grayscale effect
#[derive(Debug)]
pub struct GrayscaleEffect {
    id: PluginId,
}

impl GrayscaleEffect {
    /// Create the built-in Rec.709 grayscale effect with its stable plugin identifier.
    pub fn new() -> Self {
        Self {
            id: PluginId::new("com.paint-type.effects.grayscale"),
        }
    }
}

impl Default for GrayscaleEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl EffectPlugin for GrayscaleEffect {
    /// Replace RGB with rounded Rec.709 luminance, preserving alpha.
    ///
    /// Returns `PluginError::PluginError` if dimensions overflow or the input
    /// length differs from `width * height * 4`; empty regions are accepted.
    fn apply(&self, input: &[u8], width: u32, height: u32) -> PluginResult<Vec<u8>> {
        check_rgba8_dimensions(input, width, height)?;
        let mut output = Vec::with_capacity(input.len());
        for px in input.as_chunks::<4>().0 {
            let luma = (0.2126 * px[0] as f32 + 0.7152 * px[1] as f32 + 0.0722 * px[2] as f32)
                .round()
                .clamp(0.0, 255.0) as u8;
            output.extend_from_slice(&[luma, luma, luma, px[3]]);
        }
        Ok(output)
    }

    fn id(&self) -> &PluginId {
        &self.id
    }

    fn name(&self) -> &str {
        "Grayscale"
    }

    fn description(&self) -> &str {
        "Converts RGB channels to Rec.709 luminance grayscale"
    }

    fn required_capabilities(&self) -> &[PluginCapability] {
        &[]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brightness_contrast_effect() {
        let effect = BrightnessContrastEffect::new(50.0, 1.5);
        let input = vec![128u8, 128, 128, 255];
        let output = effect.apply(&input, 1, 1).unwrap();
        assert_eq!(output.len(), 4);
        assert_ne!(output[0], 128);
    }

    #[test]
    fn test_invert_and_grayscale_effects() {
        let inv = InvertColourEffect::new();
        assert_eq!(inv.apply(&[10, 20, 30, 255], 1, 1).unwrap(), vec![245, 235, 225, 255]);

        let gray = GrayscaleEffect::new();
        let out = gray.apply(&[255, 0, 0, 200], 1, 1).unwrap();
        assert_eq!(out[0], out[1]);
        assert_eq!(out[1], out[2]);
        assert_eq!(out[3], 200);
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

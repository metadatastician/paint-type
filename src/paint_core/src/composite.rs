// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Paint Core — compositing operators for paint.type tiles.
//
// This module implements the Porter-Duff "over" operator (premultiplied
// and unpremultiplied), a mask-modulated variant for brush tips, separable
// blend modes (`multiply`, `screen`), and a layer-stack flattener that uses
// these primitives. Tile-level helpers `Tile::composite_over` and
// `Tile::composite_over_with_mode` perform bulk tile compositing via
// `Tile::read_buffer` / `Tile::write_buffer`.
//
// All operators traffic in `[u16; 4]` quadruples carrying the bit
// patterns of IEEE 754 binary16 (R, G, B, A) channels, matching the
// representation used by libpt and Idris2 across the rest of the crate.

#![allow(clippy::needless_range_loop)]

use crate::{
    f16_bits_to_f32, f32_to_f16_bits, layer::BlendMode, Tile, TileError, TILE_SCALARS, TILE_SIZE,
};

/// Number of pixels in a single 64×64 tile (64 * 64 = 4096).
pub const TILE_PIXEL_COUNT: usize = (TILE_SIZE as usize) * (TILE_SIZE as usize);

//==============================================================================
// Internal helpers
//==============================================================================

/// Convert a single quadruple of f16 bit patterns to f32 channels.
#[inline]
fn unpack(p: [u16; 4]) -> [f32; 4] {
    [
        f16_bits_to_f32(p[0]),
        f16_bits_to_f32(p[1]),
        f16_bits_to_f32(p[2]),
        f16_bits_to_f32(p[3]),
    ]
}

/// Convert a quadruple of f32 channels back to f16 bit patterns.
#[inline]
fn pack(p: [f32; 4]) -> [u16; 4] {
    [
        f32_to_f16_bits(p[0]),
        f32_to_f16_bits(p[1]),
        f32_to_f16_bits(p[2]),
        f32_to_f16_bits(p[3]),
    ]
}

/// Scale a premultiplied pixel uniformly by a clamped layer opacity in `[0, 1]`.
#[inline]
pub(crate) fn scale_premultiplied(px: [u16; 4], opacity: f32) -> [u16; 4] {
    let op = if opacity.is_nan() {
        1.0_f32
    } else {
        opacity.clamp(0.0_f32, 1.0_f32)
    };
    if (op - 1.0_f32).abs() <= f32::EPSILON {
        return px;
    }
    if op <= 0.0_f32 {
        return [0, 0, 0, 0];
    }
    let s = unpack(px);
    pack([s[0] * op, s[1] * op, s[2] * op, s[3] * op])
}

/// Composite a premultiplied pixel `src` over `dst` using `mode`.
#[inline]
pub fn blend_premultiplied(src: [u16; 4], dst: [u16; 4], mode: BlendMode) -> [u16; 4] {
    match mode {
        BlendMode::Normal => over_premultiplied(src, dst),
        BlendMode::Multiply => multiply(src, dst),
        BlendMode::Screen => screen(src, dst),
    }
}

//==============================================================================
// Compositing operators
//==============================================================================

/// Porter-Duff "src over dst" assuming premultiplied alpha on both inputs.
///
/// * `a_out = a_src + a_dst * (1 - a_src)`
/// * `c_out = c_src + c_dst * (1 - a_src)` for each colour channel
///
/// Inputs and outputs are f16 bit patterns (R, G, B, A).
pub fn over_premultiplied(src: [u16; 4], dst: [u16; 4]) -> [u16; 4] {
    let s = unpack(src);
    let d = unpack(dst);
    let inv_a = 1.0_f32 - s[3];
    pack([
        s[0] + d[0] * inv_a,
        s[1] + d[1] * inv_a,
        s[2] + d[2] * inv_a,
        s[3] + d[3] * inv_a,
    ])
}

/// Porter-Duff "src over dst" for straight-alpha (unpremultiplied) inputs.
///
/// * `a_out = a_src + a_dst * (1 - a_src)`
/// * `c_out = (c_src * a_src + c_dst * a_dst * (1 - a_src)) / a_out`
///
/// If `a_out` is zero the function short-circuits to a fully transparent
/// pixel (all four channels zero) — this avoids division by zero and
/// matches the convention used by most image libraries.
pub fn over_unpremultiplied(src: [u16; 4], dst: [u16; 4]) -> [u16; 4] {
    let s = unpack(src);
    let d = unpack(dst);
    let inv_a = 1.0_f32 - s[3];
    let a_out = s[3] + d[3] * inv_a;
    if a_out <= 0.0_f32 {
        return [0, 0, 0, 0];
    }
    let inv_a_out = 1.0_f32 / a_out;
    pack([
        (s[0] * s[3] + d[0] * d[3] * inv_a) * inv_a_out,
        (s[1] * s[3] + d[1] * d[3] * inv_a) * inv_a_out,
        (s[2] * s[3] + d[2] * d[3] * inv_a) * inv_a_out,
        a_out,
    ])
}

/// Apply an f16 mask multiplier to `src`'s alpha channel, then composite
/// using `over_premultiplied`.
///
/// This is the per-pixel work a brush tip does: the tip carries an alpha
/// stamp (the "mask") that modulates how strongly the stroke colour is
/// laid down on the canvas at that pixel. The mask is an f16 bit pattern
/// nominally in `[0.0, 1.0]`; values outside the range are clamped.
///
/// Because the input is premultiplied, multiplying the alpha by the mask
/// alone would leave the colour channels over-bright. We therefore scale
/// all four channels by `mask` so the premultiplied invariant survives.
pub fn masked_blend(src: [u16; 4], dst: [u16; 4], mask: u16) -> [u16; 4] {
    let m = f16_bits_to_f32(mask).clamp(0.0_f32, 1.0_f32);
    let s = unpack(src);
    let scaled_src = pack([s[0] * m, s[1] * m, s[2] * m, s[3] * m]);
    over_premultiplied(scaled_src, dst)
}

/// Composite a stack of 4096-pixel layers from bottom (index 0) to top
/// (last index) using `over_premultiplied`, returning the flattened
/// result. An empty stack returns a fully transparent tile.
///
/// The accumulator lives on the stack; no heap allocation occurs.
pub fn flatten_layer_stack(
    layers: &[[[u16; 4]; TILE_PIXEL_COUNT]],
) -> [[u16; 4]; TILE_PIXEL_COUNT] {
    let mut acc: [[u16; 4]; TILE_PIXEL_COUNT] = [[0u16; 4]; TILE_PIXEL_COUNT];
    if layers.is_empty() {
        return acc;
    }
    // Bottom layer goes in as-is.
    acc.copy_from_slice(&layers[0]);
    // Composite each subsequent layer on top.
    for layer_idx in 1..layers.len() {
        let layer = &layers[layer_idx];
        for px in 0..TILE_PIXEL_COUNT {
            acc[px] = over_premultiplied(layer[px], acc[px]);
        }
    }
    acc
}

//==============================================================================
// Linear interpolation
//==============================================================================

/// Channel-wise linear interpolation between two pixels.
///
/// * `t = 0` returns `a` exactly (modulo f16 round-trip).
/// * `t = 1` returns `b` exactly (modulo f16 round-trip).
/// * `t` is clamped to `[0, 1]` before use; values outside the range
///   would cause overshoot that is rarely what a caller wants.
///
/// Operates on all four channels uniformly, so it works on both
/// premultiplied and unpremultiplied data — the meaning of "halfway"
/// just differs between the two conventions.
pub fn lerp(a: [u16; 4], b: [u16; 4], t: u16) -> [u16; 4] {
    let s = f16_bits_to_f32(t).clamp(0.0_f32, 1.0_f32);
    let inv = 1.0_f32 - s;
    let av = unpack(a);
    let bv = unpack(b);
    pack([
        av[0] * inv + bv[0] * s,
        av[1] * inv + bv[1] * s,
        av[2] * inv + bv[2] * s,
        av[3] * inv + bv[3] * s,
    ])
}

//==============================================================================
// Separable blend modes (multiply, screen) over premultiplied alpha
//==============================================================================
//
// Derivation reference: W3C Compositing and Blending Level 2 §13. For a
// separable blend `B(cs, cd)` applied in unpremultiplied space, the
// premultiplied result is
//
//     co = Sca * (1 - Da) + Dca * (1 - Sa) + Sa * Da * B(Sca/Sa, Dca/Da)
//     ao = Sa + Da - Sa * Da     (same as plain "over")
//
// Substituting the closed forms simplifies to the per-channel formulas
// implemented below.

/// Photoshop-style **multiply** blend over premultiplied inputs.
///
/// Closed form per colour channel:
///
/// ```text
/// co = Sca * (1 - Da) + Dca * (1 - Sa) + Sca * Dca
/// ao = Sa + Da - Sa * Da
/// ```
///
/// White multiplied by anything is the other operand; black multiplied
/// by anything is black. Useful for shadow / darkening passes.
pub fn multiply(src: [u16; 4], dst: [u16; 4]) -> [u16; 4] {
    let s = unpack(src);
    let d = unpack(dst);
    let inv_sa = 1.0_f32 - s[3];
    let inv_da = 1.0_f32 - d[3];
    let a_out = s[3] + d[3] * inv_sa;
    pack([
        s[0] * inv_da + d[0] * inv_sa + s[0] * d[0],
        s[1] * inv_da + d[1] * inv_sa + s[1] * d[1],
        s[2] * inv_da + d[2] * inv_sa + s[2] * d[2],
        a_out,
    ])
}

/// Photoshop-style **screen** blend over premultiplied inputs.
///
/// Closed form per colour channel:
///
/// ```text
/// co = Sca + Dca - Sca * Dca
/// ao = Sa + Da - Sa * Da
/// ```
///
/// Screen is multiply's inverse: black screened with anything is the
/// other operand; white screened with anything is white. Useful for
/// highlight / lightening passes.
pub fn screen(src: [u16; 4], dst: [u16; 4]) -> [u16; 4] {
    let s = unpack(src);
    let d = unpack(dst);
    let a_out = s[3] + d[3] * (1.0_f32 - s[3]);
    pack([
        s[0] + d[0] - s[0] * d[0],
        s[1] + d[1] - s[1] * d[1],
        s[2] + d[2] - s[2] * d[2],
        a_out,
    ])
}

//==============================================================================
// Additional Porter-Duff operators (premultiplied alpha)
//==============================================================================
//
// All four assume premultiplied alpha on both inputs, matching
// `over_premultiplied`. Formulas follow Porter & Duff 1984 §3.

/// Porter-Duff **src in dst** — show src only where dst already has alpha.
///
/// * `co = Sca * Da`
/// * `ao = Sa  * Da`
pub fn in_op(src: [u16; 4], dst: [u16; 4]) -> [u16; 4] {
    let s = unpack(src);
    let da = f16_bits_to_f32(dst[3]);
    pack([s[0] * da, s[1] * da, s[2] * da, s[3] * da])
}

/// Porter-Duff **src out dst** — show src only where dst is empty.
///
/// * `co = Sca * (1 - Da)`
/// * `ao = Sa  * (1 - Da)`
pub fn out_op(src: [u16; 4], dst: [u16; 4]) -> [u16; 4] {
    let s = unpack(src);
    let inv_da = 1.0_f32 - f16_bits_to_f32(dst[3]);
    pack([s[0] * inv_da, s[1] * inv_da, s[2] * inv_da, s[3] * inv_da])
}

/// Porter-Duff **src atop dst** — src over dst clipped to dst's alpha.
///
/// * `co = Sca * Da + Dca * (1 - Sa)`
/// * `ao = Da`   (output alpha equals destination alpha)
pub fn atop(src: [u16; 4], dst: [u16; 4]) -> [u16; 4] {
    let s = unpack(src);
    let d = unpack(dst);
    let inv_sa = 1.0_f32 - s[3];
    pack([
        s[0] * d[3] + d[0] * inv_sa,
        s[1] * d[3] + d[1] * inv_sa,
        s[2] * d[3] + d[2] * inv_sa,
        d[3],
    ])
}

/// Porter-Duff **src xor dst** — non-overlapping union of the two.
///
/// * `co = Sca * (1 - Da) + Dca * (1 - Sa)`
/// * `ao = Sa  * (1 - Da) + Da  * (1 - Sa)`
pub fn xor(src: [u16; 4], dst: [u16; 4]) -> [u16; 4] {
    let s = unpack(src);
    let d = unpack(dst);
    let inv_sa = 1.0_f32 - s[3];
    let inv_da = 1.0_f32 - d[3];
    pack([
        s[0] * inv_da + d[0] * inv_sa,
        s[1] * inv_da + d[1] * inv_sa,
        s[2] * inv_da + d[2] * inv_sa,
        s[3] * inv_da + d[3] * inv_sa,
    ])
}

//==============================================================================
// Tile-level convenience
//==============================================================================

impl Tile {
    /// Composite `src` over `self` using bulk buffer reads/writes and return
    /// a freshly allocated tile holding the result.
    pub fn composite_over(&self, src: &Tile) -> Result<Tile, TileError> {
        self.composite_over_with_mode(src, 0, 0, 1.0_f32, BlendMode::Normal)
    }

    /// Composite `src` over `self` at grid position `(grid_x, grid_y)` with
    /// `src_opacity` (`[0, 1]`) and `mode`, using single-crossing bulk
    /// `read_buffer` / `write_buffer` operations rather than 12,288 per-pixel
    /// FFI calls.
    pub fn composite_over_with_mode(
        &self,
        src: &Tile,
        grid_x: u32,
        grid_y: u32,
        src_opacity: f32,
        mode: BlendMode,
    ) -> Result<Tile, TileError> {
        let out = Tile::alloc(grid_x, grid_y).ok_or(TileError::LibError)?;

        let mut dst_buf = [0u16; TILE_SCALARS];
        let mut src_buf = [0u16; TILE_SCALARS];
        self.read_buffer(&mut dst_buf)?;
        src.read_buffer(&mut src_buf)?;

        for px in 0..TILE_PIXEL_COUNT {
            let base = px * 4;
            let s = scale_premultiplied(
                [
                    src_buf[base],
                    src_buf[base + 1],
                    src_buf[base + 2],
                    src_buf[base + 3],
                ],
                src_opacity,
            );
            let d = [
                dst_buf[base],
                dst_buf[base + 1],
                dst_buf[base + 2],
                dst_buf[base + 3],
            ];
            let composed = blend_premultiplied(s, d, mode);
            dst_buf[base] = composed[0];
            dst_buf[base + 1] = composed[1];
            dst_buf[base + 2] = composed[2];
            dst_buf[base + 3] = composed[3];
        }

        out.write_buffer(&dst_buf)?;
        Ok(out)
    }
}

//==============================================================================
// Tests
//==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // f16 bit patterns we reuse in several tests.
    const F16_ZERO: u16 = 0x0000;
    const F16_HALF: u16 = 0x3800;
    const F16_ONE: u16 = 0x3C00;

    /// Compare two f32 channels within an f16-precision tolerance.
    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1.0e-2_f32
    }

    fn approx_eq_pixel(a: [u16; 4], b: [f32; 4]) -> bool {
        let af = [
            f16_bits_to_f32(a[0]),
            f16_bits_to_f32(a[1]),
            f16_bits_to_f32(a[2]),
            f16_bits_to_f32(a[3]),
        ];
        approx_eq(af[0], b[0])
            && approx_eq(af[1], b[1])
            && approx_eq(af[2], b[2])
            && approx_eq(af[3], b[3])
    }

    // ─── over_premultiplied ────────────────────────────────────────────

    #[test]
    fn over_premul_transparent_dst_returns_src() {
        // Premultiplied red at full alpha: (1, 0, 0, 1).
        let src = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let dst = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        let out = over_premultiplied(src, dst);
        assert_eq!(out, src);
    }

    #[test]
    fn over_premul_transparent_src_returns_dst() {
        let src = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        // Premultiplied green at full alpha: (0, 1, 0, 1).
        let dst = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = over_premultiplied(src, dst);
        assert_eq!(out, dst);
    }

    #[test]
    fn over_premul_opaque_src_returns_src() {
        // Opaque (premultiplied) src on any dst yields src.
        let src = [F16_HALF, F16_HALF, F16_HALF, F16_ONE];
        let dst = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let out = over_premultiplied(src, dst);
        assert_eq!(out, src);
    }

    #[test]
    fn over_premul_half_alpha_on_opaque() {
        // Premultiplied half-alpha blue over opaque red.
        // src = (0, 0, 0.5, 0.5); dst = (1, 0, 0, 1).
        // inv_a = 0.5. RGB = (0+1*0.5, 0+0*0.5, 0.5+0*0.5) = (0.5, 0, 0.5).
        // A = 0.5 + 1*0.5 = 1.0.
        let src = [F16_ZERO, F16_ZERO, F16_HALF, F16_HALF];
        let dst = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let out = over_premultiplied(src, dst);
        assert!(approx_eq_pixel(out, [0.5, 0.0, 0.5, 1.0]));
    }

    // ─── over_unpremultiplied ──────────────────────────────────────────

    #[test]
    fn over_unpremul_transparent_dst_returns_src() {
        let src = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let dst = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        let out = over_unpremultiplied(src, dst);
        assert!(approx_eq_pixel(out, [1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn over_unpremul_transparent_src_returns_dst() {
        let src = [F16_ONE, F16_ZERO, F16_ZERO, F16_ZERO];
        let dst = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = over_unpremultiplied(src, dst);
        assert!(approx_eq_pixel(out, [0.0, 1.0, 0.0, 1.0]));
    }

    #[test]
    fn over_unpremul_double_transparent_is_zero() {
        let src = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        let dst = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        let out = over_unpremultiplied(src, dst);
        assert_eq!(out, [0, 0, 0, 0]);
    }

    #[test]
    fn over_unpremul_half_alpha_on_opaque() {
        let src = [F16_ZERO, F16_ZERO, F16_ONE, F16_HALF];
        let dst = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let out = over_unpremultiplied(src, dst);
        assert!(approx_eq_pixel(out, [0.5, 0.0, 0.5, 1.0]));
    }

    // ─── masked_blend ──────────────────────────────────────────────────

    #[test]
    fn masked_blend_mask_zero_returns_dst() {
        let src = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let dst = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = masked_blend(src, dst, F16_ZERO);
        assert_eq!(out, dst);
    }

    #[test]
    fn masked_blend_mask_one_equals_over_premultiplied() {
        let src = [F16_ZERO, F16_ZERO, F16_HALF, F16_HALF];
        let dst = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let plain = over_premultiplied(src, dst);
        let masked = masked_blend(src, dst, F16_ONE);
        assert_eq!(masked, plain);
    }

    #[test]
    fn masked_blend_half_mask_dims_src() {
        let src = [F16_ZERO, F16_ZERO, F16_HALF, F16_HALF];
        let dst = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let out = masked_blend(src, dst, F16_HALF);
        assert!(approx_eq_pixel(out, [0.75, 0.0, 0.25, 1.0]));
    }

    // ─── flatten_layer_stack ───────────────────────────────────────────

    #[test]
    fn flatten_empty_stack_is_transparent() {
        let result = flatten_layer_stack(&[]);
        assert_eq!(result[0], [0, 0, 0, 0]);
        assert_eq!(result[TILE_PIXEL_COUNT - 1], [0, 0, 0, 0]);
    }

    #[test]
    fn flatten_single_layer_returns_layer() {
        let mut layer = [[0u16; 4]; TILE_PIXEL_COUNT];
        let red = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        for px in 0..TILE_PIXEL_COUNT {
            layer[px] = red;
        }
        let stack = [layer];
        let result = flatten_layer_stack(&stack);
        assert_eq!(result[0], red);
        assert_eq!(result[TILE_PIXEL_COUNT / 2], red);
        assert_eq!(result[TILE_PIXEL_COUNT - 1], red);
    }

    #[test]
    fn flatten_two_layers_red_under_half_blue() {
        let red = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let blue_half_premul = [F16_ZERO, F16_ZERO, F16_HALF, F16_HALF];

        let bottom = [red; TILE_PIXEL_COUNT];
        let top = [blue_half_premul; TILE_PIXEL_COUNT];
        let stack = [bottom, top];

        let result = flatten_layer_stack(&stack);
        for sample_idx in [0, 1, TILE_PIXEL_COUNT / 2, TILE_PIXEL_COUNT - 1] {
            assert!(
                approx_eq_pixel(result[sample_idx], [0.5, 0.0, 0.5, 1.0]),
                "pixel {sample_idx} = {:?}",
                result[sample_idx]
            );
        }
    }

    // ─── Tile::composite_over (integration; needs libpt) ───────────────

    #[test]
    fn tile_composite_over_uniform_red_under_transparent_yields_red() {
        let dst = match Tile::alloc(0, 0) {
            Some(t) => t,
            None => return,
        };
        let src = Tile::alloc(0, 0).expect("alloc src");
        if dst.fill_f32(1.0, 0.0, 0.0, 1.0).is_err() {
            return;
        }
        let composed = match dst.composite_over(&src) {
            Ok(t) => t,
            Err(_) => return,
        };
        let p = composed.read_pixel_f32(0, 0).expect("read");
        assert!(approx_eq(p[0], 1.0));
        assert!(approx_eq(p[1], 0.0));
        assert!(approx_eq(p[2], 0.0));
        assert!(approx_eq(p[3], 1.0));
    }

    #[test]
    fn tile_composite_over_half_alpha_blue_on_opaque_red_blends() {
        let dst = match Tile::alloc(0, 0) {
            Some(t) => t,
            None => return,
        };
        let src = Tile::alloc(0, 0).expect("alloc src");
        if dst.fill_f32(1.0, 0.0, 0.0, 1.0).is_err() {
            return;
        }
        if src.fill_f32(0.0, 0.0, 0.5, 0.5).is_err() {
            return;
        }
        let composed = dst.composite_over(&src).expect("composite_over");
        let p00 = composed.read_pixel_f32(0, 0).expect("read (0,0)");
        assert!(approx_eq(p00[0], 0.5), "R(0,0) = {}", p00[0]);
        assert!(approx_eq(p00[1], 0.0), "G(0,0) = {}", p00[1]);
        assert!(approx_eq(p00[2], 0.5), "B(0,0) = {}", p00[2]);
        assert!(approx_eq(p00[3], 1.0), "A(0,0) = {}", p00[3]);

        let p63 = composed
            .read_pixel_f32(TILE_SIZE - 1, TILE_SIZE - 1)
            .expect("read (63,63)");
        assert!(approx_eq(p63[0], 0.5), "R(63,63) = {}", p63[0]);
        assert!(approx_eq(p63[1], 0.0), "G(63,63) = {}", p63[1]);
        assert!(approx_eq(p63[2], 0.5), "B(63,63) = {}", p63[2]);
        assert!(approx_eq(p63[3], 1.0), "A(63,63) = {}", p63[3]);
    }

    #[test]
    fn tile_composite_over_transparent_src_yields_dst_everywhere() {
        let dst = match Tile::alloc(0, 0) {
            Some(t) => t,
            None => return,
        };
        let src = Tile::alloc(0, 0).expect("alloc src");
        if dst.fill_f32(0.0, 1.0, 0.0, 1.0).is_err() {
            return;
        }
        let composed = dst.composite_over(&src).expect("composite_over");
        let probes: [(u32, u32); 5] = [
            (0, 0),
            (TILE_SIZE - 1, 0),
            (0, TILE_SIZE - 1),
            (TILE_SIZE - 1, TILE_SIZE - 1),
            (TILE_SIZE / 2, TILE_SIZE / 2),
        ];
        for (px, py) in probes {
            let p = composed
                .read_pixel_f32(px, py)
                .unwrap_or_else(|_| panic!("read ({px},{py})"));
            assert!(approx_eq(p[0], 0.0), "R({px},{py}) = {}", p[0]);
            assert!(approx_eq(p[1], 1.0), "G({px},{py}) = {}", p[1]);
            assert!(approx_eq(p[2], 0.0), "B({px},{py}) = {}", p[2]);
            assert!(approx_eq(p[3], 1.0), "A({px},{py}) = {}", p[3]);
        }
    }

    #[test]
    fn tile_composite_over_opaque_red_on_opaque_green_yields_red() {
        let dst = match Tile::alloc(0, 0) {
            Some(t) => t,
            None => return,
        };
        let src = Tile::alloc(0, 0).expect("alloc src");
        if dst.fill_f32(0.0, 1.0, 0.0, 1.0).is_err() {
            return;
        }
        if src.fill_f32(1.0, 0.0, 0.0, 1.0).is_err() {
            return;
        }
        let composed = dst.composite_over(&src).expect("composite_over");
        let probes: [(u32, u32); 4] = [
            (0, 0),
            (17, 41),
            (TILE_SIZE / 2, TILE_SIZE / 2),
            (TILE_SIZE - 1, TILE_SIZE - 1),
        ];
        for (px, py) in probes {
            let p = composed
                .read_pixel_f32(px, py)
                .unwrap_or_else(|_| panic!("read ({px},{py})"));
            assert!(approx_eq(p[0], 1.0), "R({px},{py}) = {}", p[0]);
            assert!(approx_eq(p[1], 0.0), "G({px},{py}) = {}", p[1]);
            assert!(approx_eq(p[2], 0.0), "B({px},{py}) = {}", p[2]);
            assert!(approx_eq(p[3], 1.0), "A({px},{py}) = {}", p[3]);
        }
    }

    // ─── lerp ──────────────────────────────────────────────────────────

    #[test]
    fn lerp_at_zero_is_a() {
        let a = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let b = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = lerp(a, b, F16_ZERO);
        assert!(approx_eq_pixel(out, [1.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn lerp_at_one_is_b() {
        let a = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let b = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = lerp(a, b, F16_ONE);
        assert!(approx_eq_pixel(out, [0.0, 1.0, 0.0, 1.0]));
    }

    #[test]
    fn lerp_at_half_is_midpoint() {
        let a = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let b = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = lerp(a, b, F16_HALF);
        assert!(approx_eq_pixel(out, [0.5, 0.5, 0.0, 1.0]));
    }

    #[test]
    fn lerp_clamps_overshoot() {
        let a = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let b = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let t_two: u16 = 0x4000;
        let out = lerp(a, b, t_two);
        assert!(approx_eq_pixel(out, [0.0, 1.0, 0.0, 1.0]));
    }

    // ─── multiply ──────────────────────────────────────────────────────

    #[test]
    fn multiply_with_white_opaque_returns_src() {
        let src = [F16_HALF, F16_HALF, F16_HALF, F16_ONE];
        let white = [F16_ONE, F16_ONE, F16_ONE, F16_ONE];
        let out = multiply(src, white);
        assert!(approx_eq_pixel(out, [0.5, 0.5, 0.5, 1.0]));
    }

    #[test]
    fn multiply_with_black_opaque_returns_black() {
        let src = [F16_ONE, F16_HALF, F16_ZERO, F16_ONE];
        let black = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ONE];
        let out = multiply(src, black);
        assert!(approx_eq_pixel(out, [0.0, 0.0, 0.0, 1.0]));
    }

    #[test]
    fn multiply_alpha_is_over_alpha() {
        let src = [F16_HALF, F16_ZERO, F16_ZERO, F16_HALF];
        let dst = [F16_ZERO, F16_HALF, F16_ZERO, F16_HALF];
        let out = multiply(src, dst);
        let out_a = f16_bits_to_f32(out[3]);
        assert!(approx_eq(out_a, 0.75));
    }

    // ─── screen ────────────────────────────────────────────────────────

    #[test]
    fn screen_with_black_opaque_returns_src() {
        let src = [F16_HALF, F16_HALF, F16_ZERO, F16_ONE];
        let black = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ONE];
        let out = screen(src, black);
        assert!(approx_eq_pixel(out, [0.5, 0.5, 0.0, 1.0]));
    }

    #[test]
    fn screen_with_white_opaque_returns_white() {
        let src = [F16_HALF, F16_HALF, F16_ZERO, F16_ONE];
        let white = [F16_ONE, F16_ONE, F16_ONE, F16_ONE];
        let out = screen(src, white);
        assert!(approx_eq_pixel(out, [1.0, 1.0, 1.0, 1.0]));
    }

    #[test]
    fn screen_alpha_is_over_alpha() {
        let src = [F16_ZERO, F16_ZERO, F16_HALF, F16_HALF];
        let dst = [F16_HALF, F16_ZERO, F16_ZERO, F16_HALF];
        let out = screen(src, dst);
        assert!(approx_eq(f16_bits_to_f32(out[3]), 0.75));
    }

    // ─── in_op ─────────────────────────────────────────────────────────

    #[test]
    fn in_op_against_transparent_dst_is_transparent() {
        let src = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let dst = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        let out = in_op(src, dst);
        assert_eq!(out, [0, 0, 0, 0]);
    }

    #[test]
    fn in_op_against_opaque_dst_returns_src() {
        let src = [F16_HALF, F16_ZERO, F16_HALF, F16_HALF];
        let dst = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = in_op(src, dst);
        assert!(approx_eq_pixel(out, [0.5, 0.0, 0.5, 0.5]));
    }

    // ─── out_op ────────────────────────────────────────────────────────

    #[test]
    fn out_op_against_transparent_dst_returns_src() {
        let src = [F16_HALF, F16_ZERO, F16_HALF, F16_HALF];
        let dst = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        let out = out_op(src, dst);
        assert!(approx_eq_pixel(out, [0.5, 0.0, 0.5, 0.5]));
    }

    #[test]
    fn out_op_against_opaque_dst_is_transparent() {
        let src = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let dst = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = out_op(src, dst);
        assert_eq!(out, [0, 0, 0, 0]);
    }

    // ─── atop ──────────────────────────────────────────────────────────

    #[test]
    fn atop_alpha_equals_dst_alpha() {
        let src = [F16_HALF, F16_ZERO, F16_ZERO, F16_HALF];
        let dst = [F16_ZERO, F16_HALF, F16_ZERO, F16_HALF];
        let out = atop(src, dst);
        assert!(approx_eq(f16_bits_to_f32(out[3]), 0.5));
    }

    #[test]
    fn atop_against_transparent_dst_is_transparent() {
        let src = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let dst = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        let out = atop(src, dst);
        assert_eq!(out, [0, 0, 0, 0]);
    }

    #[test]
    fn atop_opaque_src_on_opaque_dst_is_src() {
        let src = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let dst = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = atop(src, dst);
        assert!(approx_eq_pixel(out, [1.0, 0.0, 0.0, 1.0]));
    }

    // ─── xor ───────────────────────────────────────────────────────────

    #[test]
    fn xor_opaque_with_opaque_is_transparent() {
        let src = [F16_ONE, F16_ZERO, F16_ZERO, F16_ONE];
        let dst = [F16_ZERO, F16_ONE, F16_ZERO, F16_ONE];
        let out = xor(src, dst);
        assert_eq!(out, [0, 0, 0, 0]);
    }

    #[test]
    fn xor_with_transparent_dst_returns_src() {
        let src = [F16_HALF, F16_ZERO, F16_HALF, F16_HALF];
        let dst = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        let out = xor(src, dst);
        assert!(approx_eq_pixel(out, [0.5, 0.0, 0.5, 0.5]));
    }

    #[test]
    fn xor_with_transparent_src_returns_dst() {
        let src = [F16_ZERO, F16_ZERO, F16_ZERO, F16_ZERO];
        let dst = [F16_HALF, F16_ZERO, F16_HALF, F16_HALF];
        let out = xor(src, dst);
        assert!(approx_eq_pixel(out, [0.5, 0.0, 0.5, 0.5]));
    }
}

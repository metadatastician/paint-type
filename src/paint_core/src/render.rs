// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Region renderer -- flatten the visible layer stack into straight-alpha
// RGBA8 for display or PNG export.
//
// Reads each overlapped tile once via `Tile::read_buffer` rather than per
// pixel; an f16 composite over a tile is ~a few microseconds in-process, vs.
// 4096 FFI crossings. Output is straight-alpha because HTML5 ImageData and
// PNG RGBA8 both carry unpremultiplied channels; tiles inside paint_core are
// premultiplied throughout, so the unpremultiply happens once, right here at
// the display boundary.

use crate::composite::{blend_premultiplied, scale_premultiplied};
use crate::layer::{LayerStack, TileCoord};
use crate::{f16_bits_to_f32, TILE_SCALARS, TILE_SIZE};

/// Maximum byte budget accepted by [`try_render_region`] / [`render_region`]
/// (1 GiB, comfortably sufficient for 16384×16384 RGBA8 while preventing
/// `u32` wraparound panics or hostile multi-terabyte allocation attempts).
pub const MAX_RENDER_BYTES: usize = 1024 * 1024 * 1024;

/// Composite the rectangle `(x, y, width, height)` of `stack` bottom-to-top
/// into a freshly allocated straight-alpha RGBA8 buffer (`width * height * 4`
/// bytes, row-major).
///
/// Pixels covered by no visible tile come out `[0, 0, 0, 0]`. Returns an empty
/// `Vec` if `width == 0`, `height == 0`, or `width * height * 4` overflows /
/// exceeds [`MAX_RENDER_BYTES`].
pub fn render_region(stack: &LayerStack, x: u32, y: u32, width: u32, height: u32) -> Vec<u8> {
    try_render_region(stack, x, y, width, height).unwrap_or_default()
}

/// Fallible counterpart to [`render_region`]. Returns `None` if
/// `width * height * 4` overflows `usize` or exceeds [`MAX_RENDER_BYTES`].
pub fn try_render_region(
    stack: &LayerStack,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
) -> Option<Vec<u8>> {
    let buf_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|px| px.checked_mul(4))?;
    if buf_len > MAX_RENDER_BYTES {
        return None;
    }
    if buf_len == 0 {
        return Some(Vec::new());
    }

    let mut acc = vec![[0u16; 4]; width as usize * height as usize];

    let x_end = (x as u64).saturating_add(width as u64);
    let y_end = (y as u64).saturating_add(height as u64);
    let tile_size_u64 = TILE_SIZE as u64;

    let tile_x0 = (x as u64 / tile_size_u64) as u32;
    let tile_y0 = (y as u64 / tile_size_u64) as u32;
    let tile_x1 = ((x_end - 1) / tile_size_u64) as u32;
    let tile_y1 = ((y_end - 1) / tile_size_u64) as u32;

    let mut tile_buf = [0u16; TILE_SCALARS];

    for ty in tile_y0..=tile_y1 {
        let tile_origin_y = (ty as u64) * tile_size_u64;
        let py_start = x_or_max(y as u64, tile_origin_y);
        let py_end = y_end.min(tile_origin_y + tile_size_u64);

        for tx in tile_x0..=tile_x1 {
            let tile_origin_x = (tx as u64) * tile_size_u64;
            let px_start = x_or_max(x as u64, tile_origin_x);
            let px_end = x_end.min(tile_origin_x + tile_size_u64);
            let coord = TileCoord::new(tx, ty);

            for (_, layer) in stack.iter() {
                if !layer.visible {
                    continue;
                }
                let opacity = layer.opacity();
                if opacity <= 0.0 {
                    continue;
                }
                let Some(tile) = layer.tile(coord) else {
                    continue;
                };
                if tile.read_buffer(&mut tile_buf).is_err() {
                    continue;
                }

                for py in py_start..py_end {
                    let ly = (py - tile_origin_y) as usize;
                    let dst_row = ((py - y as u64) as usize) * width as usize;
                    for px in px_start..px_end {
                        let lx = (px - tile_origin_x) as usize;
                        let src_i = (ly * TILE_SIZE as usize + lx) * 4;
                        let src = scale_premultiplied(
                            [
                                tile_buf[src_i],
                                tile_buf[src_i + 1],
                                tile_buf[src_i + 2],
                                tile_buf[src_i + 3],
                            ],
                            opacity,
                        );
                        let dst_i = dst_row + (px - x as u64) as usize;
                        acc[dst_i] = blend_premultiplied(src, acc[dst_i], layer.blend);
                    }
                }
            }
        }
    }

    let mut out = vec![0u8; buf_len];
    for (px_i, &p) in acc.iter().enumerate() {
        let r = f16_bits_to_f32(p[0]);
        let g = f16_bits_to_f32(p[1]);
        let b = f16_bits_to_f32(p[2]);
        let a = f16_bits_to_f32(p[3]).clamp(0.0, 1.0);
        let o = px_i * 4;
        if a <= 0.0 {
            out[o..o + 4].copy_from_slice(&[0, 0, 0, 0]);
        } else {
            let inv = 1.0 / a;
            out[o] = to_u8(r * inv);
            out[o + 1] = to_u8(g * inv);
            out[o + 2] = to_u8(b * inv);
            out[o + 3] = to_u8(a);
        }
    }
    Some(out)
}

#[inline]
fn x_or_max(a: u64, b: u64) -> u64 {
    if a > b {
        a
    } else {
        b
    }
}

#[inline]
fn to_u8(v: f32) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{BlendMode, Layer};
    use crate::Tile;

    fn solid_tile(x: u32, y: u32, rgba: [f32; 4]) -> Tile {
        let t = Tile::alloc(x, y).expect("tile alloc");
        t.fill_f32(rgba[0], rgba[1], rgba[2], rgba[3])
            .expect("fill");
        t
    }

    #[test]
    fn empty_stack_renders_transparent() {
        let stack = LayerStack::new();
        let out = render_region(&stack, 0, 0, 4, 4);
        assert_eq!(out, vec![0u8; 64]);
    }

    #[test]
    fn zero_sized_region_is_empty() {
        let stack = LayerStack::new();
        assert!(render_region(&stack, 10, 10, 0, 5).is_empty());
        assert!(render_region(&stack, 10, 10, 5, 0).is_empty());
    }

    #[test]
    fn overflow_dimensions_do_not_panic() {
        let stack = LayerStack::new();
        assert_eq!(try_render_region(&stack, 0, 0, u32::MAX, 2), None);
        assert!(render_region(&stack, 0, 0, u32::MAX, 2).is_empty());
    }

    #[test]
    fn opaque_red_over_nothing_round_trips_to_straight_rgba8() {
        let mut stack = LayerStack::new();
        let mut layer = Layer::new("bg");
        layer.put_tile(TileCoord::new(0, 0), solid_tile(0, 0, [1.0, 0.0, 0.0, 1.0]));
        stack.push(layer);

        let out = render_region(&stack, 0, 0, 2, 2);
        assert_eq!(out, [255, 0, 0, 255].repeat(4));
    }

    #[test]
    fn unpremultiplies_at_display_boundary() {
        let mut stack = LayerStack::new();
        let mut layer = Layer::new("fg");
        layer.put_tile(TileCoord::new(0, 0), solid_tile(0, 0, [0.5, 0.0, 0.0, 0.5]));
        stack.push(layer);

        let out = render_region(&stack, 0, 0, 1, 1);
        assert_eq!(&out[..3], &[255, 0, 0]);
        assert!((out[3] as i16 - 128).abs() <= 1, "alpha was {}", out[3]);
    }

    #[test]
    fn respects_layer_opacity_visibility_and_blend() {
        let mut stack = LayerStack::new();

        let mut base = Layer::new("base");
        base.put_tile(TileCoord::new(0, 0), solid_tile(0, 0, [1.0, 0.0, 0.0, 1.0]));
        stack.push(base);

        let mut hidden = Layer::new("hidden");
        hidden.visible = false;
        hidden.put_tile(TileCoord::new(0, 0), solid_tile(0, 0, [0.0, 1.0, 0.0, 1.0]));
        stack.push(hidden);

        let mut top = Layer::new("top");
        top.set_opacity(0.5);
        top.blend = BlendMode::Normal;
        top.put_tile(TileCoord::new(0, 0), solid_tile(0, 0, [0.0, 0.0, 1.0, 1.0]));
        stack.push(top);

        let out = render_region(&stack, 0, 0, 1, 1);
        assert!((out[0] as i16 - 128).abs() <= 2, "R was {}", out[0]);
        assert_eq!(out[1], 0);
        assert!((out[2] as i16 - 128).abs() <= 2, "B was {}", out[2]);
        assert_eq!(out[3], 255);
    }

    #[test]
    fn region_straddling_four_tiles_stitches_cleanly() {
        let mut stack = LayerStack::new();
        let mut layer = Layer::new("grid");
        layer.put_tile(TileCoord::new(0, 0), solid_tile(0, 0, [1.0, 0.0, 0.0, 1.0]));
        layer.put_tile(TileCoord::new(1, 0), solid_tile(1, 0, [0.0, 1.0, 0.0, 1.0]));
        layer.put_tile(TileCoord::new(0, 1), solid_tile(0, 1, [0.0, 0.0, 1.0, 1.0]));
        stack.push(layer);

        let out = render_region(&stack, 63, 63, 2, 2);
        assert_eq!(
            out,
            vec![
                255, 0, 0, 255, // (63, 63) -> tile (0, 0)
                0, 255, 0, 255, // (64, 63) -> tile (1, 0)
                0, 0, 255, 255, // (63, 64) -> tile (0, 1)
                0, 0, 0, 0, // (64, 64) -> tile (1, 1), unallocated
            ]
        );
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Regression & maintenance audit integration suite for `paint_core`.
// Kept in a dedicated integration target so `tests/e2e_pipeline.rs` retains
// its exact two-test output asserted by `tests/e2e/scenario_pipeline_dogfood.sh`.

use paint_core::{
    brush::{Brush, BrushTip, Stroke},
    layer::{BlendMode, Layer, LayerStack, TileCoord},
    render::{render_region, try_render_region},
    Tile,
};

#[test]
fn b06_render_region_rejects_u32_wraparound_without_panicking() {
    let stack = LayerStack::new();
    assert_eq!(try_render_region(&stack, 0, 0, u32::MAX, 4), None);
    assert!(render_region(&stack, 0, 0, u32::MAX, 4).is_empty());
    let clipped = try_render_region(&stack, u32::MAX - 2, 0, 8, 8);
    assert_eq!(clipped.map(|v| v.len()), Some(256));
}

#[test]
fn b07_stroke_ignores_non_finite_samples_and_avoids_f32_drift() {
    let tip = BrushTip::soft_round(8); // spacing = 2.0
    let brush = Brush::new(tip, [1.0, 0.0, 0.0, 1.0], 0.25);
    let mut stroke = Stroke::new();

    // Non-finite inputs before start do not start the stroke.
    assert!(stroke.push(f32::NAN, 0.0, &brush).is_empty());
    assert!(stroke.push(0.0, f32::INFINITY, &brush).is_empty());
    assert!(!stroke.has_started());

    // Start at (0, 0) and feed 10,000 sub-pixel 0.1px steps (= 1,000.0px total).
    // With spacing = 2.0px, exact f64 carry produces 1 initial + 500 run = 501 stamps
    // ending right at x = 1000.0 without cumulative drift.
    let mut count = stroke.push(0.0, 0.0, &brush).len();
    let mut last = (0.0_f32, 0.0_f32);
    for step in 1..=10_000 {
        if step == 5_000 {
            // Interleaved NaN/Inf must not poison ongoing stroke state.
            assert!(stroke.push(f32::NAN, f32::NEG_INFINITY, &brush).is_empty());
        }
        let x = (step as f64 * 0.1) as f32;
        for s in stroke.push(x, 0.0, &brush) {
            count += 1;
            last = s;
        }
    }
    assert!(
        (499..=501).contains(&count),
        "unexpected stamp count {count}"
    );
    assert!(
        (last.0 - 1000.0).abs() <= 2.1,
        "last stamp drifted to {}",
        last.0
    );
}

#[test]
fn b14_b17_flatten_honours_opacity_and_blend_modes_with_bulk_buffers() {
    let coord = TileCoord::new(5, 7);

    // Base white layer.
    let mut base = Layer::new("base");
    let t_base = Tile::alloc(coord.x, coord.y).expect("alloc base");
    t_base.fill_f32(1.0, 1.0, 1.0, 1.0).expect("fill base");
    base.put_tile(coord, t_base);

    // 50% opacity pure red Multiply layer -> over white gives (1.0, 0.5, 0.5, 1.0).
    let mut mult = Layer::new("multiply_half");
    mult.set_opacity(0.5);
    mult.blend = BlendMode::Multiply;
    let t_mult = Tile::alloc(coord.x, coord.y).expect("alloc mult");
    t_mult.fill_f32(1.0, 0.0, 0.0, 1.0).expect("fill mult");
    mult.put_tile(coord, t_mult);

    let mut stack = LayerStack::new();
    stack.push(base);
    stack.push(mult);

    let flat = stack.flatten("merged").expect("flatten");
    let merged_tile = flat.tile(coord).expect("flattened tile at (5, 7)");
    let px = merged_tile
        .read_pixel_f32(16, 16)
        .expect("read merged pixel");
    assert!((px[0] - 1.0).abs() < 1e-2, "R = {}", px[0]);
    assert!((px[1] - 0.5).abs() < 1e-2, "G = {}", px[1]);
    assert!((px[2] - 0.5).abs() < 1e-2, "B = {}", px[2]);
    assert!((px[3] - 1.0).abs() < 1e-2, "A = {}", px[3]);
}

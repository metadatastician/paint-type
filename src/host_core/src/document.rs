// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Document -- canvas dimensions, the layer stack, the active layer, the
// non-destructive branching undo graph (`paint_core::undo::UndoGraph`),
// the current brush, stroke state, and effect-plugin execution.

use std::collections::{HashMap, VecDeque};

use paint_core::brush::{Brush, BrushTip, Stroke};
use paint_core::layer::{BlendMode as CoreBlendMode, Layer, LayerId, LayerStack, TileCoord};
use paint_core::render::render_region;
use paint_core::undo::{RevId, UndoGraph};
use paint_core::{f16_bits_to_f32, f32_to_f16_bits, Tile, TILE_SCALARS, TILE_SIZE};
use paint_type_plugins::effect::EffectPlugin;
use paint_type_plugins::PluginRegistry;
use ptype_format::{
    BlendMode as PtypeBlendMode, Canvas as PtypeCanvas, Layer as PtypeLayer, Tile as PtypeTile,
    TileKey,
};

use crate::protocol::{LayerInfo, ToolKind};

const MAX_HISTORY_REVS: usize = 64;
const MAX_HISTORY_BYTES: usize = 256 * 1024 * 1024;

fn snapshot_byte_cost(canvas: &PtypeCanvas) -> usize {
    let mut bytes = core::mem::size_of::<PtypeCanvas>();
    if let Some(author) = &canvas.author {
        bytes = bytes.saturating_add(author.len());
    }
    if let Some(icc) = &canvas.icc_profile {
        bytes = bytes.saturating_add(icc.len());
    }
    for layer in &canvas.layers {
        let tile_bytes = layer
            .tiles
            .len()
            .saturating_mul(TILE_SCALARS * core::mem::size_of::<u16>());
        bytes = bytes
            .saturating_add(core::mem::size_of::<PtypeLayer>())
            .saturating_add(layer.name.len())
            .saturating_add(tile_bytes);
    }
    bytes
}

/// An axis-aligned dirty rectangle in canvas pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

fn core_to_ptype_blend(b: CoreBlendMode) -> PtypeBlendMode {
    match b {
        CoreBlendMode::Normal => PtypeBlendMode::Normal,
        CoreBlendMode::Multiply => PtypeBlendMode::Multiply,
        CoreBlendMode::Screen => PtypeBlendMode::Screen,
    }
}

fn ptype_to_core_blend(b: PtypeBlendMode) -> CoreBlendMode {
    match b {
        PtypeBlendMode::Normal => CoreBlendMode::Normal,
        PtypeBlendMode::Multiply => CoreBlendMode::Multiply,
        PtypeBlendMode::Screen => CoreBlendMode::Screen,
    }
}

fn blend_name(b: CoreBlendMode) -> &'static str {
    match b {
        CoreBlendMode::Normal => "normal",
        CoreBlendMode::Multiply => "multiply",
        CoreBlendMode::Screen => "screen",
    }
}

fn parse_blend_name(raw: &str) -> Result<CoreBlendMode, String> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "normal" => Ok(CoreBlendMode::Normal),
        "multiply" => Ok(CoreBlendMode::Multiply),
        "screen" => Ok(CoreBlendMode::Screen),
        other => Err(format!("unknown blend mode '{other}'")),
    }
}

pub struct Document {
    width: u32,
    height: u32,
    background: [f32; 4],
    author: Option<String>,
    icc_profile: Option<Vec<u8>>,
    stack: LayerStack,
    layer_meta: HashMap<LayerId, (Option<u32>, bool)>,
    active: LayerId,
    colour: [f32; 4],
    diameter: u32,
    hardness: f32,
    stroke: Stroke,
    stroke_in_progress: bool,
    brush: Brush,
    tool: ToolKind,
    history: UndoGraph<PtypeCanvas>,
    history_bytes: usize,
    current_rev: RevId,
    redo_stack: Vec<RevId>,
    plugins: PluginRegistry,
}

impl Document {
    /// Create a document with one empty layer named "Layer 1" and initialise
    /// its `UndoGraph` root snapshot.
    pub fn new(width: u32, height: u32) -> Self {
        let width = width.max(1);
        let height = height.max(1);
        let mut stack = LayerStack::new();
        let active = stack.push(Layer::new("Layer 1"));
        let colour = [0.0, 0.0, 0.0, 1.0];
        let diameter = 16;
        let hardness = 0.0;

        let mut initial = PtypeCanvas::new(width, height);
        initial.layers.push(PtypeLayer::new("Layer 1"));
        let initial_cost = snapshot_byte_cost(&initial);

        Self {
            width,
            height,
            background: [0.0; 4],
            author: None,
            icc_profile: None,
            stack,
            layer_meta: HashMap::new(),
            active,
            colour,
            diameter,
            hardness,
            stroke: Stroke::new(),
            stroke_in_progress: false,
            brush: build_brush(diameter, hardness, colour),
            tool: ToolKind::Brush,
            history: UndoGraph::new(initial),
            history_bytes: initial_cost,
            current_rev: RevId::ROOT,
            redo_stack: Vec::new(),
            plugins: PluginRegistry::new(),
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    pub fn plugins(&self) -> &PluginRegistry {
        &self.plugins
    }

    pub fn plugins_mut(&mut self) -> &mut PluginRegistry {
        &mut self.plugins
    }

    fn flush_pending_stroke(&mut self) {
        if self.stroke_in_progress {
            self.stroke_in_progress = false;
            self.record_revision();
        }
    }

    fn record_revision(&mut self) {
        let snapshot = self.to_ptype_canvas();
        let new_cost = snapshot_byte_cost(&snapshot);

        if self.history.len() >= MAX_HISTORY_REVS
            || self.history_bytes.saturating_add(new_cost) > MAX_HISTORY_BYTES
        {
            let budget = MAX_HISTORY_BYTES.saturating_sub(new_cost);
            let mut kept: Vec<PtypeCanvas> = Vec::new();
            let mut kept_bytes: usize = 0;
            let mut cursor = Some(self.current_rev);
            while let Some(rev) = cursor {
                if kept.len() >= MAX_HISTORY_REVS - 1 {
                    break;
                }
                let Some(entry) = self.history.checkout(rev) else {
                    break;
                };
                let c = snapshot_byte_cost(entry);
                if kept_bytes.saturating_add(c) > budget {
                    break;
                }
                kept_bytes = kept_bytes.saturating_add(c);
                kept.push(entry.clone());
                cursor = self.history.parent_of(rev);
            }
            kept.reverse();
            let mut iter = kept.into_iter();
            if let Some(root) = iter.next() {
                let mut compacted = UndoGraph::new(root);
                let mut cur = RevId::ROOT;
                for item in iter {
                    cur = compacted.commit(cur, item);
                }
                self.current_rev = compacted.commit(cur, snapshot);
                self.history = compacted;
                self.history_bytes = kept_bytes.saturating_add(new_cost);
            } else {
                self.history = UndoGraph::new(snapshot);
                self.current_rev = RevId::ROOT;
                self.history_bytes = new_cost;
            }
        } else {
            self.current_rev = self.history.commit(self.current_rev, snapshot);
            self.history_bytes = self.history_bytes.saturating_add(new_cost);
        }

        self.redo_stack.clear();
    }

    fn restore_ptype_canvas_internal(&mut self, ptype_canvas: &PtypeCanvas) -> Rect {
        self.width = ptype_canvas.width.max(1);
        self.height = ptype_canvas.height.max(1);
        self.background = ptype_canvas.background;
        self.author = ptype_canvas.author.clone();
        self.icc_profile = ptype_canvas.icc_profile.clone();
        self.stack = LayerStack::new();
        self.layer_meta.clear();

        let mut layer_ids: Vec<LayerId> = Vec::with_capacity(ptype_canvas.layers.len());

        for ptype_layer in &ptype_canvas.layers {
            let mut layer = Layer::new(&ptype_layer.name);
            layer.visible = ptype_layer.visible;
            layer.set_opacity(ptype_layer.opacity as f32);
            layer.blend = ptype_to_core_blend(ptype_layer.blend);

            for (key, ptype_tile) in &ptype_layer.tiles {
                let coord = TileCoord::new(key.tx, key.ty);
                let Some(tile) = Tile::alloc(coord.x, coord.y) else {
                    continue;
                };
                let mut buf = [0u16; TILE_SCALARS];
                buf.copy_from_slice(&ptype_tile.pixels);
                let _ = tile.write_buffer(&buf);
                layer.put_tile(coord, tile);
            }

            let pushed_id = self.stack.push(layer);
            if ptype_layer.layer_id.is_some() || ptype_layer.locked {
                self.layer_meta
                    .insert(pushed_id, (ptype_layer.layer_id, ptype_layer.locked));
            }
            layer_ids.push(pushed_id);
        }

        self.active = match ptype_canvas.active_layer {
            Some(idx) if (idx as usize) < layer_ids.len() => layer_ids[idx as usize],
            _ => layer_ids.first().copied().unwrap_or_else(|| {
                let layer = Layer::new("Layer 1");
                self.stack.push(layer)
            }),
        };

        Rect {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        }
    }

    /// Step backward one revision along the `UndoGraph` parent chain.
    pub fn undo(&mut self) -> Option<Rect> {
        self.flush_pending_stroke();
        let parent = self.history.parent_of(self.current_rev)?;
        let snapshot = self.history.checkout(parent)?.clone();
        self.redo_stack.push(self.current_rev);
        self.current_rev = parent;
        Some(self.restore_ptype_canvas_internal(&snapshot))
    }

    /// Step forward one revision along `redo_stack` or latest child branch.
    pub fn redo(&mut self) -> Option<Rect> {
        self.flush_pending_stroke();
        let next_rev = self
            .redo_stack
            .pop()
            .or_else(|| self.history.children_of(self.current_rev).last().copied())?;
        let snapshot = self.history.checkout(next_rev)?.clone();
        self.current_rev = next_rev;
        Some(self.restore_ptype_canvas_internal(&snapshot))
    }

    pub fn set_colour(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.colour = [r, g, b, a];
        self.brush = build_brush(self.diameter, self.hardness, self.colour);
    }

    pub fn set_brush(&mut self, diameter: u32, hardness: f32) {
        self.diameter = diameter.clamp(1, TILE_SIZE);
        self.hardness = if hardness.is_nan() {
            0.0
        } else {
            hardness.clamp(0.0, 1.0)
        };
        self.brush = build_brush(self.diameter, self.hardness, self.colour);
    }

    pub fn set_tool(&mut self, kind: ToolKind) {
        self.tool = kind;
    }

    /// Describe all layers in bottom-to-top stack order plus the active index.
    pub fn list_layers(&self) -> (Vec<LayerInfo>, usize) {
        let mut out = Vec::with_capacity(self.stack.len());
        let mut active_index = 0usize;
        for (idx, (id, layer)) in self.stack.iter().enumerate() {
            if id == self.active {
                active_index = idx;
            }
            out.push(LayerInfo {
                index: idx,
                id: id.as_u32(),
                name: layer.name.clone(),
                visible: layer.visible,
                opacity: layer.opacity(),
                blend: blend_name(layer.blend).to_string(),
            });
        }
        (out, active_index)
    }

    /// Push a new transparent layer onto the stack and make it active.
    pub fn add_layer(&mut self, name: Option<String>) -> usize {
        self.flush_pending_stroke();
        let next_num = self.stack.len() + 1;
        let label = name
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| format!("Layer {next_num}"));
        let id = self.stack.push(Layer::new(label));
        self.active = id;
        self.record_revision();
        self.stack.len().saturating_sub(1)
    }

    /// Delete the layer at `index` (refuses to delete the last remaining layer).
    pub fn delete_layer(&mut self, index: usize) -> Result<Rect, String> {
        self.flush_pending_stroke();
        if self.stack.len() <= 1 {
            return Err("cannot delete the only remaining layer".to_string());
        }
        let id = self
            .stack
            .id_at(index)
            .ok_or_else(|| format!("layer index {index} out of bounds"))?;
        self.stack.delete(id);
        if self.stack.position_of(self.active).is_none() {
            let fallback_idx = index.min(self.stack.len().saturating_sub(1));
            if let Some(new_active) = self.stack.id_at(fallback_idx) {
                self.active = new_active;
            }
        }
        self.record_revision();
        Ok(Rect {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        })
    }

    /// Select the active painting layer by 0-based stack index.
    pub fn select_layer(&mut self, index: usize) -> Result<(), String> {
        let id = self
            .stack
            .id_at(index)
            .ok_or_else(|| format!("layer index {index} out of bounds"))?;
        self.active = id;
        Ok(())
    }

    /// Reorder a layer from stack index `from` to `to`.
    pub fn reorder_layer(&mut self, from: usize, to: usize) -> Result<Rect, String> {
        self.flush_pending_stroke();
        let id = self
            .stack
            .id_at(from)
            .ok_or_else(|| format!("layer index {from} out of bounds"))?;
        self.stack
            .reorder_to(id, to)
            .map_err(|_| format!("target layer index {to} out of bounds"))?;
        self.record_revision();
        Ok(Rect {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        })
    }

    /// Toggle layer visibility at `index`.
    pub fn set_layer_visibility(&mut self, index: usize, visible: bool) -> Result<Rect, String> {
        self.flush_pending_stroke();
        let id = self
            .stack
            .id_at(index)
            .ok_or_else(|| format!("layer index {index} out of bounds"))?;
        if let Some(layer) = self.stack.get_mut(id) {
            layer.visible = visible;
        }
        self.record_revision();
        Ok(Rect {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        })
    }

    /// Set layer opacity at `index`.
    pub fn set_layer_opacity(&mut self, index: usize, opacity: f32) -> Result<Rect, String> {
        self.flush_pending_stroke();
        let id = self
            .stack
            .id_at(index)
            .ok_or_else(|| format!("layer index {index} out of bounds"))?;
        if let Some(layer) = self.stack.get_mut(id) {
            layer.set_opacity(opacity);
        }
        self.record_revision();
        Ok(Rect {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        })
    }

    /// Set layer blend mode (`"normal"`, `"multiply"`, `"screen"`) at `index`.
    pub fn set_layer_blend(&mut self, index: usize, blend: &str) -> Result<Rect, String> {
        self.flush_pending_stroke();
        let mode = parse_blend_name(blend)?;
        let id = self
            .stack
            .id_at(index)
            .ok_or_else(|| format!("layer index {index} out of bounds"))?;
        if let Some(layer) = self.stack.get_mut(id) {
            layer.blend = mode;
        }
        self.record_revision();
        Ok(Rect {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        })
    }

    /// Apply an `EffectPlugin` across every allocated tile of the active layer
    /// and record the resulting state into `UndoGraph`.
    pub fn apply_effect(&mut self, effect: &dyn EffectPlugin) -> Result<Rect, String> {
        self.flush_pending_stroke();
        let active = self.active;
        let coords: Vec<TileCoord> = self
            .stack
            .get(active)
            .map(|l| l.tiles().map(|(c, _)| c).collect())
            .unwrap_or_default();

        for coord in coords {
            let mut buf = [0u16; TILE_SCALARS];
            if let Some(layer) = self.stack.get(active) {
                if let Some(tile) = layer.tile(coord) {
                    let _ = tile.read_buffer(&mut buf);
                }
            }

            let mut rgba8 = vec![0u8; TILE_SIZE as usize * TILE_SIZE as usize * 4];
            for px_i in 0..(TILE_SIZE as usize * TILE_SIZE as usize) {
                let s = px_i * 4;
                let a = f16_bits_to_f32(buf[s + 3]).clamp(0.0, 1.0);
                let o = px_i * 4;
                if a > 0.0 {
                    let inv = 1.0 / a;
                    rgba8[o] = (f16_bits_to_f32(buf[s]) * inv * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    rgba8[o + 1] = (f16_bits_to_f32(buf[s + 1]) * inv * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    rgba8[o + 2] = (f16_bits_to_f32(buf[s + 2]) * inv * 255.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    rgba8[o + 3] = (a * 255.0).round().clamp(0.0, 255.0) as u8;
                }
            }

            let transformed = effect
                .apply(&rgba8, TILE_SIZE, TILE_SIZE)
                .map_err(|e| e.to_string())?;
            let expected_len = TILE_SIZE as usize * TILE_SIZE as usize * 4;
            if transformed.len() != expected_len {
                return Err(format!(
                    "effect output byte length mismatch: expected {expected_len}, got {}",
                    transformed.len()
                ));
            }

            for px_i in 0..(TILE_SIZE as usize * TILE_SIZE as usize) {
                let o = px_i * 4;
                let a = transformed[o + 3] as f32 / 255.0;
                buf[o] = f32_to_f16_bits((transformed[o] as f32 / 255.0) * a);
                buf[o + 1] = f32_to_f16_bits((transformed[o + 1] as f32 / 255.0) * a);
                buf[o + 2] = f32_to_f16_bits((transformed[o + 2] as f32 / 255.0) * a);
                buf[o + 3] = f32_to_f16_bits(a);
            }

            if let Some(layer) = self.stack.get(active) {
                if let Some(tile) = layer.tile(coord) {
                    let _ = tile.write_buffer(&buf);
                }
            }
        }

        self.record_revision();
        Ok(Rect {
            x: 0,
            y: 0,
            w: self.width,
            h: self.height,
        })
    }

    /// Replace the document contents with the given straight-alpha RGBA8 image.
    /// Converts straight-alpha source pixels to premultiplied RGBA16F tiles and
    /// records a revision in `UndoGraph`.
    pub fn load_png(&mut self, rgba8: &[u8], w: u32, h: u32) -> Rect {
        self.flush_pending_stroke();
        self.width = w.max(1);
        self.height = h.max(1);
        self.background = [0.0; 4];
        self.stack = LayerStack::new();
        let active = self.stack.push(Layer::new("Layer 1"));
        self.active = active;

        let tiles_x = w.div_ceil(TILE_SIZE);
        let tiles_y = h.div_ceil(TILE_SIZE);

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                let Some(tile) = Tile::alloc(tx, ty) else {
                    continue;
                };
                let mut buf = [0u16; TILE_SCALARS];
                for row in 0..TILE_SIZE {
                    let canvas_y = ty * TILE_SIZE + row;
                    for col in 0..TILE_SIZE {
                        let canvas_x = tx * TILE_SIZE + col;
                        if canvas_x >= w || canvas_y >= h {
                            continue;
                        }
                        let src = ((canvas_y as usize) * (w as usize) + (canvas_x as usize)) * 4;
                        let dst = ((row * TILE_SIZE + col) * 4) as usize;
                        // Convert straight-alpha PNG bytes into premultiplied RGBA16F.
                        let a = rgba8[src + 3] as f32 / 255.0;
                        buf[dst] = f32_to_f16_bits((rgba8[src] as f32 / 255.0) * a);
                        buf[dst + 1] = f32_to_f16_bits((rgba8[src + 1] as f32 / 255.0) * a);
                        buf[dst + 2] = f32_to_f16_bits((rgba8[src + 2] as f32 / 255.0) * a);
                        buf[dst + 3] = f32_to_f16_bits(a);
                    }
                }
                let _ = tile.write_buffer(&buf);
                if let Some(layer) = self.stack.get_mut(active) {
                    layer.put_tile(TileCoord::new(tx, ty), tile);
                }
            }
        }

        self.record_revision();
        Rect { x: 0, y: 0, w, h }
    }

    fn stamp_at(&mut self, cx: f32, cy: f32) -> Rect {
        if !cx.is_finite() || !cy.is_finite() {
            return Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            };
        }
        let half = self.diameter as f32 * 0.5;
        let min_x = (cx - half).floor().max(0.0) as u32;
        let min_y = (cy - half).floor().max(0.0) as u32;
        let max_x = ((cx + half).ceil() as i64).clamp(0, self.width as i64) as u32;
        let max_y = ((cy + half).ceil() as i64).clamp(0, self.height as i64) as u32;

        if max_x <= min_x || max_y <= min_y {
            return Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            };
        }

        let tile_x0 = min_x / TILE_SIZE;
        let tile_y0 = min_y / TILE_SIZE;
        let tile_x1 = (max_x - 1) / TILE_SIZE;
        let tile_y1 = (max_y - 1) / TILE_SIZE;
        let active = self.active;

        for ty in tile_y0..=tile_y1 {
            for tx in tile_x0..=tile_x1 {
                let coord = TileCoord::new(tx, ty);
                let Some(layer) = self.stack.get_mut(active) else {
                    continue;
                };
                if layer.tile(coord).is_none() {
                    let Some(tile) = Tile::alloc(coord.x, coord.y) else {
                        continue;
                    };
                    layer.put_tile(coord, tile);
                }
                if let Some(tile) = layer.tile(coord) {
                    let local_cx = cx - (tx * TILE_SIZE) as f32;
                    let local_cy = cy - (ty * TILE_SIZE) as f32;
                    if self.tool == ToolKind::Eraser {
                        let _ = self.brush.erase_stamp(tile, local_cx, local_cy);
                    } else {
                        let _ = self.brush.stamp(tile, local_cx, local_cy);
                    }
                }
            }
        }

        Rect {
            x: min_x,
            y: min_y,
            w: max_x - min_x,
            h: max_y - min_y,
        }
    }

    /// Begin a stroke. Commits any unclosed previous stroke, resets stroke
    /// state, and stamps the first dab.
    pub fn pointer_down(&mut self, x: f32, y: f32) -> Rect {
        self.flush_pending_stroke();
        self.stroke.reset();
        self.stroke_in_progress = true;
        let stamps = self.stroke.push(x, y, &self.brush);
        self.apply_stamps(&stamps)
    }

    /// Continue a stroke.
    pub fn pointer_move(&mut self, x: f32, y: f32) -> Rect {
        let stamps = self.stroke.push(x, y, &self.brush);
        self.apply_stamps(&stamps)
    }

    /// Finish a stroke and commit the new revision into `UndoGraph`.
    pub fn pointer_up(&mut self) {
        self.flush_pending_stroke();
    }

    fn apply_stamps(&mut self, stamps: &[(f32, f32)]) -> Rect {
        stamps
            .iter()
            .map(|&(cx, cy)| self.stamp_at(cx, cy))
            .filter(|r| r.w > 0 && r.h > 0)
            .reduce(union)
            .unwrap_or(Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            })
    }

    /// Render a canvas rectangle to straight-alpha RGBA8.
    pub fn render(&self, r: Rect) -> Vec<u8> {
        render_region(&self.stack, r.x, r.y, r.w, r.h)
    }

    /// Render the whole canvas (used for save and full repaints).
    pub fn render_all(&self) -> Vec<u8> {
        render_region(&self.stack, 0, 0, self.width, self.height)
    }

    /// Flood-fill from `(cx, cy)` with the current colour and commit a revision.
    pub fn fill_at(&mut self, cx: f32, cy: f32) -> Rect {
        if !cx.is_finite() || !cy.is_finite() {
            return Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            };
        }
        self.flush_pending_stroke();

        let seed_x = (cx.max(0.0) as u32).min(self.width.saturating_sub(1));
        let seed_y = (cy.max(0.0) as u32).min(self.height.saturating_sub(1));
        let seed_colour = self.pixel_rgba8(seed_x, seed_y);

        let mut scratch: HashMap<(u32, u32), [u16; TILE_SCALARS]> = HashMap::new();
        let mut touched_tiles: Vec<(u32, u32)> = Vec::new();
        let mut visited: HashMap<(u32, u32), ()> = HashMap::new();
        let mut queue: VecDeque<(u32, u32)> = VecDeque::new();
        let mut pixel_count: u32 = 0;
        const MAX_PIXELS: u32 = 4_000_000;

        queue.push_back((seed_x, seed_y));
        visited.insert((seed_x, seed_y), ());

        while let Some((px, py)) = queue.pop_front() {
            if pixel_count >= MAX_PIXELS {
                break;
            }

            let current = self.pixel_rgba8(px, py);
            if !colours_match(current, seed_colour) {
                continue;
            }

            pixel_count += 1;

            let tile_x = px / TILE_SIZE;
            let tile_y = py / TILE_SIZE;
            let key = (tile_x, tile_y);

            if let std::collections::hash_map::Entry::Vacant(e) = scratch.entry(key) {
                let mut buf = [0u16; TILE_SCALARS];
                let active = self.active;
                if let Some(layer) = self.stack.get_mut(active) {
                    let coord = TileCoord::new(tile_x, tile_y);
                    if layer.tile(coord).is_none() {
                        if let Some(tile) = Tile::alloc(tile_x, tile_y) {
                            layer.put_tile(coord, tile);
                        }
                    }
                    if let Some(tile) = layer.tile(coord) {
                        let _ = tile.read_buffer(&mut buf);
                    }
                }
                e.insert(buf);
                touched_tiles.push(key);
            }

            if let Some(buf) = scratch.get_mut(&key) {
                let local_x = px % TILE_SIZE;
                let local_y = py % TILE_SIZE;
                let idx = ((local_y as usize) * TILE_SIZE as usize + (local_x as usize)) * 4;
                let a = self.colour[3];
                buf[idx] = f32_to_f16_bits(self.colour[0] * a);
                buf[idx + 1] = f32_to_f16_bits(self.colour[1] * a);
                buf[idx + 2] = f32_to_f16_bits(self.colour[2] * a);
                buf[idx + 3] = f32_to_f16_bits(a);
            }

            let neighbours = [
                (px.wrapping_sub(1), py),
                (px + 1, py),
                (px, py.wrapping_sub(1)),
                (px, py + 1),
            ];
            for (nx, ny) in neighbours {
                if nx < self.width && ny < self.height && !visited.contains_key(&(nx, ny)) {
                    visited.insert((nx, ny), ());
                    queue.push_back((nx, ny));
                }
            }
        }

        if touched_tiles.is_empty() {
            return Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            };
        }

        let active = self.active;
        for &(tile_x, tile_y) in &touched_tiles {
            if let Some(buf) = scratch.get(&(tile_x, tile_y)) {
                let coord = TileCoord::new(tile_x, tile_y);
                if let Some(layer) = self.stack.get_mut(active) {
                    if let Some(tile) = layer.tile(coord) {
                        let _ = tile.write_buffer(buf);
                    }
                }
            }
        }

        self.record_revision();

        touched_tiles
            .iter()
            .map(|&(tx, ty)| {
                let x = tx * TILE_SIZE;
                let y = ty * TILE_SIZE;
                let w = TILE_SIZE.min(self.width.saturating_sub(x));
                let h = TILE_SIZE.min(self.height.saturating_sub(y));
                Rect { x, y, w, h }
            })
            .filter(|r| r.w > 0 && r.h > 0)
            .reduce(union)
            .unwrap_or(Rect {
                x: 0,
                y: 0,
                w: 0,
                h: 0,
            })
    }

    fn pixel_rgba8(&self, px: u32, py: u32) -> [u8; 4] {
        let tile_x = px / TILE_SIZE;
        let tile_y = py / TILE_SIZE;
        let coord = TileCoord::new(tile_x, tile_y);
        let active = self.active;
        if let Some(layer) = self.stack.get(active) {
            if let Some(tile) = layer.tile(coord) {
                let local_x = px % TILE_SIZE;
                let local_y = py % TILE_SIZE;
                if let Ok(bits) = tile.read_pixel_bits(local_x, local_y) {
                    return [
                        (f16_bits_to_f32(bits[0]) * 255.0).round().clamp(0.0, 255.0) as u8,
                        (f16_bits_to_f32(bits[1]) * 255.0).round().clamp(0.0, 255.0) as u8,
                        (f16_bits_to_f32(bits[2]) * 255.0).round().clamp(0.0, 255.0) as u8,
                        (f16_bits_to_f32(bits[3]) * 255.0).round().clamp(0.0, 255.0) as u8,
                    ];
                }
            }
        }
        [0, 0, 0, 0]
    }

    /// Replace document contents with `ptype_canvas` and commit into `UndoGraph`.
    pub fn load_ptype(&mut self, ptype_canvas: &PtypeCanvas) -> Rect {
        self.flush_pending_stroke();
        let rect = self.restore_ptype_canvas_internal(ptype_canvas);
        self.record_revision();
        rect
    }

    /// Convert the document to a lossless `.ptype` `Canvas` preserving
    /// background, blend modes, visibility, opacity, and metadata.
    pub fn to_ptype_canvas(&self) -> PtypeCanvas {
        let mut ptype_canvas = PtypeCanvas::new(self.width, self.height);
        ptype_canvas.background = self.background;
        ptype_canvas.author = self.author.clone();
        ptype_canvas.icc_profile = self.icc_profile.clone();

        for (idx, (layer_id, layer)) in self.stack.iter().enumerate() {
            if layer_id == self.active && idx > 0 {
                ptype_canvas.active_layer = Some(idx as u32);
            }
            let mut ptype_layer = PtypeLayer::new(&layer.name);
            ptype_layer.visible = layer.visible;
            ptype_layer.opacity = layer.opacity() as f64;
            ptype_layer.blend = core_to_ptype_blend(layer.blend);
            if let Some(&(ext_id, locked)) = self.layer_meta.get(&layer_id) {
                ptype_layer.layer_id = ext_id;
                ptype_layer.locked = locked;
            }

            for (coord, tile) in layer.tiles() {
                let mut buf = [0u16; TILE_SCALARS];
                let _ = tile.read_buffer(&mut buf);
                let key = TileKey {
                    tx: coord.x,
                    ty: coord.y,
                };
                ptype_layer.tiles.insert(key, PtypeTile { pixels: buf });
            }

            ptype_canvas.layers.push(ptype_layer);
        }

        ptype_canvas
    }
}

fn colours_match(a: [u8; 4], b: [u8; 4]) -> bool {
    (a[0] as i16 - b[0] as i16).abs() <= 2
        && (a[1] as i16 - b[1] as i16).abs() <= 2
        && (a[2] as i16 - b[2] as i16).abs() <= 2
        && (a[3] as i16 - b[3] as i16).abs() <= 2
}

fn build_brush(diameter: u32, hardness: f32, colour: [f32; 4]) -> Brush {
    let tip = if hardness >= 0.5 {
        BrushTip::hard_round(diameter)
    } else {
        BrushTip::soft_round(diameter)
    };
    Brush::new(tip, colour, 0.25)
}

fn union(a: Rect, b: Rect) -> Rect {
    let x0 = a.x.min(b.x);
    let y0 = a.y.min(b.y);
    let x1 = (a.x + a.w).max(b.x + b.w);
    let y1 = (a.y + a.h).max(b.y + b.h);
    Rect {
        x: x0,
        y: y0,
        w: x1 - x0,
        h: y1 - y0,
    }
}

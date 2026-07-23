use serde::Deserialize;
use std::fs;
use std::io::{self, BufRead};

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum AbiItem {
    Tile {
        #[serde(rename = "tileSize")]
        tile_size: u32,
        #[serde(rename = "rgba16fChannels")]
        channels: u32,
        magic: u32,
    },
    LayerStack {
        #[serde(rename = "maxLayers")]
        max_layers: u32,
        #[serde(rename = "maxLayerNameLen")]
        max_layer_name_len: u32,
        magic: u32,
    },
}

fn generate_tile_twasm(tile_size: u32, channels: u32, magic: u32) -> String {
    let bytes_per_pixel = channels * 2;
    let total_pixels = tile_size * tile_size;
    let pixel_bytes = total_pixels * bytes_per_pixel;
    
    format!("// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Joshua Jewell (JoshuaJewell)
//
// paint-type-tile.twasm — typed-wasm schema for paint-type's RGBA16F tile.
//
// STATUS: Verified bridge.
// Source for the AffineScript → typed-wasm bridge layer.
// This is the actual source the bridge generator emits, plus a corpus entry
// for the round-trip soundness property (typed-wasm#130).
//
// Surface mirrored: pt_tile_alloc / pt_tile_free / pt_tile_fill /
// pt_tile_read_pixel / pt_tile_write_pixel / pt_tile_blit / pt_version
// (see src/interface/ffi/src/main.zig in this repo).

// --- Region Declarations (the \"schema\") ---

// One pixel: four IEEE 754 binary16 channels in premultiplied alpha.
// 8 bytes total, naturally aligned.
region RGBA16F {{
    r: u16;   // f16 bit pattern; verifier treats as opaque u16
    g: u16;
    b: u16;
    a: u16;

    where 0 <= r;     // u16 range is its own bounds; placeholder for
    where 0 <= g;     // future f16-classifier constraint (TP-3 carrier).
    where 0 <= b;
    where 0 <= a;

    align 2;
}}

// Header: 16 bytes, magic + version + grid coordinates + pad.
region TileHeader {{
    magic:   u32;     // {magic:#010X} \"PTLE\" — written by pt_tile_alloc
    version: u32;     // schema version (bumped on layout change)
    grid_x:  u32;     // tile coordinate in the larger image plane
    grid_y:  u32;

    where magic == {magic:#010X};   // L5: structural pin

    align 4;
}}

// A full tile: 32 KiB pixel buffer + 16 B header = 32784 B.
// Mirror of struct `Tile` in src/interface/ffi/src/main.zig + the
// constants TILE_SIZE / TILE_CHANNELS / TILE_PIXEL_BYTES in
// src/paint_core/src/lib.rs.
region Tile {{
    header: @TileHeader;        // embedded
    pixels: @RGBA16F[{total_pixels}];  // {total_pixels} pixels × {bytes_per_pixel} B = {pixel_bytes} B

    align 8;
}}

// --- Memory Declaration ---

memory tile_memory {{
    initial: 1;        // 1 page = 64 KiB — fits one tile + headroom
    maximum: 1024;     // 64 MiB ceiling — caller-decided in v0.3.0

    place Tile at 0;
}}

// --- Functions ---

// Level 10: linear allocation. Returns an OWNING handle that must be
// freed exactly once. Mirrors `pt_tile_alloc(x, y) -> u64` where 0
// signals allocation failure.
fn alloc_tile(grid_x: u32, grid_y: u32) -> own region<Tile>
    effects {{ Alloc }}
{{
    region.alloc Tile {{
        header = TileHeader {{
            magic = {magic:#010X},
            version = 1,
            grid_x = grid_x,
            grid_y = grid_y
        }}
        // pixels left zero-initialised; caller calls fill_tile next.
    }} -> tile;
    return tile;
}}

// Level 10: consumes the linear handle. Mirrors `pt_tile_free`.
fn free_tile(tile: own region<Tile>)
    effects {{ Free }}
{{
    region.free $tile;
}}

// Level 8: write-effect on Tile. Mirrors `pt_tile_fill`.
// All four channels write under exclusive borrow — guaranteed by &mut.
fn fill_tile(
    tile: &mut region<Tile>,
    r: u16, g: u16, b: u16, a: u16
)
    effects {{ WriteRegion(Tile) }}
{{
    region.scan $tile.pixels -> |px| {{
        region.set $px .r, r;
        region.set $px .g, g;
        region.set $px .b, b;
        region.set $px .a, a;
    }}
}}

// Level 5: bounds-proven pixel read. Mirrors `pt_tile_read_pixel`.
// idx_x and idx_y are bounded to [0, {tile_size}) by the array type; the
// verifier eliminates the runtime bounds check.
fn read_pixel(
    tile: &region<Tile>,
    idx_x: i32, idx_y: i32
) -> @RGBA16F
    effects {{ ReadRegion(Tile) }}
{{
    let linear: i32 = idx_y * {tile_size} + idx_x;
    region.get $tile.pixels[linear] -> px;
    return px;
}}

// Level 8: write at one pixel. Mirrors `pt_tile_write_pixel`.
fn write_pixel(
    tile: &mut region<Tile>,
    idx_x: i32, idx_y: i32,
    r: u16, g: u16, b: u16, a: u16
)
    effects {{ WriteRegion(Tile) }}
{{
    let linear: i32 = idx_y * {tile_size} + idx_x;
    region.set $tile.pixels[linear] .r, r;
    region.set $tile.pixels[linear] .g, g;
    region.set $tile.pixels[linear] .b, b;
    region.set $tile.pixels[linear] .a, a;
}}

// Level 7: paired-region aliasing safety. Mirrors `pt_tile_blit(dst, src)`
// where dst and src are distinct tiles. The `&mut`/`&` split is what
// makes the no-self-blit guarantee statically checkable.
fn blit_tile(
    dst: &mut region<Tile>,
    src: &region<Tile>
)
    effects {{ ReadRegion(Tile), WriteRegion(Tile) }}
{{
    region.scan $src.pixels indexed -> |i, src_px| {{
        region.get $src_px .r -> sr;
        region.get $src_px .g -> sg;
        region.get $src_px .b -> sb;
        region.get $src_px .a -> sa;
        region.set $dst.pixels[i] .r, sr;
        region.set $dst.pixels[i] .g, sg;
        region.set $dst.pixels[i] .b, sb;
        region.set $dst.pixels[i] .a, sa;
    }}
}}
")
}

fn generate_layer_twasm(max_layers: u32, max_layer_name_len: u32, magic: u32) -> String {
    format!("// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Joshua Jewell (JoshuaJewell)
//
// paint-type-layer.twasm — typed-wasm schema for paint-type's layer
// metadata stack.
//
// STATUS: Verified bridge.
// Source for the AffineScript → typed-wasm bridge layer.
// Surface mirrored (see src/interface/ffi/src/main.zig in this repo):
//   pt_layer_stack_new / pt_layer_stack_free
//   pt_layer_push / pt_layer_delete / pt_layer_reorder_to
//   pt_layer_count / pt_layer_get_id_at / pt_layer_get_name
//   pt_layer_set_opacity / pt_layer_get_opacity
//   pt_layer_set_visible / pt_layer_get_visible
//
// Constants pinned to PT_LAYER_NAME_MAX = {max_layer_name_len}, PT_LAYER_MAX_PER_STACK = {max_layers}.

// --- Region Declarations ---

// A bounded UTF-8 name buffer. {max_layer_name_len} bytes — long enough for any human-
// authored layer name, short enough that a full stack of {max_layers} layers
// (LayerStack below) keeps under ~70 KiB per pt_layer_stack_new alloc.
region LayerName {{
    bytes: u8[{max_layer_name_len}];

    align 1;
}}

// A single layer's metadata. 272 bytes:
//   id (4) + name_len (4) + opacity_bits (4) + visible (4) + name ({max_layer_name_len}).
// `id == 0` is the PT_LAYER_ID_NONE sentinel; allocated ids are dense
// from 1 upward and never recycled after delete.
region Layer {{
    id:           u32;
    name_len:     u32;
    opacity_bits: u32;       // IEEE 754 binary32 bit pattern, clamped [0,1]
    visible:      u32;       // 0 = hidden, non-zero = visible
    name:         @LayerName;

    where name_len <= {max_layer_name_len};

    align 4;
}}

// The full stack: 16 B header + {max_layers} layer slots = ~70 KiB.
// Mirror of struct `PtLayerStack` in src/interface/ffi/src/main.zig.
// PT_LAYER_STACK_MAGIC = {magic:#010X} (\"PLST\") is the safety pin.
region LayerStack {{
    magic:       u32;
    layer_count: u32;
    next_id:     u32;
    _pad:        u32;
    layers:      @Layer[{max_layers}];

    where magic == {magic:#010X};
    where layer_count <= {max_layers};

    align 4;
}}

// --- Memory Declaration ---

memory layer_memory {{
    initial: 2;        // 2 pages = 128 KiB — one stack + headroom
    maximum: 2;        // exact: one stack lives here at a time

    place LayerStack at 0;
}}

// --- Functions ---

// Level 10: linear allocation. Mirrors `pt_layer_stack_new() -> u64`.
fn stack_new() -> own region<LayerStack>
    effects {{ Alloc }}
{{
    region.alloc LayerStack {{
        magic = {magic:#010X},
        layer_count = 0,
        next_id = 1,
        _pad = 0
        // layers[] zero-initialised; id 0 == PT_LAYER_ID_NONE.
    }} -> stack;
    return stack;
}}

// Level 10: consumes the linear handle. Mirrors `pt_layer_stack_free`.
fn stack_free(stack: own region<LayerStack>)
    effects {{ Free }}
{{
    region.free $stack;
}}

// Level 8: writes the new layer entry. Mirrors `pt_layer_push`.
// Returns the newly issued id, or PT_LAYER_ID_NONE (0) if the stack is
// full or name overflows.
fn push_layer(
    stack: &mut region<LayerStack>,
    name_buf: &region<LayerName>,
    name_len: u32
) -> u32
    effects {{ ReadRegion(LayerName), WriteRegion(LayerStack) }}
{{
    region.get $stack .layer_count -> count;
    if count >= {max_layers} {{
        return 0;            // full
    }}
    if name_len > {max_layer_name_len} {{
        return 0;            // name overflow
    }}

    region.get $stack .next_id -> new_id;

    region.set $stack.layers[count] .id, new_id;
    region.set $stack.layers[count] .name_len, name_len;
    region.set $stack.layers[count] .opacity_bits, 0x3F800000;  // 1.0f
    region.set $stack.layers[count] .visible, 1;
    // Copy name_buf into layers[count].name — modelled as a region.scan.
    region.scan $name_buf.bytes indexed -> |i, b| {{
        region.get $b -> byte_val;
        region.set $stack.layers[count].name.bytes[i], byte_val;
    }}

    region.set $stack .layer_count, count + 1;
    region.set $stack .next_id, new_id + 1;
    return new_id;
}}

// Level 5: looks up by id. Mirrors `pt_layer_get_id_at`.
// Returns 0 if position is out of range.
fn get_id_at(
    stack: &region<LayerStack>,
    position: u32
) -> u32
    effects {{ ReadRegion(LayerStack) }}
{{
    region.get $stack .layer_count -> count;
    if position >= count {{
        return 0;
    }}
    region.get $stack.layers[position] .id -> id;
    return id;
}}

// Level 8: clamps opacity_bits into [0.0, 1.0] (NaN → 1.0).
// Mirrors `pt_layer_set_opacity`. The clamp itself is modelled here as
// host responsibility — typed-wasm carries the f32 bits opaquely.
fn set_opacity(
    stack: &mut region<LayerStack>,
    id: u32,
    bits: u32
) -> u32
    effects {{ ReadRegion(LayerStack), WriteRegion(LayerStack) }}
{{
    region.get $stack .layer_count -> count;
    let mut found: u32 = 0;
    region.scan $stack.layers where id == id -> |layer| {{
        region.set $layer .opacity_bits, bits;
        found = 1;
    }}
    if found == 0 {{
        return 1;            // unknown id
    }}
    return 0;
}}
")
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stdin = io::stdin();
    
    for line_result in stdin.lock().lines() {
        let line = line_result?;
        if line.trim().is_empty() {
            continue;
        }
        
        let item: AbiItem = serde_json::from_str(&line)?;
        
        match item {
            AbiItem::Tile { tile_size, channels, magic } => {
                let twasm = generate_tile_twasm(tile_size, channels, magic);
                fs::write("../../src/bridges/paint-type-tile.twasm", twasm)?;
                println!("Generated src/bridges/paint-type-tile.twasm");
            }
            AbiItem::LayerStack { max_layers, max_layer_name_len, magic } => {
                let twasm = generate_layer_twasm(max_layers, max_layer_name_len, magic);
                fs::write("../../src/bridges/paint-type-layer.twasm", twasm)?;
                println!("Generated src/bridges/paint-type-layer.twasm");
            }
        }
    }
    
    Ok(())
}

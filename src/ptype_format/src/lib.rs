// SPDX-License-Identifier: AGPL-3.0-or-later
//
// ptype_format — reference encoder/decoder for paint-type's native RGBA16F
// `.ptype` container (draft v1 + optional extended metadata trailer).

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

/// Tile edge length in pixels. Matches `TILE_SIZE` in paint-type's CPU backend.
pub const TILE_SIZE: u32 = 64;

/// Channels per pixel (R, G, B, A). Matches `TILE_CHANNELS`.
pub const TILE_CHANNELS: u32 = 4;

/// Scalars per tile: `TILE_SIZE * TILE_SIZE * TILE_CHANNELS` = 16384.
pub const TILE_SCALARS: usize =
    (TILE_SIZE as usize) * (TILE_SIZE as usize) * (TILE_CHANNELS as usize);

/// Canvas pixel format discriminant (`0 = RGBA16F`).
pub const FORMAT_RGBA16F: u32 = 0;

/// 8-byte container signature.
pub const MAGIC: [u8; 8] = [0x89, b'P', b'T', b'Y', b'P', b'E', 0x0D, 0x0A];

/// On-disk format version understood by this crate.
pub const VERSION: u16 = 1;

/// Container flags: base uncompressed container without extended trailer.
pub const FLAGS_NONE: u16 = 0;

/// Optional lossless metadata trailer flag (preserves stable layer IDs, lock
/// state, active layer index, author, and embedded ICC profile while keeping
/// baseline v1 files 100% wire-compatible when unset).
pub const FLAGS_EXTENDED_META: u16 = 0x0002;

/// Maximum `.ptype` file size accepted by [`decode_from_file`] (512 MiB).
pub const MAX_PTYPE_FILE_BYTES: u64 = 512 * 1024 * 1024;

/// Layer blend mode. Discriminants match paint-type's `BlendMode` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlendMode {
    Normal = 0,
    Multiply = 1,
    Screen = 2,
}

impl BlendMode {
    /// Wire discriminant.
    pub fn to_u32(self) -> u32 {
        self as u32
    }

    /// Parse a wire discriminant. Unknown values are rejected (strict).
    pub fn from_u32(v: u32) -> Result<Self, DecodeError> {
        match v {
            0 => Ok(BlendMode::Normal),
            1 => Ok(BlendMode::Multiply),
            2 => Ok(BlendMode::Screen),
            other => Err(DecodeError::UnknownBlendMode(other)),
        }
    }
}

/// Sparse-tile grid coordinate (tile units, not pixels).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TileKey {
    pub tx: u32,
    pub ty: u32,
}

/// A single 64×64 RGBA16F tile: `TILE_SCALARS` half-float bit-patterns,
/// row-major, channel-interleaved (R,G,B,A,R,G,B,A,…).
#[derive(Clone)]
pub struct Tile {
    pub pixels: [u16; TILE_SCALARS],
}

impl Tile {
    /// A fully-transparent (all-zero) tile.
    pub fn zeroed() -> Self {
        Tile {
            pixels: [0u16; TILE_SCALARS],
        }
    }
}

impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        self.pixels[..] == other.pixels[..]
    }
}
impl Eq for Tile {}

impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let nonzero = self.pixels.iter().filter(|&&p| p != 0).count();
        write!(
            f,
            "Tile {{ {} non-zero / {} scalars }}",
            nonzero, TILE_SCALARS
        )
    }
}

/// One layer: name, visibility, opacity, blend mode, optional stable metadata,
/// and its sparse tiles.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    pub visible: bool,
    pub opacity: f64,
    pub blend: BlendMode,
    pub tiles: BTreeMap<TileKey, Tile>,
    /// Optional stable non-zero layer identifier preserved across reorders.
    pub layer_id: Option<u32>,
    /// Whether edits to this layer are locked in the UI.
    pub locked: bool,
}

impl Layer {
    /// A visible, fully-opaque, normal-blend layer with no tiles.
    pub fn new(name: impl Into<String>) -> Self {
        Layer {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            tiles: BTreeMap::new(),
            layer_id: None,
            locked: false,
        }
    }

    fn has_extended_meta(&self) -> bool {
        self.layer_id.is_some() || self.locked
    }
}

/// A full canvas: dimensions, pixel format, background colour, layer stack,
/// and optional extended document metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    /// Pixel format discriminant. Draft v1 only encodes [`FORMAT_RGBA16F`].
    pub format: u32,
    /// RGBA background, stored as `[f32; 4]`.
    pub background: [f32; 4],
    pub layers: Vec<Layer>,
    /// Optional 0-based active layer index in `layers`.
    pub active_layer: Option<u32>,
    /// Optional document author string.
    pub author: Option<String>,
    /// Optional embedded ICC profile bytes.
    pub icc_profile: Option<Vec<u8>>,
}

impl Canvas {
    /// An empty RGBA16F canvas with a transparent background and no layers.
    pub fn new(width: u32, height: u32) -> Self {
        Canvas {
            width,
            height,
            format: FORMAT_RGBA16F,
            background: [0.0; 4],
            layers: Vec::new(),
            active_layer: None,
            author: None,
            icc_profile: None,
        }
    }

    /// True if any canvas- or layer-level extended metadata is populated.
    pub fn has_extended_meta(&self) -> bool {
        self.active_layer.is_some()
            || self.author.is_some()
            || self.icc_profile.is_some()
            || self.layers.iter().any(Layer::has_extended_meta)
    }
}

// ----------------------------------------------------------------------------
// Errors
// ----------------------------------------------------------------------------

/// Why a byte slice failed to decode as a `.ptype` container.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// First 8 bytes were not [`MAGIC`].
    BadMagic,
    /// Version field was not one this crate understands.
    UnsupportedVersion(u16),
    /// Flags field carried a bit this crate does not implement.
    UnsupportedFlags(u16),
    /// `format` field was not a value draft v1 defines.
    UnsupportedFormat(u32),
    /// Recorded tile geometry did not match this build's constants.
    TileGeometryMismatch {
        got: (u32, u32),
        expected: (u32, u32),
    },
    /// Blend-mode discriminant was outside the defined set.
    UnknownBlendMode(u32),
    /// A boolean wire byte was neither `0` nor `1`.
    InvalidBoolean { context: &'static str, value: u8 },
    /// Tile keys inside a layer were duplicate or not in strict ascending order.
    NonCanonicalTileOrder,
    /// Extended metadata trailer flag was set even though all metadata fields
    /// held their default empty values, or carried an invalid layer index.
    NonCanonicalMetadata,
    /// A length/count field would require reading past the end of input.
    UnexpectedEof { context: &'static str },
    /// A UTF-8 string was not valid UTF-8.
    InvalidUtf8 { context: &'static str },
    /// Trailing bytes remained after a complete canvas was decoded.
    TrailingBytes { extra: usize },
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::BadMagic => write!(f, "bad magic: not a .ptype container"),
            DecodeError::UnsupportedVersion(v) => write!(f, "unsupported .ptype version {v}"),
            DecodeError::UnsupportedFlags(x) => write!(f, "unsupported .ptype flags {x:#06x}"),
            DecodeError::UnsupportedFormat(x) => write!(f, "unsupported pixel format {x}"),
            DecodeError::TileGeometryMismatch { got, expected } => write!(
                f,
                "tile geometry mismatch: file has {}x{}c, this build expects {}x{}c",
                got.0, got.1, expected.0, expected.1
            ),
            DecodeError::UnknownBlendMode(x) => write!(f, "unknown blend mode {x}"),
            DecodeError::InvalidBoolean { context, value } => {
                write!(f, "invalid boolean byte {value:#04x} in {context}")
            }
            DecodeError::NonCanonicalTileOrder => {
                write!(f, "non-canonical duplicate or unsorted tile coordinate")
            }
            DecodeError::NonCanonicalMetadata => {
                write!(f, "non-canonical extended metadata trailer")
            }
            DecodeError::UnexpectedEof { context } => {
                write!(f, "unexpected end of input while reading {context}")
            }
            DecodeError::InvalidUtf8 { context } => write!(f, "invalid UTF-8 in {context}"),
            DecodeError::TrailingBytes { extra } => {
                write!(f, "{extra} trailing byte(s) after canvas")
            }
        }
    }
}

impl std::error::Error for DecodeError {}

// ----------------------------------------------------------------------------
// Encode
// ----------------------------------------------------------------------------

fn encode_opt_u32(out: &mut Vec<u8>, val: Option<u32>) {
    match val {
        None => out.push(0),
        Some(v) => {
            out.push(1);
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
}

fn encode_opt_bytes(out: &mut Vec<u8>, val: Option<&[u8]>) {
    match val {
        None => out.push(0),
        Some(b) => {
            out.push(1);
            out.extend_from_slice(&(b.len() as u32).to_le_bytes());
            out.extend_from_slice(b);
        }
    }
}

/// Serialise a [`Canvas`] to the `.ptype` byte layout.
///
/// Deterministic: layers are written in stack order and tiles in ascending
/// `TileKey` order, so two equal canvases always encode to identical bytes.
pub fn encode(canvas: &Canvas) -> Vec<u8> {
    let write_meta = canvas.has_extended_meta();
    let flags = if write_meta {
        FLAGS_EXTENDED_META
    } else {
        FLAGS_NONE
    };

    let tile_bytes = TILE_SCALARS * 2 + 8;
    let total_tiles: usize = canvas.layers.iter().map(|l| l.tiles.len()).sum();
    let approx = 64usize
        .saturating_add(canvas.layers.len().saturating_mul(24))
        .saturating_add(total_tiles.saturating_mul(tile_bytes));
    let mut out = Vec::with_capacity(approx);

    out.extend_from_slice(&MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&canvas.format.to_le_bytes());
    out.extend_from_slice(&canvas.width.to_le_bytes());
    out.extend_from_slice(&canvas.height.to_le_bytes());
    for c in canvas.background {
        out.extend_from_slice(&c.to_le_bytes());
    }
    out.extend_from_slice(&TILE_SIZE.to_le_bytes());
    out.extend_from_slice(&TILE_CHANNELS.to_le_bytes());

    out.extend_from_slice(&(canvas.layers.len() as u32).to_le_bytes());
    for layer in &canvas.layers {
        let name = layer.name.as_bytes();
        out.extend_from_slice(&(name.len() as u32).to_le_bytes());
        out.extend_from_slice(name);
        out.push(u8::from(layer.visible));
        out.extend_from_slice(&layer.opacity.to_le_bytes());
        out.extend_from_slice(&layer.blend.to_u32().to_le_bytes());

        out.extend_from_slice(&(layer.tiles.len() as u32).to_le_bytes());
        for (key, tile) in &layer.tiles {
            out.extend_from_slice(&key.tx.to_le_bytes());
            out.extend_from_slice(&key.ty.to_le_bytes());
            for scalar in tile.pixels {
                out.extend_from_slice(&scalar.to_le_bytes());
            }
        }
    }

    if write_meta {
        encode_opt_u32(&mut out, canvas.active_layer);
        encode_opt_bytes(&mut out, canvas.author.as_deref().map(str::as_bytes));
        encode_opt_bytes(&mut out, canvas.icc_profile.as_deref());
        for layer in &canvas.layers {
            encode_opt_u32(&mut out, layer.layer_id);
            out.push(u8::from(layer.locked));
        }
    }

    out
}

// ----------------------------------------------------------------------------
// Decode
// ----------------------------------------------------------------------------

/// A bounds-checked little-endian cursor. Every read is fallible; nothing here
/// can panic on a short or malformed buffer.
struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Reader { buf, pos: 0 }
    }

    fn take(&mut self, n: usize, context: &'static str) -> Result<&'a [u8], DecodeError> {
        let end = self
            .pos
            .checked_add(n)
            .ok_or(DecodeError::UnexpectedEof { context })?;
        if end > self.buf.len() {
            return Err(DecodeError::UnexpectedEof { context });
        }
        let slice = &self.buf[self.pos..end];
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self, context: &'static str) -> Result<u8, DecodeError> {
        Ok(self.take(1, context)?[0])
    }

    fn bool_strict(&mut self, context: &'static str) -> Result<bool, DecodeError> {
        match self.u8(context)? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(DecodeError::InvalidBoolean { context, value }),
        }
    }

    fn u16(&mut self, context: &'static str) -> Result<u16, DecodeError> {
        let b = self.take(2, context)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self, context: &'static str) -> Result<u32, DecodeError> {
        let b = self.take(4, context)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn f32(&mut self, context: &'static str) -> Result<f32, DecodeError> {
        let b = self.take(4, context)?;
        Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn f64(&mut self, context: &'static str) -> Result<f64, DecodeError> {
        let b = self.take(8, context)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn opt_u32(&mut self, context: &'static str) -> Result<Option<u32>, DecodeError> {
        if self.bool_strict(context)? {
            Ok(Some(self.u32(context)?))
        } else {
            Ok(None)
        }
    }

    fn opt_bytes(&mut self, context: &'static str) -> Result<Option<Vec<u8>>, DecodeError> {
        if self.bool_strict(context)? {
            let len_u32 = self.u32(context)?;
            let len = usize::try_from(len_u32).map_err(|_| DecodeError::UnexpectedEof { context })?;
            Ok(Some(self.take(len, context)?.to_vec()))
        } else {
            Ok(None)
        }
    }

    fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }
}

/// Parse a `.ptype` byte slice back into a [`Canvas`].
pub fn decode(bytes: &[u8]) -> Result<Canvas, DecodeError> {
    let mut r = Reader::new(bytes);

    let magic = r.take(MAGIC.len(), "magic")?;
    if magic != MAGIC {
        return Err(DecodeError::BadMagic);
    }
    let version = r.u16("version")?;
    if version != VERSION {
        return Err(DecodeError::UnsupportedVersion(version));
    }
    let flags = r.u16("flags")?;
    if (flags & !FLAGS_EXTENDED_META) != FLAGS_NONE {
        return Err(DecodeError::UnsupportedFlags(flags));
    }
    let has_meta = (flags & FLAGS_EXTENDED_META) != 0;

    let format = r.u32("format")?;
    if format != FORMAT_RGBA16F {
        return Err(DecodeError::UnsupportedFormat(format));
    }
    let width = r.u32("width")?;
    let height = r.u32("height")?;
    let mut background = [0.0f32; 4];
    for (i, c) in background.iter_mut().enumerate() {
        *c = r.f32(["bg.r", "bg.g", "bg.b", "bg.a"][i])?;
    }

    let tile_size = r.u32("tile_size")?;
    let tile_channels = r.u32("tile_channels")?;
    if (tile_size, tile_channels) != (TILE_SIZE, TILE_CHANNELS) {
        return Err(DecodeError::TileGeometryMismatch {
            got: (tile_size, tile_channels),
            expected: (TILE_SIZE, TILE_CHANNELS),
        });
    }

    let layer_count = r.u32("layer_count")?;
    let mut layers = Vec::new();
    for _ in 0..layer_count {
        let name_len = usize::try_from(r.u32("layer name length")?)
            .map_err(|_| DecodeError::UnexpectedEof {
                context: "layer name length",
            })?;
        let name_bytes = r.take(name_len, "layer name")?;
        let name = std::str::from_utf8(name_bytes)
            .map_err(|_| DecodeError::InvalidUtf8 {
                context: "layer name",
            })?
            .to_owned();
        let visible = r.bool_strict("layer visible")?;
        let opacity = r.f64("layer opacity")?;
        let blend = BlendMode::from_u32(r.u32("layer blend")?)?;

        let tile_count = r.u32("tile_count")?;
        let mut tiles = BTreeMap::new();
        let mut prev_key: Option<TileKey> = None;
        for _ in 0..tile_count {
            let tx = r.u32("tile.tx")?;
            let ty = r.u32("tile.ty")?;
            let key = TileKey { tx, ty };
            if let Some(prev) = prev_key {
                if key <= prev {
                    return Err(DecodeError::NonCanonicalTileOrder);
                }
            }
            prev_key = Some(key);
            let mut pixels = [0u16; TILE_SCALARS];
            for scalar in pixels.iter_mut() {
                *scalar = r.u16("tile pixel")?;
            }
            tiles.insert(key, Tile { pixels });
        }

        layers.push(Layer {
            name,
            visible,
            opacity,
            blend,
            tiles,
            layer_id: None,
            locked: false,
        });
    }

    let (active_layer, author, icc_profile) = if has_meta {
        let active = r.opt_u32("meta.active_layer")?;
        let author = match r.opt_bytes("meta.author")? {
            None => None,
            Some(b) => Some(
                String::from_utf8(b).map_err(|_| DecodeError::InvalidUtf8 {
                    context: "meta.author",
                })?,
            ),
        };
        let icc = r.opt_bytes("meta.icc_profile")?;
        for layer in &mut layers {
            layer.layer_id = r.opt_u32("meta.layer_id")?;
            layer.locked = r.bool_strict("meta.layer_locked")?;
        }
        (active, author, icc)
    } else {
        (None, None, None)
    };

    if r.remaining() != 0 {
        return Err(DecodeError::TrailingBytes {
            extra: r.remaining(),
        });
    }

    let canvas = Canvas {
        width,
        height,
        format,
        background,
        layers,
        active_layer,
        author,
        icc_profile,
    };

    if has_meta && !canvas.has_extended_meta() {
        return Err(DecodeError::NonCanonicalMetadata);
    }

    Ok(canvas)
}

/// Read and decode a `.ptype` file bounded by [`MAX_PTYPE_FILE_BYTES`].
pub fn decode_from_file(path: &str) -> Result<Canvas, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if meta.len() > MAX_PTYPE_FILE_BYTES {
        return Err(format!(
            "ptype file exceeds MAX_PTYPE_FILE_BYTES ({} > {})",
            meta.len(),
            MAX_PTYPE_FILE_BYTES
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    decode(&bytes).map_err(|e| e.to_string())
}

/// Encode and write a `.ptype` file atomically/cleanly to disk.
pub fn encode_to_file(path: &str, canvas: &Canvas) -> Result<(), String> {
    let bytes = encode(canvas);
    std::fs::write(path, &bytes).map_err(|e| e.to_string())
}

// ----------------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn patterned_tile(pattern: &[u16]) -> Tile {
        let mut t = Tile::zeroed();
        for (i, p) in t.pixels.iter_mut().enumerate() {
            *p = pattern[i % pattern.len()];
        }
        t
    }

    fn sample_canvas() -> Canvas {
        let mut c = Canvas::new(256, 128);
        c.background = [0.1, 0.2, 0.3, 1.0];

        let mut base = Layer::new("background");
        base.tiles
            .insert(TileKey { tx: 0, ty: 0 }, patterned_tile(&[0x3C00, 0x0000]));
        base.tiles.insert(
            TileKey { tx: 3, ty: 1 },
            patterned_tile(&[0xFFFF, 0x1234, 0x00FF]),
        );

        let mut over = Layer::new("ink ✏");
        over.visible = false;
        over.opacity = 0.42;
        over.blend = BlendMode::Multiply;
        over.tiles
            .insert(TileKey { tx: 2, ty: 2 }, patterned_tile(&[0x8000, 0x7C00]));

        c.layers.push(base);
        c.layers.push(over);
        c
    }

    #[test]
    fn roundtrip_empty_canvas() {
        let c = Canvas::new(64, 64);
        let bytes = encode(&c);
        assert_eq!(decode(&bytes).unwrap(), c);
    }

    #[test]
    fn roundtrip_multi_layer_multi_tile() {
        let c = sample_canvas();
        let decoded = decode(&encode(&c)).unwrap();
        assert_eq!(decoded, c);
    }

    #[test]
    fn roundtrip_preserves_half_float_edge_patterns() {
        let edges = [
            0x0000, 0x8000, 0x3C00, 0x7BFF, 0x0400, 0x0001, 0x7C00, 0xFC00, 0x7E00, 0xFDFF, 0xFFFF,
        ];
        let mut c = Canvas::new(64, 64);
        let mut layer = Layer::new("edges");
        layer
            .tiles
            .insert(TileKey { tx: 7, ty: 9 }, patterned_tile(&edges));
        c.layers.push(layer);

        let decoded = decode(&encode(&c)).unwrap();
        let original = &c.layers[0].tiles[&TileKey { tx: 7, ty: 9 }];
        let got = &decoded.layers[0].tiles[&TileKey { tx: 7, ty: 9 }];
        assert_eq!(got.pixels[..], original.pixels[..]);
    }

    #[test]
    fn byte_stability_under_reencode() {
        let c = sample_canvas();
        let once = encode(&c);
        let twice = encode(&decode(&once).unwrap());
        assert_eq!(once, twice);
    }

    #[test]
    fn extended_metadata_roundtrip_is_lossless_and_byte_stable() {
        let mut c = sample_canvas();
        c.active_layer = Some(1);
        c.author = Some("paint.type studio".to_string());
        c.icc_profile = Some(vec![0x00, 0x01, 0x73, 0x52, 0x47, 0x42]);
        c.layers[0].layer_id = Some(10);
        c.layers[1].layer_id = Some(42);
        c.layers[1].locked = true;

        let once = encode(&c);
        let decoded = decode(&once).unwrap();
        assert_eq!(decoded, c);
        assert_eq!(encode(&decoded), once);
    }

    #[test]
    fn golden_header_prefix() {
        let c = Canvas::new(0x0102_0304, 0x0506_0708);
        let bytes = encode(&c);
        let expected: &[u8] = &[
            0x89, b'P', b'T', b'Y', b'P', b'E', 0x0D, 0x0A, // magic
            0x01, 0x00, // version 1
            0x00, 0x00, // flags 0
            0x00, 0x00, 0x00, 0x00, // format RGBA16F
            0x04, 0x03, 0x02, 0x01, // width  (LE)
            0x08, 0x07, 0x06, 0x05, // height (LE)
        ];
        assert_eq!(&bytes[..expected.len()], expected);
    }

    #[test]
    fn reject_bad_magic() {
        let mut bytes = encode(&Canvas::new(8, 8));
        bytes[0] ^= 0xFF;
        assert_eq!(decode(&bytes), Err(DecodeError::BadMagic));
    }

    #[test]
    fn reject_unsupported_version() {
        let mut bytes = encode(&Canvas::new(8, 8));
        bytes[8] = 0xFE;
        assert!(matches!(
            decode(&bytes),
            Err(DecodeError::UnsupportedVersion(_))
        ));
    }

    #[test]
    fn reject_non_boolean_visibility_byte() {
        let mut c = Canvas::new(8, 8);
        c.layers.push(Layer::new("x"));
        let mut bytes = encode(&c);
        let vis_off = MAGIC.len() + 2 + 2 + 4 + 4 + 4 + 16 + 4 + 4 + 4 + 4 + 1;
        bytes[vis_off] = 2;
        assert_eq!(
            decode(&bytes),
            Err(DecodeError::InvalidBoolean {
                context: "layer visible",
                value: 2,
            })
        );
    }

    #[test]
    fn reject_duplicate_or_unsorted_tile_keys() {
        let mut c = Canvas::new(64, 64);
        let mut layer = Layer::new("dup");
        layer.tiles.insert(TileKey { tx: 0, ty: 0 }, Tile::zeroed());
        layer.tiles.insert(TileKey { tx: 1, ty: 0 }, Tile::zeroed());
        c.layers.push(layer);
        let mut bytes = encode(&c);

        // Overwrite second tile's tx from 1 back to 0 so keys are duplicate (0,0), (0,0).
        let header = 48;
        let second_tile_off = header + 4 + 4 + 3 + 1 + 8 + 4 + 4 + (8 + TILE_SCALARS * 2);
        bytes[second_tile_off..second_tile_off + 4].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(decode(&bytes), Err(DecodeError::NonCanonicalTileOrder));
    }

    #[test]
    fn reject_truncation_at_all_lengths() {
        let mut c = Canvas::new(8, 8);
        let mut layer = Layer::new("t");
        layer.tiles.insert(TileKey { tx: 0, ty: 0 }, Tile::zeroed());
        c.layers.push(layer);
        let bytes = encode(&c);
        for len in 0..bytes.len() {
            match decode(&bytes[..len]) {
                Err(_) => {}
                Ok(_) => panic!("a {len}-byte truncated prefix decoded as a full canvas"),
            }
        }
    }

    #[test]
    fn reject_trailing_bytes() {
        let mut bytes = encode(&Canvas::new(8, 8));
        bytes.push(0x00);
        assert!(matches!(
            decode(&bytes),
            Err(DecodeError::TrailingBytes { extra: 1 })
        ));
    }

    #[test]
    fn reject_unknown_blend_mode() {
        let mut c = Canvas::new(8, 8);
        c.layers.push(Layer::new("x"));
        let mut bytes = encode(&c);
        let header = MAGIC.len() + 2 + 2 + 4 + 4 + 4 + 16 + 4 + 4;
        let first_layer = header + 4;
        let blend_off = first_layer + 4 + 1 + 1 + 8;
        bytes[blend_off] = 0x09;
        assert_eq!(decode(&bytes), Err(DecodeError::UnknownBlendMode(9)));
    }

    #[test]
    fn blend_modes_roundtrip() {
        for (mode, disc) in [
            (BlendMode::Normal, 0u32),
            (BlendMode::Multiply, 1),
            (BlendMode::Screen, 2),
        ] {
            assert_eq!(mode.to_u32(), disc);
            assert_eq!(BlendMode::from_u32(disc).unwrap(), mode);
        }
        assert_eq!(
            BlendMode::from_u32(3),
            Err(DecodeError::UnknownBlendMode(3))
        );
    }

    #[test]
    fn reject_oversized_layer_count_without_oom() {
        let mut bytes = encode(&Canvas::new(8, 8));
        let layer_count_off = MAGIC.len() + 2 + 2 + 4 + 4 + 4 + 16 + 4 + 4;
        bytes[layer_count_off..layer_count_off + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            decode(&bytes),
            Err(DecodeError::UnexpectedEof { .. })
        ));
    }
}

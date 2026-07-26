<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
---
title: Getting Started with paint.type
site: paint.type
description: Step-by-step guide to setting up and using paint.type
date: 2026-07-26
slug: getting-started
layout: default
brand: paint.type
tags: [getting-started, installation, tutorial, beginner]
---

# Getting Started with paint.type

> **From zero to verified painting in 10 minutes.**

This guide will walk you through setting up paint.type and creating your first
formally-verified digital artwork.

## Prerequisites

paint.type requires the following tools:

### Required

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | nightly | Core painting engine |
| Zig | 0.15.1 | FFI bridge and container builds |
| Idris2 | 0.8.0 | ABI proofs and type verification |

### Optional (for proof verification)

| Tool | Version | Purpose |
|------|---------|---------|
| Lean 4 | 4.13.0 | Undo graph and API type proofs |
| Agda | 2.6.4.3 | CRDT property proofs |
| Java | 25+ | TLA+ model checking |

## Installation

### 1. Clone the Repository

```bash
git clone https://github.com/metadatastician/paint-type
cd paint-type
```

### 2. Install Rust

```bash
# Using rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default nightly
rustup component add clippy rust-analysis rust-src
```

### 3. Install Zig

```bash
# Using zigup or from source
curl -sS https://webi.ms/zig | sh
# Or download from https://ziglang.org/download/
zig version  # Should show 0.15.1 or later
```

### 4. Install Idris2

```bash
# Using the estate setup script
./setup.sh  # This installs Idris2 0.8.0 and dependencies

# Or manually
idris2 --version  # Should show 0.8.0 or later
```

### 5. Build the Project

```bash
# Build the Rust core
cd src/paint_core
cargo build --release

# Build the Zig FFI
cd ../interface/ffi
zig build

# Return to root
cd ../../..
```

## Verifying the Build

Before running, verify that all proofs type-check:

```bash
# Check Idris2 proofs
idris2 --check src/interface/Abi/Types.idr
idris2 --check src/interface/Abi/Layout.idr
idris2 --check src/interface/Abi/Foreign.idr

# Check verification proofs (if Lean/Agda installed)
cd verification/proofs/idris2
idris2 --check ABI/Platform.idr ABI/Compliance.idr Pixel.idr TilePool.idr

cd ../../lean4
lean ApiTypes.lean UndoGraph.lean

cd ../agda
agda --no-libraries Compositing.agda TileCRDT.agda
```

If all commands succeed, your build is verified!

## Running paint.type

### Launch the Application

```bash
./paint-type-launcher.sh
```

This starts the paint.type application with the default configuration.

### Command-Line Options

```bash
# Show help
./paint-type-launcher.sh --help

# Start with a specific canvas size
./paint-type-launcher.sh --width 1920 --height 1080

# Enable verbose logging
./paint-type-launcher.sh --verbose

# Start in collaboration mode
./paint-type-launcher.sh --collab --peer-name my-peer
```

## Your First Painting

### 1. Create a New Canvas

- Click **File → New Canvas** or press `Ctrl+N`
- Select a size (default: 800×600)
- Choose a background color

### 2. Select a Brush

- Open the **Brush Panel** (B key)
- Choose from available brush tips:
  - **Soft Round**: Smooth, airbrush-like strokes
  - **Hard Round**: Sharp, precise edges
- Adjust size and opacity

### 3. Choose Colors

- Open the **Color Panel** (C key)
- Use the color wheel or RGB sliders
- Save favorite colors to your palette

### 4. Start Painting

- Click and drag to paint
- Use `[` and `]` to decrease/increase brush size
- Use `Shift+[` and `Shift+]` to adjust opacity
- Press `Z` to undo, `Ctrl+Z` for redo

## Using Layers

### Create a New Layer

- Click **Layer → New Layer** or press `Ctrl+Shift+N`
- Layers are created above the current layer
- Each layer has a stable ID that persists across reorderings

### Layer Operations

| Action | Shortcut | Description |
|--------|----------|-------------|
| New Layer | Ctrl+Shift+N | Create a new layer |
| Delete Layer | Ctrl+Shift+D | Delete current layer |
| Merge Down | Ctrl+E | Merge with layer below |
| Layer Up | Ctrl+] | Move layer up |
| Layer Down | Ctrl+[ | Move layer down |
| Toggle Visibility | Click eye icon | Show/hide layer |
| Adjust Opacity | Drag slider | Change layer opacity |

### Blend Modes

paint.type supports 11 compositing operators:

| Mode | Description | Use Case |
|------|-------------|----------|
| Normal | Default (over) | General painting |
| Multiply | Darkens base | Shadows, depth |
| Screen | Lightens base | Highlights, glows |
| Overlay | Multiply or Screen | Contrast enhancement |
| Darken | Minimum of base and blend | Darkening |
| Lighten | Maximum of base and blend | Lightening |
| Color Dodge | Brightens base | Vivid effects |
| Color Burn | Darkens base | Rich shadows |
| Hard Light | Strong contrast | Dramatic lighting |
| Soft Light | Subtle contrast | Gentle lighting |
| Lerp | Linear interpolation | Smooth transitions |

## Undo and History

paint.type has a **persistent undo graph** with formally-verified monotonicity:

- **No silent discard**: Every revision is preserved
- **History only grows**: Commits add, never remove
- **Acyclic ancestry**: No cycles in the undo tree
- **Bounded depth**: Reverts complete in ≤ r steps

### Undo Shortcuts

| Action | Shortcut | Description |
|--------|----------|-------------|
| Undo | Ctrl+Z | Revert last action |
| Redo | Ctrl+Y | Re-apply last undo |
| History Panel | Ctrl+Shift+H | View undo graph |
| Revert to Revision | Click in history | Jump to any point |

## Saving Your Work

### Save Canvas

- **File → Save** or `Ctrl+S`
- Saves to `paint-type` native format (.ptc)
- Preserves all layers, blend modes, and undo history

### Export Options

| Format | Extension | Description |
|--------|-----------|-------------|
| PNG | .png | Lossless, supports transparency |
| JPEG | .jpg | Lossy, smaller files |
| BMP | .bmp | Uncompressed bitmap |
| PPM | .ppm | Portable pixmap |

## Collaborative Painting

paint.type supports real-time collaboration via the **Burble protocol**.

### Starting a Session

1. **Peer 1**: Launch with collaboration enabled
   ```bash
   ./paint-type-launcher.sh --collab --peer-name peer1
   ```

2. **Peer 2**: Join the session
   ```bash
   ./paint-type-launcher.sh --collab --peer-name peer2 --join peer1
   ```

### Collaboration Features

- **Automatic discovery**: Peers find each other on the same LAN via Groove
- **CRDT merge**: Concurrent edits converge to the same final state
- **Permission model**: Fine-grained access control
- **Low latency**: <10ms p95 edit-stream latency

### Permission Levels

| Permission | Description |
|------------|-------------|
| Read | View canvas and layers |
| Paint | Apply brush strokes |
| Layer-Mutate | Create, delete, reorder layers |
| Invite | Add new peers to session |
| Kick | Remove peers from session |

## Advanced Features

### Brush Engine Configuration

The **Ephapax brush engine** supports:

- **Tip shapes**: Soft round, hard round, custom
- **Blend modes**: All 11 compositing operators
- **Mask modulation**: Pressure, velocity, randomness
- **Stroke interpolation**: Spacing carry-over for smooth strokes
- **Performance**: 88 ns/commit, 2 ns/checkout

### Type-Safe FFI

The Zig-Rust bridge provides:

- **Non-null pointer proofs** (ABI-1)
- **Memory layout correctness** (ABI-2)
- **Platform type size proofs** (ABI-3)
- **FFI function return type proofs** (ABI-4)
- **C ABI compliance** (ABI-5)

### Formal Verification

All critical invariants are proven:

- **Tile Pool**: No double-free, no use-after-free (INV-1)
- **Undo Graph**: Monotonicity, no silent discard (INV-2)
- **Compositing**: Blend function totality (INV-3)
- **CRDT Merge**: Commutativity, associativity (CONC-1/2/3)

## Troubleshooting

### Common Issues

**"Idris2 not found"**
: Run `./setup.sh` to install Idris2 and dependencies.

**"Zig version too old"**
: Update to Zig 0.15.1 or later.

**"Proof verification failed"**
: Ensure you have Lean 4.13.0 and Agda 2.6.4.3 installed.

**"Permission denied" on collaboration**
: Check that the permission model is correctly configured.

### Getting Help

- **GitHub Issues**: https://github.com/metadatastician/paint-type/issues
- **Documentation**: https://github.com/metadatastician/paint-type#readme
- **Estate Standards**: https://github.com/hyperpolymath/standards

## Next Steps

Now that you're up and running, explore:

- [Architecture Overview](architecture.html) — Deep dive into the system design
- [API Documentation](../docs/) — Complete API reference
- [Verification Status](../PROOF-STATUS.adoc) — Current proof completion status
- [Roadmap](../ROADMAP.adoc) — Upcoming features and milestones

---

[Back to Home](index.html) | [View on GitHub](https://github.com/metadatastician/paint-type)
<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
---
title: Architecture
site: paint.type
description: Deep dive into paint.type's type-safe, formally-verified architecture
date: 2026-07-26
slug: architecture
layout: default
brand: paint.type
tags: [architecture, design, type-safety, formal-verification, layers]
---

# paint.type Architecture

> **Types that breathe fire.** A deep dive into the architecture of a formally-verified
digital painting system.

paint.type is designed from the ground up with **type safety** and **formal verification**
as first-class concerns. This document describes the layered architecture that makes
this possible.

## High-Level Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                          USER INTERFACE LAYER                                │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────────┐  │
│  │ Web/WASM    │  │ Native GUI  │  │ CLI / TUI                     │  │
│  │ (Browser)   │  │ (Desktop)   │  │ (Terminal)                    │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         HOST CORE LAYER                                    │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
│  │  Session         │  │ Collaboration    │  │ Permission       │         │
│  │  Management      │  │ (Burble/CRDT)    │  │ Model            │         │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘         │
│  ┌─────────────────┐  ┌─────────────────┐                                  │
│  │  Transport       │  │ LLM Channel      │                                  │
│  │  (WebRTC/WS)    │  │ (Boj-Server MCP) │                                  │
│  └─────────────────┘  └─────────────────┘                                  │
└─────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        PAINT CORE LAYER                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────────────┐  │
│  │ Tile        │  │ Layer        │  │ Compositing                │  │
│  │ System      │  │ System       │  │ Engine                     │  │
│  └─────────────┘  └─────────────┘  └──────────────────────────┘      │
│  ┌─────────────┐  ┌─────────────┐  ┌──────────────────────────────┐  │
│  │ Brush       │  │ Undo Graph   │  │ Blend Modes                │  │
│  │ Engines     │  │ (Monotonic)  │  │ (11 operators)              │  │
│  └─────────────┘  └─────────────┘  └──────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      INTERFACE/FFI LAYER                                   │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────────────┐  │
│  │ Zig FFI Bridge  │  │ Rust Core        │  │ Idris2 Proofs           │  │
│  │ (libpt.so)      │  │ (paint_core)     │  │ (ABI verification)      │  │
│  └─────────────────┘  └─────────────────┘  └─────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────┘
                                      │
                                      ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                       VERIFICATION LAYER                                   │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐    │
│  │ Idris2      │  │ Lean 4       │  │ Agda         │  │ TLA+         │    │
│  │ (ABI, INV-1)│  │ (TP-2, INV-2)│  │ (INV-3,     │  │ (CONC-3,     │    │
│  │             │  │              │  │  CONC-1/2)  │  │  INV-2 re-  │    │
│  └─────────────┘  └─────────────┘  └─────────────┘  │  verification)│    │
│                                                      └─────────────┘    │
└─────────────────────────────────────────────────────────────────────────┘
```

## Layer 1: User Interface Layer

The UI layer provides multiple frontends for paint.type:

### Web/WASM Interface

- **Technology**: Typed-WASM bridge
- **Status**: Draft schemas compiled with `tw build`
- **Files**: `src/bridges/paint-type-{tile,layer}.twasm`
- **Features**:
  - Full paint.type surface exposed to browser
  - Typed WASM ensures memory safety
  - Zero-copy data passing where possible

### Native GUI (Planned)

- **Technology**: GTK, Qt, or native platform APIs
- **Status**: Design phase
- **Target**: Desktop platforms (Linux, Windows, macOS)

### CLI/TUI (Planned)

- **Technology**: Terminal-based interface
- **Status**: Conceptual
- **Use Case**: Scripting, automation, headless operation

## Layer 2: Host Core Layer

The host core manages sessions, collaboration, and permissions.

### Session Management

- **File**: `src/paint_collab/src/session.rs`
- **Features**:
  - Lamport clock for operation ordering
  - CRDT replica management
  - Pluggable transport
  - Local edit gating and broadcast

### Collaboration Engine

- **File**: `src/paint_collab/src/crdt.rs`
- **Technology**: CRDT (Conflict-free Replicated Data Type)
- **Design**:
  - Per-pixel last-writer-wins registers
  - Join-semilattice structure
  - Commutative, associative, idempotent merge
- **Guarantees**:
  - Strong Eventual Consistency (SEC)
  - Convergence regardless of message order
  - No silent discard of operations

#### CRDT Tile Merge

Each pixel cell carries:
- **Order key**: `(lamport: u64, peer: u64, value: u64)`
- **Merge operation**: Keep the cell with the greater key
- **Properties**:
  - **Commutativity** (CONC-1): A⊕B = B⊕A
  - **Associativity** (CONC-2): A⊕(B⊕C) = (A⊕B)⊕C
  - **Idempotence**: A⊕A = A

### Transport Layer

- **File**: `src/paint_collab/src/transport.rs`
- **Implementations**:
  - `SimTransport`: In-process testing
  - `BurbleTransport`: WebRTC via Burble (scaffold)
- **Guarantees**:
  - Order- and duplicate-insensitive by construction
  - Message delivery within 2s on LAN

### Discovery

- **File**: `src/paint_collab/src/groove.rs`
- **Technology**: Groove v1 manifest schema
- **Mechanism**: `.well-known/groove/` manifest
- **Features**:
  - mDNS discovery
  - HTTP .well-known fetch
  - No central broker required

### LLM Channel

- **File**: `src/paint_collab/src/llm.rs`
- **Backend**: boj-server MCP gateway
- **Status**: Off by default
- **Security**:
  - Assistant messages pass same permission gate
  - Never bypasses permission model
  - Full audit trail

### Permission Model

- **File**: `src/paint_collab/src/permission.rs`
- **Design**: Per-peer capability bitset
- **Permissions**:
  - `READ`: View canvas and layers
  - `PAINT`: Apply brush strokes
  - `LAYER_MUTATE`: Create/delete/reorder layers
  - `INVITE`: Add new peers
  - `KICK`: Remove peers
- **Guarantees**:
  - Every mutating action passes one gate
  - Denials return `CapabilityError`
  - Never silent success

## Layer 3: Paint Core Layer

The paint core contains the actual painting logic.

### Tile System

- **File**: `src/paint_core/src/tile.rs`
- **Design**: Fixed-size tile grid (4096 pixels per tile)
- **Pixel format**: RGBA16F (16-bit float per channel)
- **Operations**:
  - `pt_tile_write_pixel`: Write a single pixel
  - `pt_tile_blit`: Blit a rectangle
  - `pt_tile_fill`: Fill with a color
  - `pt_tile_composite_over`: Composite tiles

### Layer System

- **File**: `src/paint_core/src/layer.rs`
- **Features**:
  - Stable IDs across reorderings
  - `LayerStack` for managing multiple layers
  - `flatten_layer_stack` for final output
- **Operations**:
  - `pt_layer_create`: Create a new layer
  - `pt_layer_delete`: Delete a layer
  - `pt_layer_reorder`: Reorder layers
  - `pt_layer_set_opacity`: Set layer opacity
  - `pt_layer_set_blend_mode`: Set blend mode

### Compositing Engine

- **File**: `src/paint_core/src/composite.rs`
- **Blend modes** (11 total):
  - Porter-Duff: `over_premultiplied`, `over_unpremultiplied`
  - Standard: `multiply`, `screen`, `overlay`
  - Comparison: `darken`, `lighten`
  - Color: `color_dodge`, `color_burn`
  - Light: `hard_light`, `soft_light`
  - Interpolation: `lerp`
- **Verification**:
  - INV-3: Compositing blend function totality
  - Proved in Agda: `verification/proofs/agda/Compositing.agda`

### Brush Engines

#### Ephapax Engine

- **File**: `src/ephapax/src/brush.rs`
- **Features**:
  - `BrushTip`: Soft round, hard round
  - `Brush::stamp`: Mask-modulated blend
  - `Stroke`: Point interpolation with spacing carry-over
- **Performance**: 88 ns/commit, 2 ns/checkout

#### Groove Engine

- **File**: `src/ephapax/src/groove_brush.rs`
- **Status**: Experimental
- **Features**:
  - Pressure sensitivity
  - Tilt support
  - Custom tip shapes

### Undo Graph

- **File**: `src/paint_core/src/undo.rs`
- **Design**:
  - Append-only `Vec<Node<T>>`
  - `RevId`: Revision identifier (u32)
  - Parent pointers form a DAG
- **Invariants** (INV-2 - **PROVEN**):
  1. **Length monotonicity**: Grows by exactly 1 per commit
  2. **Old revisions survive**: Checkout is stable across commits
  3. **Parent edges immutable**: Point strictly to lower IDs
  4. **Ancestry acyclic**: Terminates in ≤ r steps
- **Proof**: `verification/proofs/lean4/UndoGraph.lean`
- **Re-verification**:
  - TLA+ `Monotone` property in `BurbleSession.tla`
  - Agda `⊔-upper-*` lemmas in `TileCRDT.agda`

## Layer 4: Interface/FFI Layer

The interface layer provides language interoperability.

### Zig FFI Bridge

- **File**: `src/interface/ffi/`
- **Components**:
  - `src/interface/ffi/src/main.zig`: Main FFI exports
  - `src/interface/ffi/build.zig`: Build configuration
  - `src/interface/ffi/contractile.just`: Justfile recipes
- **Exports**: 23 Zig exports (`pt_tile_*`, `pt_layer_*`, slot helpers)
- **Features**:
  - Opaque handles for Rust types
  - Result enum for error handling
  - Null/bad-magic/out-of-bounds validation

### Rust Core

- **File**: `src/paint_core/`
- **Package**: `paint_core` (Rust crate)
- **Features**:
  - `no_std` compatible where possible
  - `panic!` free in hot paths
  - Extensive documentation

### Idris2 ABI Proofs

- **Files**: `src/interface/Abi/{Types,Layout,Foreign}.idr`
- **Proofs**:
  - **ABI-1**: Non-null pointer proofs (`So (ptr /= 0)`)
  - **ABI-2**: Memory layout correctness (`HasSize`, `HasAlignment`)
  - **ABI-3**: Platform type size proofs (per platform)
  - **ABI-4**: FFI function return type proofs
  - **ABI-5**: C ABI compliance (`CABICompliant`, `FieldsAligned`)
- **Status**: ✅ All 5 proofs **DONE**

## Layer 5: Verification Layer

All formal proofs are located in `verification/proofs/`.

### Proof Categories

| Category | Count | Done | Tools | Location |
|----------|-------|------|-------|----------|
| ABI/FFI | 5 | 5 | Idris2 | `verification/proofs/idris2/ABI/` |
| Typing | 3 | 3 | Lean4 | `verification/proofs/lean4/` |
| Invariant | 3 | 3 | Idris2, Lean4, Agda | `verification/proofs/{idris2,lean4,agda}/` |
| Security | 2 | 0 | TLA+ | `verification/proofs/tlaplus/` |
| Concurrency | 3 | 3 | Agda, TLA+ | `verification/proofs/{agda,tlaplus}/` |
| **Total** | **16** | **14** | | |

### Proof Details

#### ABI/FFI Proofs (Idris2)

- **ABI-1**: `src/interface/Abi/Types.idr`
  - Non-null pointer proofs
  - Verification: `idris2 --check`

- **ABI-2**: `src/interface/Abi/Layout.idr`
  - Memory layout correctness
  - Verification: `idris2 --check`

- **ABI-3**: `verification/proofs/idris2/ABI/Platform.idr`
  - Platform type size proofs
  - Verification: `idris2 --check` + CI

- **ABI-4**: `src/interface/Abi/Foreign.idr`
  - FFI function return type proofs
  - Verification: `idris2 --check`

- **ABI-5**: `verification/proofs/idris2/ABI/Compliance.idr`
  - C ABI compliance
  - Verification: `idris2 --check` + CI

#### Typing Proofs (Lean4)

- **TP-2**: `verification/proofs/lean4/ApiTypes.lean`
  - Public API type safety
  - Verification: `lean` (requires Lean 4.13.0)

- **TP-3**: `verification/proofs/idris2/Pixel.idr`
  - RGBA16F pixel format bounds
  - No overflow, no NaN propagation

#### Invariant Proofs

- **INV-1**: `verification/proofs/idris2/TilePool.idr`
  - Tile pool invariant
  - No double-free, no use-after-free
  - Verification: `idris2 --check`

- **INV-2**: `verification/proofs/lean4/UndoGraph.lean`
  - Undo graph monotonicity
  - 4 clauses proven (see above)
  - Verification: `lean` (requires Lean 4.13.0)

- **INV-3**: `verification/proofs/agda/Compositing.agda`
  - Compositing blend function totality
  - Terminates on all inputs
  - Verification: `agda --no-libraries`

#### Concurrency Proofs

- **CONC-1, CONC-2**: `verification/proofs/agda/TileCRDT.agda`
  - CRDT tile merge commutativity and associativity
  - Join-semilattice laws
  - Verification: `agda --no-libraries`

- **CONC-3**: `verification/proofs/tlaplus/BurbleSession.tla`
  - Session liveness
  - Every committed mutation eventually visible
  - Verification: TLC model-check

#### Security Proofs (Remaining)

- **SEC-1**: `verification/proofs/tlaplus/PluginSandbox.tla`
  - Plugin WASM sandbox isolation
  - Status: Needed

- **SEC-2**: `verification/proofs/tlaplus/PluginApiSurface.tla`
  - Plugin API surface confinement
  - Status: Needed

## Cross-Cutting Concerns

### Memory Safety

- All Rust code uses safe Rust (no `unsafe`)
- Zig FFI validates all pointers
- Idris2 proofs ensure ABI correctness
- No undefined behavior by construction

### Type Safety

- Rust compiler enforces memory safety
- Idris2 dependent types model layouts precisely
- Lean4 proves API surface properties
- Agda proves algebraic structures

### Performance

- Rust core optimized for speed
- Zig FFI with zero-cost abstractions
- 88 ns/commit, 2 ns/checkout for undo
- Property tests with proptest (2000 cases each)

### Testing

- **Unit tests**: `cargo test` (Rust)
- **Aspect tests**: `bash tests/aspect_tests.sh`
- **E2E tests**: `bash tests/e2e.sh` (9 stages)
- **Fuzz tests**: `cargo fuzz` (3 targets)
- **Property tests**: `proptest` (5 convergence properties)
- **Proof tests**: CI enforces all proofs type-check

## Verification Status

### Completed Proofs (14/16)

| ID | Proof | Tool | File | Date |
|----|-------|------|------|------|
| ABI-1 | Non-null pointer proofs | Idris2 | `src/interface/Abi/Types.idr` | 2026-05-11 |
| ABI-2 | Memory layout correctness | Idris2 | `src/interface/Abi/Layout.idr` | 2026-05-11 |
| ABI-3 | Platform type size proofs | Idris2 | `verification/proofs/idris2/ABI/Platform.idr` | 2026-06-01 |
| ABI-4 | FFI function return type proofs | Idris2 | `src/interface/Abi/Foreign.idr` | 2026-05-11 |
| ABI-5 | C ABI compliance | Idris2 | `verification/proofs/idris2/ABI/Compliance.idr` | 2026-06-01 |
| TP-2 | Public API type safety | Lean4 | `verification/proofs/lean4/ApiTypes.lean` | 2026-06-14 |
| TP-3 | RGBA16F pixel bounds | Idris2 | `verification/proofs/idris2/Pixel.idr` | 2026-06-01 |
| INV-1 | Tile pool invariant | Idris2 | `verification/proofs/idris2/TilePool.idr` | 2026-06-14 |
| INV-2 | Undo graph monotonicity | Lean4 | `verification/proofs/lean4/UndoGraph.lean` | 2026-06-14 |
| INV-3 | Compositing totality | Agda | `verification/proofs/agda/Compositing.agda` | 2026-06-14 |
| CONC-1 | CRDT merge commutativity | Agda | `verification/proofs/agda/TileCRDT.agda` | 2026-07-25 |
| CONC-2 | CRDT merge associativity | Agda | `verification/proofs/agda/TileCRDT.agda` | 2026-07-25 |
| CONC-3 | Session liveness | TLA+ | `verification/proofs/tlaplus/BurbleSession.tla` | 2026-07-25 |

### Remaining Proofs (2/16)

| ID | Proof | Tool | File | Status |
|----|-------|------|------|--------|
| SEC-1 | Plugin WASM sandbox isolation | TLA+ | `verification/proofs/tlaplus/PluginSandbox.tla` | Not started |
| SEC-2 | Plugin API surface confinement | TLA+ | `verification/proofs/tlaplus/PluginApiSurface.tla` | Not started |

Both SEC proofs unblocked by v0.4.0 plugin system completion.

## License

Code: AGPL-3.0-or-later  
Documentation: CC-BY-SA-4.0

---

[Back to Home](index.html) | [Getting Started](getting-started.html) | [View on GitHub](https://github.com/metadatastician/paint-type)
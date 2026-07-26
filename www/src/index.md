<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
---
title: paint.type
site: paint.type
description: A cross-language, type-safe digital painting system with formally verified invariants
date: 2026-07-26
slug: index
layout: default
brand: paint.type
tags: [painting, digital-art, type-safety, formal-verification, rust, zig, wasm]
---

# paint.type

> **Types that paint.** A cross-language, type-safe digital painting system with formally
> verified invariants, built for creators who demand correctness.

paint.type is a next-generation digital painting application that combines the expressiveness
of digital art with the rigor of formal verification. Every brush stroke, every blend mode,
every undo operation is backed by machine-checked proofs.

## Feature Highlights

### ✅ Type-Safe Core

- **Rust** backend with Idris2-verified ABI bridge
- **Zig** FFI layer with memory-safety guarantees
- **WASM** bindings for web integration
- Zero undefined behavior by construction

### 🎨 Professional Painting Tools

- Multiple brush engines (Ephapax, Groove)
- 11 compositing operators (over, lerp, multiply, screen, in, out, atop, xor, etc.)
- Layer system with stable IDs across reorderings
- Persistent undo graph with monotonicity guarantees

### 🔒 Formal Verification

- **14/16 proofs complete** (88% proven)
- ABI/FFI boundary proofs (Idris2)
- Typing proofs (Lean4)
- Invariant proofs (Idris2, Lean4, Agda)
- Concurrency proofs (Agda, TLA+)
- CRDT merge properties verified

### 🤖 Collaborative Features

- Burble-based WebRTC collaboration
- CRDT tile merge (commutative, associative, idempotent)
- Permission model (read, paint, layer-mutate, invite, kick)
- Session liveness guarantees

## Verification Status

| Category | Total | Done | In Progress | Remaining |
|----------|-------|------|-------------|-----------|
| ABI/FFI | 5 | 5 | 0 | 0 |
| Typing | 3 | 3 | 0 | 0 |
| Invariant | 3 | 3 | 0 | 0 |
| Security | 2 | 0 | 0 | 2 |
| Concurrency | 3 | 3 | 0 | 0 |
| **Total** | **16** | **14** | **0** | **2** |

**Overall: 88% proven** — ABI, Typing, Invariant, and Concurrency categories fully closed.

## Screenshots

![Brush Demo](images/brush_demo.png "Ephapax brush engine in action")

The Ephapax brush engine features:
- Soft and hard round brush tips
- Mask-modulated blending
- Stroke point interpolation with spacing carry-over
- 88 ns/commit, 2 ns/checkout performance

![Composite Demo](images/composite_demo.png "Composite operations showcase")

Composite operations include:
- Porter-Duff over (premultiplied and unpremultiplied)
- Masked blend
- Layer stack flattening
- Tile compositing

## Quick Start

### Prerequisites

- Rust (nightly recommended)
- Zig 0.15.1+
- Idris2 0.8.0+
- Lean 4.13.0 (for proofs)
- Agda 2.6.4.3 (for CRDT proofs)

### Building

```bash
# Clone the repository
git clone https://github.com/metadatastician/paint-type
cd paint-type

# Build the Rust core
cd src/paint_core
cargo build --release

# Build the Zig FFI
cd src/interface/ffi
zig build

# Run the type checker on proofs
idris2 --check src/interface/Abi/Types.idr
lean verification/proofs/lean4/UndoGraph.lean
agda --no-libraries verification/proofs/agda/TileCRDT.agda
```

### Running

```bash
# Launch the application
./paint-type-launcher.sh

# Or run tests
cargo test
bash tests/aspect_tests.sh
```

## Architecture

paint.type is organized into several layers:

```
┌─────────────────────────────────────────────────────────────┐
│                    User Interface Layer                      │
│  (Web/WASM, Native GUI, CLI)                                │
├─────────────────────────────────────────────────────────────┤
│                   Host Core Layer                           │
│  (Session management, collaboration, permissions)            │
├─────────────────────────────────────────────────────────────┤
│                  Paint Core Layer                            │
│  (Tile operations, compositing, brush engines)              │
├─────────────────────────────────────────────────────────────┤
│                 Interface/FFI Layer                          │
│  (Zig, Rust, Idris2 ABI bridge)                              │
├─────────────────────────────────────────────────────────────┤
│                 Verification Layer                           │
│  (Formal proofs in Idris2, Lean4, Agda, TLA+)                │
└─────────────────────────────────────────────────────────────┘
```

## Collaboration

paint.type supports real-time collaborative painting via the Burble protocol:

- Two-peer sessions establish via Burble + Groove discovery within 2s on LAN
- Sustained edit-stream latency stays <10ms p95
- CRDT tile merge ensures convergence regardless of message order
- Permission model gates all actions per peer

## Verification

All critical invariants are formally verified:

- **Undo Graph Monotonicity (INV-2)**: History only grows, no silent discard
- **CRDT Merge (CONC-1/2/3)**: Commutative, associative, convergent
- **Tile Pool (INV-1)**: No double-free, no use-after-free
- **API Type Safety (TP-2)**: Public API surface is type-safe
- **ABI Compliance (ABI-1/2/3/4/5)**: Memory layout, pointer safety, platform compat

## License

Code: AGPL-3.0-or-later  
Documentation: CC-BY-SA-4.0

See [LICENSE](../LICENSE) and [LICENSES](../LICENSES/) for details.

---

[GitHub Repository](https://github.com/metadatastician/paint-type) |
[Documentation](../README.adoc) |
[Proof Status](../PROOF-STATUS.adoc) |
[Roadmap](../ROADMAP.adoc)
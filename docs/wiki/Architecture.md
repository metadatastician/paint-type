# paint.type Architecture

**Source of Truth:** This document provides architectural overview. For detailed component information, see in-repo documentation.
**Last Updated:** 2026-07-26
**Maintainer:** hyperpolymath / Joshua Jewell

---

## Overview

paint.type is a **multi-tier, multi-language** image editor designed with **provable safety** and **cross-platform compatibility** as first-class concerns. The architecture separates concerns across language boundaries, with each tier picking the language that gives it the required safety property.

## Tier Architecture

### Tier 1: Desktop Shell (Gossamer)

**Purpose:** Platform-abstracted webview hosting with linear capability safety.

**Technology Stack:**
- **Primary:** Rust + Zig (FFI)
- **Backends:** WebKitGTK (Linux), WKWebView (macOS), WebView2 (Windows)
- **Safety Property:** Linear capability tokens, provable resource safety

**Components:**
- Shell window management
- Webview hosting
- Typed IPC protocol
- Linear capability system
- Cross-platform abstraction

**Ephapax Integration:**
- Shell.eph — Linear webview handles
- Bridge.eph — Typed IPC
- Capabilities.eph — Linear capability tokens

**Status:** ✅ **DONE** (DEP-04) — v0.3.1 released, integrated into paint-type

---

### Tier 2: Web UI

**Purpose:** User interface chrome hosted in Gossamer webview.

**Technology Stack:**
- **Primary:** HTML5 + CSS + JavaScript (TypeScript planned)
- **Host:** Served by Gossamer webview
- **Safety Property:** Sandboxed execution, CSP-locked

**Components:**
- Layer panel
- Tool bar
- Canvas viewport
- Inspector panels
- Plugin UI
- Settings dialog

**AffineScript Integration (Planned):**
- UI chrome compilation via AffineScript → typed-wasm
- **Status:** ❌ **BLOCKED** (DEP-03 — no working AffineScript compiler)

**Current Status:** Web UI chrome components implemented and working

---

### Tier 3: Bridge Language (AffineScript)

**Purpose:** Algebraic effects for IPC traffic classes, separating latency-critical commands from throughput-oriented data delivery.

**Technology Stack:**
- **Language:** AffineScript (OCaml-based, planned)
- **Target:** typed-wasm
- **Safety Property:** Algebraic effects separate concerns, multi-module type safety

**Key Design:**
- Latency-critical brush commands separated from throughput-oriented tile delivery
- Multi-module type safety across IPC boundary
- Shared region schemas verified at compile time

**Status:** ❌ **BLOCKED** (DEP-03) — No working compiler available

**ABI→.twasm Generator:** ✅ **DONE** (paint-type#39) — Schemas available:
- `src/bridges/paint-type-tile.twasm`
- `src/bridges/paint-type-layer.twasm`

---

### Tier 4: Cross-Language Boundary (typed-wasm)

**Purpose:** Multi-module type safety at the AffineScript/Ephapax boundary.

**Technology Stack:**
- **Primary:** typed-wasm (Rust-based verification)
- **Schema Format:** .twasm (Typed WASM)
- **Safety Property:** Shared region schemas verified at compile time, no bitmap copies

**Key Features:**
- Type verification of WASM modules
- Schema compilation
- Integration with paint-type build system

**Status:** ✅ **DONE** (DEP-02) — All schemas verified, paint-type slice closed

---

### Tier 5: Native Image Core (Ephapax)

**Purpose:** Linear types with region-based allocation for image processing.

**Technology Stack:**
- **Primary:** Rust (implementation)
- **FFI:** Zig (C ABI compatibility)
- **Proofs:** Coq, Idris2 (formal verification)
- **Safety Property:** Linear types (`!Copy + !Clone` Tile), no aliased mutable access

**Components:**

#### Tile System
- RGBA16F format
- 64×64 tiles
- Region-based ownership
- Compositing pipeline

#### Compositing Primitives (All DONE)
1. Porter-Duff `over`
2. Masked blend
3. Flatten layer stack
4. Non-uniform composite (tile-level)
5. Brush-stroke / kernel sampling
6. lerp (linear interpolation)
7. multiply
8. screen
9. in_op
10. out_op
11. atop
12. xor

**Total:** 12 compositing operators implemented

#### Brush Engine (All DONE)
- BrushTip (soft/hard round)
- Brush::stamp with mask-modulated blend
- Stroke point interpolation with spacing carryover

#### Undo System
- Persistent UndoGraph
- O(1) branch complexity
- Non-destructive editing

#### Layer Model
- Create, delete, flatten
- Reorder
- Non-destructive operations

#### Codecs
- PNG (read/write)
- RGBA16F (native format)

**V1 → V2 Redesign Status:**
- **L1 (Region capabilities):** Active — 54 Qed / 2 Admitted
- **L2 (Structural modality):** Complete — 1 Qed / 0 Admitted
- **L3 (Echo/residue calculus):** Complete — 12 Qed / 0 Admitted
- **L4 (Dyadic interaction):** Scaffold — 0 Qed / 0 Admitted

**Open Research Questions (2):**
1. `formal/Semantics_L1.v:3318` — `step_pop_disjoint_from_type_l1`
2. `formal/Semantics_L1.v:3337` — `preservation_l1` (gated on step_pop)

**Research Blocker:** L1 Eliminator Fork — requires choreographic typing experiment

**Status:** In Progress (DEP-01) — V2 redesign active, foundation solid

---

### Tier 6: ABI Definitions (Idris2)

**Purpose:** Dependent types prove pointer non-nullability, layout correctness, ABI compliance.

**Technology Stack:**
- **Language:** Idris2
- **Build:** RefC → C → Zig FFI
- **Safety Property:** Dependent type proofs for memory safety

**Components:**
- `src/interface/Abi/Types.idr` — Type definitions
- `src/interface/Abi/Layout.idr` — Layout proofs
- `src/interface/Abi/Foreign.idr` — FFI correctness

**ABI Surface:** 17 files (per PROOF-NEEDS.md)
- Clean of `believe_me` / `sorry` / `postulate`
- Working ABI definitions

**Status:** ✅ **DONE** (part of DEP-10 proven) — Consumable as Idris2 proof foundation

---

### Tier 7: Verification Layer

**Purpose:** Formal proofs of safety properties.

**Technology Stack:**
- **Coq:** Region calculus proofs
- **Idris2:** ABI and layout proofs
- **Agda:** Estate-wide proof discipline

**Components:**

#### Ephapax Formal Proofs
- `formal/Semantics.v` — V1 preservation theorem (provably false, sacrosanct)
- `formal/Counterexample.v` — 5 Qed lemmas proving V1 unsoundness
- `formal/Semantics_L1.v` — L1 semantics with 2 open admits
- `formal/PRESERVATION-DESIGN.md` — Preservation design doctrine
- `formal/L1-ELIMINATOR-FORK.md` — Research plan for L1 admits

#### proven (Idris2 Verified-Foundations)
- `src/abi/Ephapax/…` — ABI definitions and proofs
- `src/formal/Ephapax/…` — Region linearity, narrow no-escape proof
- No `believe_me` / `sorry` / `assert_total`
- Build pipeline: Idris2 → RefC → C → Zig FFI

**Status:**
- DEP-10 proven: ✅ **DONE**
- Track-C security triage: ✅ Closed via PR #162

---

## Seam Architecture

paint.type has **four named seams** with typed contracts:

### Seam 1: Internal — paint_core ↔ libpt

**Contract:** Rust `extern "C"` declarations in `src/paint_core/src/lib.rs` mirror Zig `pub export fn` in `src/interface/ffi/src/main.zig`.

**Verification:**
- ABI-compatibility proven in `src/interface/Abi/Foreign.idr`
- Size-checked
- Layout-proven
- panic-attack-scanned

---

### Seam 2: Internal — host ↔ paint_core

**Contract:** Pure Rust calls through `host_core::dispatch::dispatch`; shared `Document` behind `Arc<Mutex<_>>` for the Gossamer command thread.

**Verification:**
- panic-attack-scanned
- Size-checked
- Layout-proven

---

### Seam 3: External — Gossamer shell ↔ webview

**Contract:** JSON-encoded `Command` / `Response` via `window.__gossamer_invoke("dispatch", …)`. CSP-locked to own origin + inline UI script.

**Verification:**
- Protocol-versioned
- Version-guarded
- Recovery path: DUST-ABI-0001 for libpt ABI mismatch

---

### Seam 4: External — paint-type ↔ Burble / Groove

**Contract:** Service discovery via `.well-known/groove/manifest.json`; collaboration via Burble WebRTC (CRDT tile mutations).

**Verification:**
- Protocol-versioned
- Version-guarded

---

## Language + Seam Compliance Policy

paint.type adheres to a **strict per-tier language policy**:

| Tier | Language | Safety Property | Purpose |
|------|----------|----------------|---------|
| Application + image core | Rust | Linear types via `!Copy + !Clone` Tile | No aliased mutable access |
| FFI bridge | Zig (FFI directory only) | C ABI compatibility | No C foot-guns |
| ABI definitions + proofs | Idris2 | Dependent types | Prove pointer non-nullability, layout correctness, ABI compliance |
| UI bridge schemas | AffineScript → typed-wasm | Algebraic effects | Separate latency-critical from throughput-oriented |
| Build orchestration | `just` + bash | Recipe-based | No Python, no `make` |
| Container runtime | podman + Chainguard Wolfi | Rootless containers | ML-DSA-87 signed |

---

## Collaboration Architecture (v0.5+)

### Burble Integration

**Purpose:** Sub-10ms WebRTC voice and data-channel for real-time collaboration.

**Components:**
- WebRTC session layer
- Tile-edit traffic over data-channel
- Session presence model
- LLM channel participant

**Status:** Todo (DEP-06) — Voice-first today, needs data-channel proof

---

### Groove Protocol Integration

**Purpose:** Zero-config peer discovery.

**Components:**
- `.well-known/groove/manifest.json`
- Automatic localhost service discovery
- paint-type and Burble find each other automatically
- No configuration, no account, no setup

**Status:** Todo (DEP-07) — 8/10 FFI binding targets untested

---

### boj-server Integration

**Purpose:** MCP LLM channel + plugin package index.

**Components:**
- MCP (Model Context Protocol) channel
- LLM channel for session context
- Plugin package index
- Stable cartridge surface for paint-type LLM channel

**LLM Channel Behavior:**
- Permission-gated, off-by-default
- Holds context, mediates conflicts
- Answers when addressed
- Does not touch canvas unless explicitly asked
- Does not offer unsolicited opinions

**Status:** Todo (DEP-08)

---

## Plugin System (v0.4 — DONE)

### Plugin Tiers

| Tier | Technology | Purpose | Safety |
|------|------------|---------|--------|
| WASM | typed-wasm | Cross-platform plugins | Sandboxed |
| Native | Rust + Zig FFI | Maximum performance | Isolated |

### Plugin APIs

#### Effect Plugin API
- Colour adjustments
- Filters
- Image operations

#### Tool Plugin API
- Custom brush behaviours
- Input handling
- Canvas interaction

### Plugin Infrastructure
- WASM tier sandbox (`src/plugins/sandbox.rs`)
- Plugin manifest and signing (cerro-torre, ML-DSA)
- Plugin browser and package index
- Effect plugin API (`src/plugins/effect.rs`)
- Tool plugin API (`src/plugins/tool.rs`)

**Status:** ✅ **DONE** (v0.4) — All plugin system features closed

---

## File Format

### Native Format
- **Primary:** RGBA16F
- **Tile Size:** 64×64
- **Structure:** Tiled, region-based

### Supported Formats
- PNG (read/write)
- RGBA16F (native)
- Future: JPEG, WebP, GIF, TIFF (planned)

---

## Build System

### just Recipes

```bash
# Full build
just build

# Rust build
just build-rust

# Zig FFI
just build-ffi

# Idris2 type checking
just typecheck
just check-core
just check-dev

# Verification
just verify-totality
just verify-ffi

# Clean
just clean

# Registry and scorecards (from standards)
just registry
just scorecards
just verify-claims
```

### CI/CD Pipeline

| Workflow | Purpose | Status |
|----------|---------|--------|
| rust.yml | Rust CI | ✅ Active |
| idris-ci.yml | Idris2 CI | ✅ Active |
| e2e.yml | End-to-end tests | ✅ Active |
| coverage.yml | Coverage reporting | ✅ Active |
| codeql.yml | CodeQL analysis | ✅ Active |
| scorecard.yml | OpenSSF Scorecard | ✅ Active |
| fuzz.yml | Fuzz testing | ✅ Active |
| panic-attack.yml | Dangerous pattern detection | ✅ **2026-07-26** |
| status-gate.yml | Proof/test count gates | ✅ Active |
| abi-verify.yml | ABI verification | ✅ Active |
| ffi-seams.yml | FFI seam verification | ✅ Active |
| coq-build.yml | Coq formal proofs | ✅ Active |

---

## Platform Support

| Platform | Backend | Status |
|----------|---------|--------|
| Linux | WebKitGTK | ✅ Supported |
| macOS | WKWebView | ✅ Supported |
| Windows | WebView2 | ✅ Supported |
| Android | NDK (planned) | ⚠️ Upstream blocked |

---

## Security Architecture

### Capability System
- Linear capability tokens (Gossamer)
- Permission-based gating (collaboration)
- Read/Paint/LayerMutate/Invite/Kick permissions

### Signing
- ML-DSA-87 (via cerro-torre)
- Plugin package signing
- Container image signing

### Sandboxing
- WASM plugin sandbox
- CSP-locked webview
- Typed IPC boundaries

---

## Verification Commands

```bash
# Full build
just build

# Type checking
git grep -l "sorry\|believe_me\|postulate" src/
idris2 --check --total paint-type.ipkg

# Test all
cargo test
just test

# FFI verification
just verify-ffi

# Proof count verification (Ephapax)
./scripts/status-gate.sh --proofs

# panic-attack scan (Ephapax)
panic-attack assail --config panic-attack.toml

# Admitted proof check (Ephapax)
grep -R --include='*.v' -n '^[[:space:]]*Admitted\.' formal
```

---

## References

- [README.adoc](https://github.com/metadatastician/paint-type/blob/main/README.adoc) — Project overview
- [OPERATIONAL-STATUS.adoc](https://github.com/metadatastician/paint-type/blob/main/OPERATIONAL-STATUS.adoc) — Component topology
- [ARCHITECTURE.md](https://github.com/metadatastician/paint-type/blob/main/ARCHITECTURE.md) — Architecture notes
- [DEPENDENCY-SCHEDULER.adoc](https://github.com/metadatastician/paint-type/blob/main/DEPENDENCY-SCHEDULER.adoc) — Dependency tracking
- [DEP-IMPLEMENTATION-STATUS.adoc](https://github.com/metadatastician/paint-type/blob/main/DEP-IMPLEMENTATION-STATUS.adoc) — DEP status details
- [DEP-AUTOMATION-TRACKING.adoc](https://github.com/metadatastician/paint-type/blob/main/DEP-AUTOMATION-TRACKING.adoc) — Automation coverage

---

*This document was last updated on 2026-07-26. For the most current information, refer to the in-repo documentation.*

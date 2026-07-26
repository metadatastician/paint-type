# paint.type Development Guide

**Source of Truth:** This guide summarizes development practices. For detailed information, see in-repo documentation.
**Last Updated:** 2026-07-26
**Maintainer:** hyperpolymath / Joshua Jewell

---

## Quick Start

### Prerequisites

| Tool | Version | Purpose | Installation |
|------|---------|---------|-------------|
| Git | Latest | Version control | System package manager |
| Rust | Latest stable | Image core, plugins | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Zig | Latest stable | FFI bridge | [ziglang.org/download](https://ziglang.org/download/) |
| Idris2 | 0.8.0+ | ABI definitions, proofs | `pack install idris2` or [idris-lang.org](https://www.idris-lang.org/) |
| Node.js | 18+ | WASM tooling | [nodejs.org](https://nodejs.org/) |
| just | 1.0+ | Recipe runner | `cargo install just` or [casey/just](https://github.com/casey/just) |
| podman | Latest | Container runtime | System package manager |

### Setup

```bash
# Clone the repository
git clone https://github.com/metadatastician/paint-type.git
cd paint-type

# Install dependencies (Debian/Ubuntu example)
sudo apt-get install build-essential git curl
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh
source $HOME/.cargo/env
cargo install just

# Build the project
just build
```

### Build Recipes

```bash
# Full build (Rust + Zig FFI)
just build

# Rust only
just build-rust

# Zig FFI only
just build-ffi

# Idris2 type checking (full)
just typecheck

# Idris2 type checking (core only)
just check-core

# Idris2 type checking (dev, no totality)
just check-dev

# FFI verification
just verify-ffi

# Totality verification
just verify-totality

# Clean build artifacts
just clean

# Full clean (including dependencies)
just clean-all
```

---

## Development Environment

### IDE Setup

#### VS Code (Recommended)

**Extensions:**
- rust-analyzer (Rust language support)
- Zig Language (Zig support)
- Idris2 LSP (Idris2 support)
- AsciiDoc (documentation)
- GitLens (Git integration)

**Workspace Configuration:**
```json
{
  "rust-analyzer.server.extraEnv": {
    "RUSTFLAGS": "-D warnings"
  },
  "[rust]": {
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "[zig]": {
    "editor.defaultFormatter": "zls.zls"
  }
}
```

#### JetBrains (IntelliJ/RustRover)

- Rust plugin
- Zig plugin (if available)
- Markdown/AsciiDoc support

### Dev Containers

A `.devcontainer/` directory is provided for VS Code Dev Containers and GitHub Codespaces.

**Features:**
- Pre-configured with all dependencies
- Rust, Zig, Idris2 pre-installed
- just, cargo, pack available
- Pre-configured extensions

**Usage:**
1. Open in VS Code
2. Click "Reopen in Container"
3. Wait for build to complete
4. Run `just build`

---

## Code Organization

### Directory Structure

```
paint-type/
├── src/                          # Main source code
│   ├── paint_core/               # Image core (Rust)
│   │   ├── src/                  # Rust implementation
│   │   │   └── lib.rs            # Main library
│   │   ├── tests/                # Unit tests
│   │   └── README.adoc           # Component documentation
│   │
│   ├── host_core/               # Host integration (Rust)
│   │   ├── src/
│   │   └── tests/
│   │
│   ├── interface/               # FFI interface
│   │   ├── Abi/                  # Idris2 ABI definitions
│   │   │   ├── Types.idr
│   │   │   ├── Layout.idr
│   │   │   └── Foreign.idr
│   │   └── ffi/                  # Zig FFI implementation
│   │       └── src/
│   │           └── main.zig
│   │
│   ├── ephapax/                 # Legacy Ephapax (Rust)
│   ├── affinescript/            # AffineScript bridge
│   ├── wasm/                    # WASM modules
│   └── bridges/                 # Typed WASM schemas
│       ├── paint-type-tile.twasm
│       └── paint-type-layer.twasm
│
├── tests/                       # Integration tests
│   ├── rust/                    # Rust integration tests
│   └── e2e/                     # End-to-end tests
│
├── docs/                        # Documentation
│   ├── wiki/                    # Wiki pages
│   ├── architecture/            # Architecture docs
│   ├── decisions/               # ADRs (Architecture Decision Records)
│   └── ...
│
├── .github/                     # GitHub configuration
│   ├── workflows/               # CI/CD workflows
│   │   ├── rust.yml             # Rust CI
│   │   ├── idris-ci.yml         # Idris2 CI
│   │   ├── e2e.yml              # E2E tests
│   │   ├── coverage.yml         # Coverage reporting
│   │   ├── codeql.yml           # CodeQL analysis
│   │   ├── scorecard.yml        # OpenSSF Scorecard
│   │   ├── fuzz.yml             # Fuzz testing
│   │   └── ...
│   └── ISSUES/                  # DEP issue tracking
│       ├── DEP-01-EPHAPAX.adoc
│       ├── DEP-02-TYPED-WASM.adoc
│       └── ...
│
├── scripts/                     # Utility scripts
│   └── status-gate.sh           # Proof/test count verification
│
├── features/                    # Feature flags
│
├── third_party/                 # Third-party dependencies
│
├── .machine_readable/           # Machine-readable metadata
│
├── Justfile                     # Build recipes
├── build.zig                    # Zig build configuration
├── README.adoc                  # Project overview
├── ROADMAP.adoc                 # Milestone plan
└── DEPENDENCY-SCHEDULER.adoc    # DEP tracking
```

### Key Components

#### paint_core (Rust)

**Purpose:** Native image core with linear type safety.

**Key Types:**
- `Tile` — RGBA16F 64×64 tile with linear ownership (`!Copy + !Clone`)
- `Layer` — Collection of tiles
- `Document` — Collection of layers with undo graph
- `BrushTip` — Brush shape definition
- `Brush` — Brush with stamp capability
- `Stroke` — Stroke with point interpolation

**Key Modules:**
- `tile.rs` — Tile operations
- `layer.rs` — Layer operations
- `composite.rs` — Compositing primitives
- `brush.rs` — Brush engine
- `undo.rs` — Undo graph
- `codec.rs` — PNG codec

#### host_core (Rust)

**Purpose:** Host integration between Gossamer shell and paint_core.

**Key Types:**
- `Document` — Shared document state
- `Command` — IPC commands
- `Response` — IPC responses

**Key Modules:**
- `dispatch.rs` — Command dispatch
- `document.rs` — Document management

#### interface/Abi (Idris2)

**Purpose:** ABI definitions with dependent type proofs.

**Key Files:**
- `Types.idr` — Type definitions
- `Layout.idr` — Layout proofs (pointer non-nullability, alignment)
- `Foreign.idr` — FFI correctness proofs

**ABI Surface:** 17 files proving:
- Pointer non-nullability
- Layout correctness
- ABI compliance

#### interface/ffi (Zig)

**Purpose:** C ABI compatible FFI implementation.

**Key Features:**
- `pub export fn` for C ABI compatibility
- No C foot-guns (no undefined behavior)
- Memory safety via Zig's allocator system

---

## Language-Specific Development

### Rust Development

#### Coding Standards

```rust
// Good: Linear types for Tile
pub struct Tile {
    data: Box<[u8; TILE_SIZE * TILE_SIZE * 8]>, // RGBA16F = 8 bytes per pixel
    region: Region,
}

// Prevent copying
impl !Copy for Tile {}
impl !Clone for Tile {}

// Good: Explicit error handling
pub fn load_png(path: &Path) -> Result<Document, PngError> {
    // Implementation
}

// Bad: unwrap
let doc = load_png(path).unwrap(); // Don't do this

// Good: pattern matching
match load_png(path) {
    Ok(doc) => { /* use doc */ }
    Err(e) => { /* handle error */ }
}
```

#### Testing

```bash
# Run all Rust tests
cargo test

# Run with coverage
cargo llvm-cov

# Run specific test
cargo test test_name

# Run doctests
cargo test --doc
```

#### Fuzzing

```bash
# Install cargo fuzz
cargo install cargo-fuzz

# Run fuzzer
cargo fuzz run fuzz_target

# Check fuzz targets
cargo fuzz list
```

### Zig Development

#### Coding Standards

```zig
// Good: Explicit allocators
pub fn createTile(allocator: std.mem.Allocator) !Tile {
    const tile = try allocator.create(Tile);
    // Initialize tile
    return tile;
}

// Good: Error unions
pub fn loadFile(path: []const u8) ![]u8 {
    const file = try std.fs.cwd().readFileAllocate(path, 1024, 1024);
    defer allocator.free(file);
    return file;
}

// Bad: expect (use try or catch)
const file = std.fs.cwd().readFileAllocate(path, 1024, 1024) catch unreachable;
```

#### FFI

```zig
// Export for C ABI
pub export fn pt_add(a: c_int, b: c_int) c_int {
    return a + b;
}

// Import from C
const libc = @cImport({
    @cInclude("stdlib.h");
});

pub fn free_ptr(ptr: *anyopaque) void {
    libc.free(ptr);
}
```

### Idris2 Development

#### ABI Definitions

```idris
-- Types.idr
namespace Abi

public export
record TileLayout where
  constructor MkTileLayout
  width  : Nat
  height : Nat
  stride : Nat

public export
record Region where
  constructor MkRegion
  base   : Ptr
  extent : TileLayout
```

#### Layout Proofs

```idris
-- Layout.idr
namespace Abi

public export
layoutCorrect : (layout : TileLayout) -> Type
layoutCorrect layout = 
  width layout = 64 /\(height layout) = 64
```

#### Type Checking

```bash
# Type check full library
idris2 --check --total paint-type.ipkg

# Type check specific file
idris2 --check src/interface/Abi/Types.idr

# Dev build (no totality, faster)
just check-dev
```

---

## Testing Strategy

### Test Pyramid

```
          ┌─────────────┐
          │   E2E Tests  │  ← 5% of tests, high value
          └─────────────┘
          ┌─────────────┐
          │ Integration │  ← 15% of tests
          └─────────────┘
          ┌─────────────┐
          │  Unit Tests │  ← 70% of tests, fast
          └─────────────┘
          ┌─────────────┐
          │  Fuzz Tests │  ← 10% of tests, automated
          └─────────────┘
```

### Test Types

| Test Type | Framework | Location | Coverage |
|-----------|-----------|----------|----------|
| Rust unit tests | cargo test | src/*/tests/ | High |
| Rust doctests | cargo test --doc | src/*.rs | Medium |
| Idris2 tests | idris2 | src/interface/tests/ | Medium |
| Zig tests | zig test | src/interface/ffi/tests/ | High |
| Integration tests | custom | tests/ | High |
| E2E tests | custom | tests/e2e/ | Medium |
| Fuzz tests | cargo fuzz | fuzz/ | High |

### Running Tests

```bash
# All tests
just test

# Rust tests only
cargo test

# Idris2 type checking
just typecheck

# FFI verification
just verify-ffi

# E2E tests
git grep -l "sorry\|believe_me\|postulate" src/
```

### Test Coverage

```bash
# Generate coverage report
just coverage

# View coverage report
open target/coverage/html/index.html
```

---

## Debugging

### Rust Debugging

```bash
# Debug build
cargo build

# Run with logging
RUST_LOG=debug cargo run

# LLDB debugging
rust-lldb target/debug/paint-type

# GDB debugging
gdb target/debug/paint-type
```

### Zig Debugging

```bash
# Debug build
zig build -Doptimize=Debug

# Run with logging
zig build run -Doptimize=Debug
```

### Proof Debugging (Coq)

```bash
# Check proof state
coqide formal/Semantics_L1.v

# Check admitted proofs
grep -n "Admitted\." formal/*.v

# Verify proof count
./scripts/status-gate.sh --proofs
```

---

## Performance Optimization

### Profiling

```bash
# Rust profiling
cargo build --release
perf record target/release/paint-type
perf report

# Flamegraph
cargo install flamegraph
cargo flamegraph

# Memory profiling
cargo install dhat
cargo dhat
```

### Benchmarking

```bash
# Run benchmarks
cargo bench

# Custom benchmarks
just bench
```

### Optimization Targets

| Component | Target | Current | Goal |
|-----------|--------|---------|------|
| Tile composite | < 1ms | ~0.5ms | < 0.1ms |
| Brush stamp | < 1ms | ~0.8ms | < 0.2ms |
| Layer composite | < 10ms | ~5ms | < 2ms |
| Undo/Redo | O(1) | O(1) | O(1) |
| Canvas render | 60fps | ~45fps | 60fps |

---

## Code Review

### Review Checklist

#### For All Changes

- [ ] Build passes (`just build`)
- [ ] Tests pass (`just test`)
- [ ] Type checking passes (`just typecheck`)
- [ ] No new warnings
- [ ] Documentation updated
- [ ] Changelog updated (if user-facing)

#### For Rust Changes

- [ ] No `unsafe` blocks (or justified with `// SAFETY:`)
- [ ] No `.unwrap()` or `.expect()` (use proper error handling)
- [ ] No `panic!` (return Result/Option)
- [ ] Linear types respected (`!Copy + !Clone` for Tile)
- [ ] No aliased mutable access
- [ ] FFI boundaries are safe

#### For Zig Changes

- [ ] Explicit allocators used
- [ ] No `expect`/`catch unreachable` (use proper error handling)
- [ ] C ABI compatibility maintained
- [ ] No undefined behavior

#### For Idris2 Changes

- [ ] No `believe_me`
- [ ] No `assert_total`
- [ ] No `postulate`
- [ ] Totality maintained
- [ ] Proofs are REAL (not placeholders)

#### For Coq Changes

- [ ] Admitted proofs documented in PROOF-NEEDS.md
- [ ] Admitted proofs counted in status-gate
- [ ] Research questions documented

### Review Commands

```bash
# Check for unsafe Rust
git grep -n "unsafe" src/

# Check for unwrap/expect
git grep -n "\.unwrap\|\.expect" src/

# Check for panic
git grep -n "panic!" src/

# Check for Idris2 holes
git grep -n "believe_me\|assert_total\|postulate" src/

# Check for Coq admits
git grep -n "Admitted\." formal/

# Run panic-attack scan (Ephapax)
panic-attack assail --config panic-attack.toml
```

---

## Contributing

### Pull Request Process

1. **Fork** the repository
2. **Clone** your fork
3. **Create a branch** (`git checkout -b feature/your-feature`)
4. **Make changes** (follow coding standards)
5. **Run tests** (`just test`)
6. **Run type checking** (`just typecheck`)
7. **Update documentation**
8. **Update changelog** (if applicable)
9. **Push** to your fork
10. **Open PR** to main repository

### PR Template

```markdown
## Description

Describe your changes.

## Related Issues

- Closes #123
- Related to #456

## Changes

- Feature: Added X
- Bugfix: Fixed Y
- Refactor: Improved Z

## Checklist

- [ ] Build passes
- [ ] Tests pass
- [ ] Type checking passes
- [ ] Documentation updated
- [ ] Changelog updated

## Verification

```bash
# Commands to verify the changes
just build
just test
just typecheck
```
```

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): subject

body

footer
```

**Types:** feat, fix, docs, style, refactor, test, chore, revert

**Examples:**
```
feat(paint_core): add multiply blend mode
fix(ffi): correct pointer alignment in tile layout
 docs(readme): update status badges
docs(architecture): add tier diagram
refactor(composite): optimize over operator
 test(brush): add stroke interpolation tests
chore(ci): update Rust version
```

---

## Continuous Integration

### CI Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| rust.yml | Push/PR | Rust CI |
| idris-ci.yml | Push/PR | Idris2 CI |
| e2e.yml | Push/PR | End-to-end tests |
| coverage.yml | Push/PR | Coverage reporting |
| codeql.yml | Push/PR | CodeQL analysis |
| scorecard.yml | Daily | OpenSSF Scorecard |
| fuzz.yml | Nightly | Fuzz testing |

### CI Commands

```yaml
# Rust CI
- cargo build
- cargo test
- cargo clippy
- cargo fmt --check

# Idris2 CI
- idris2 --check --total paint-type.ipkg
- pack typecheck proven.ipkg

# E2E CI
- just build
- just test
- just typecheck
```

### CI Badges

```markdown
![Rust CI](https://github.com/metadatastician/paint-type/actions/workflows/rust.yml/badge.svg)
![Idris2 CI](https://github.com/metadatastician/paint-type/actions/workflows/idris-ci.yml/badge.svg)
![E2E](https://github.com/metadatastician/paint-type/actions/workflows/e2e.yml/badge.svg)
![Coverage](https://github.com/metadatastician/paint-type/actions/workflows/coverage.yml/badge.svg)
![CodeQL](https://github.com/metadatastician/paint-type/actions/workflows/codeql.yml/badge.svg)
![Scorecard](https://github.com/metadatastician/paint-type/actions/workflows/scorecard.yml/badge.svg)
```

---

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| Rust version mismatch | `rustup update` or check `rust-toolchain` file |
| Zig version mismatch | Update Zig to latest stable |
| Idris2 not found | `pack install idris2` |
| just not found | `cargo install just` |
| Build fails | `just clean` then `just build` |
| Tests fail | Check test output, run specific test |
| Type checking fails | Check Idris2 error messages |

### Build Errors

**Error:** `error: could not find rustc`
```bash
rustup install stable
source $HOME/.cargo/env
```

**Error:** `zig: command not found`
```bash
# Download from https://ziglang.org/download/
zig version
```

**Error:** `idris2: command not found`
```bash
pack install idris2
# or
which idris2
```

**Error:** `just: command not found`
```bash
cargo install just
```

### Test Failures

**Error:** Tests fail in CI but pass locally
```bash
# Run exact CI commands
cargo test
cargo test --doc
```

**Error:** FFI tests fail
```bash
# Check FFI implementation
just verify-ffi

# Check Zig build
cd src/interface/ffi && zig build
```

### Type Checking Failures

**Error:** Idris2 type checking fails
```bash
# Check specific file
idris2 --check src/interface/Abi/Types.idr

# Check with totality
idris2 --check --total paint-type.ipkg
```

---

## References

- [README.adoc](https://github.com/metadatastician/paint-type/blob/main/README.adoc) — Project overview
- [QUICKSTART-DEV.adoc](https://github.com/metadatastician/paint-type/blob/main/QUICKSTART-DEV.adoc) — Developer quickstart
- [QUICKSTART-MAINTAINER.adoc](https://github.com/metadatastician/paint-type/blob/main/QUICKSTART-MAINTAINER.adoc) — Maintainer guide
- [CONTRIBUTING.md](https://github.com/metadatastician/paint-type/blob/main/CONTRIBUTING.md) — Contribution guidelines
- [ROADMAP.adoc](https://github.com/metadatastician/paint-type/blob/main/ROADMAP.adoc) — Milestone plan
- [Justfile](https://github.com/metadatastician/paint-type/blob/main/Justfile) — All build recipes

---

*This document was last updated on 2026-07-26. For the most current information, refer to the in-repo documentation.*

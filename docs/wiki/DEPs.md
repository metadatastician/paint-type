# paint.type DEPs — Dependency Epics

**Source of Truth:** This page summarizes DEP status. For detailed information, see the in-repo DEP files.
**Last Updated:** 2026-07-26
**Maintainer:** hyperpolymath (estate owner)

---

## Overview

DEPs (Dependency Epics) are the **15 critical upstream dependencies** that paint-type relies on. Each DEP represents a foundational component that must be "made great" for paint-type to achieve its full potential.

**Current Status Summary:**
- ✅ **DONE:** 4 DEPs (DEP-02, DEP-04, DEP-09, DEP-10)
- 🟡 **In Progress:** 4 DEPs (DEP-01, DEP-05, DEP-12, DEP-15)
- ❌ **BLOCKED:** 1 DEP (DEP-03)
- 📋 **Todo:** 6 DEPs (DEP-06, DEP-07, DEP-08, DEP-11, DEP-13, DEP-14)

---

## DEP Registry

### 🟢 DONE (4)

| DEP | Dependency | Tier | Priority | Readiness | Closure Date | Impact |
|-----|------------|------|----------|-----------|---------------|--------|
| DEP-02 | **typed-wasm** | Spine | Maintain | A | 2026-07-26 | WASM bridge verified, paint-type slice closed |
| DEP-04 | **Gossamer** | Spine | P1 | A | 2026-07-26 | Desktop shell integrated, v0.3.0/v0.3.1 released |
| DEP-09 | **standards** | Governance | P1 | B | 2026-07-26 | CI/CD reusables green, multiplier operational |
| DEP-10 | **proven** | Proof-foundation | P1 | B | 2026-07-26 | Idris2 proof foundation solid, Track-C triage closed |

---

### 🟡 In Progress (4)

| DEP | Dependency | Tier | Priority | Readiness | Status | Next Steps |
|-----|------------|------|----------|-----------|--------|------------|
| DEP-01 | **Ephapax** | Spine | P1 | C | V2 redesign active | Close 2 open research admits |
| DEP-05 | **cerro-torre** | Plugin/Supply-chain | P2 | B | ML-DSA capability added | Full proven-pqc integration |
| DEP-12 | **panic-attack** | Tooling | Maintain | B | Ephapax automation added | Keep assail gate green |
| DEP-15 | **Toolchain & formats** | Tooling | P1 | C | Estate migration ongoing | Nix→Guix migration |

---

### ❌ BLOCKED (1)

| DEP | Dependency | Tier | Priority | Readiness | Blocker | Impact |
|-----|------------|------|----------|-----------|---------|--------|
| DEP-03 | **AffineScript** | Spine | P1 | C | **NO WORKING COMPILER** | Blocks v0.2→v0.3 shipping, UI chrome compilation |

**Evidence:**
- git-reticulator/PROOF-NEEDS.md: "AffineScript has no working compiler yet"
- git-reticulator/docs/decisions/rust-spark-stance.adoc: "AffineScript has no working compiler, and the code calls git2/postgres crates AffineScript cannot bind"

**Completed:**
- ✅ ABI→`.twasm` generator (paint-type#39)

**Blocked:**
- ❌ Widen typed-wasm enforcement (L1–L5)
- ❌ Compile UI chrome (layer-panel, tool-bar, canvas-viewport)

---

### 📋 Todo (6)

| DEP | Dependency | Tier | Priority | Readiness | Tasks |
|-----|------------|------|----------|-----------|-------|
| DEP-06 | **Burble** | Collaboration | P2 | Unknown | Prove WebRTC data-channel, sub-10ms API, burble↔groove wiring |
| DEP-07 | **groove-protocol** | Collaboration | P2 | B | Finish 8/10 FFI binding targets, Groove manifest integration |
| DEP-08 | **boj-server** | Collaboration | P3 | B | Stable cartridge surface, package-index tier |
| DEP-11 | **echo-types** | Proof-foundation | P2 | C | Keep Agda build working, audit-and-record proofs |
| DEP-13 | **Container / supply-chain stack** | Plugin/Supply-chain | P2 | C | Signed container pipeline, cerro-torre ML-DSA, Svalinn + Vordr |
| DEP-14 | **Hypatia** | Spine | P2 | B | Keep Hexadeca reference canonical, conformant copy |

---

## Detailed DEP Status

### DEP-01: Ephapax — Region-Calculus / Linear-Affine Core

**Repository:** https://github.com/hyperpolymath/ephapax  
**Live Path:** `/home/hyperpolymath/developer/hyper-repos/_LANGUAGES _SET/_NEXTGEN_LANGUAGES _SET/ephapax/`  
**Issue File:** [.github/ISSUES/DEP-01-EPHAPAX.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-01-EPHAPAX.adoc)

#### V1 → V2 Redesign

| Layer | Concern | Status | Qed/Admitted |
|-------|---------|--------|--------------|
| L1 | Region capabilities | Active | 54 Qed / 2 Admitted |
| L2 | Structural modality | Complete | 1 Qed / 0 Admitted |
| L3 | Echo/residue calculus | Complete | 12 Qed / 0 Admitted |
| L4 | Dyadic interaction | Scaffold | 0 Qed / 0 Admitted |

#### Open Research Questions

1. **`formal/Semantics_L1.v:3318`** — `step_pop_disjoint_from_type_l1`
2. **`formal/Semantics_L1.v:3337`** — `preservation_l1` (capstone, gated on step_pop)

**Research Document:** `formal/L1-ELIMINATOR-FORK.md`

**Challenge:** Preservation needs to handle a case where a subterm depends on a region `rv`, but an eliminator erases `rv` from the result type, and a step exits `rv`. The judgment's snapshot + result-type-local view cannot express "rv must be live throughout the evaluation of e_sub".

**Proposed Solution:** Type region-liveness **choreographically**, across time segments. Replace "liveness = membership in a snapshot" with "liveness = a position in a global protocol over time segments".

#### Automation (2026-07-26)

- ✅ `panic-attack.toml` — Configuration created
- ✅ `panic-attack.yml` — CI workflow created
- ✅ Status gate CI active

**Verification:**
- Claimed admitted proofs: 3 (PROOF-NEEDS.md §4)
- Actual admitted proofs: 3 (grep formal/*.v)
- **Status: ✅ IN SYNC**

#### Remaining Tasks

- [x] Set up drift detection (panic-attack)
- [ ] Verify Gossamer's Ephapax integration is genuinely load-bearing
- [ ] Service-handle region modelling (gossamer#69)

---

### DEP-02: typed-wasm — Verified WASM Bridge Target

**Repository:** https://github.com/hyperpolymath/typed-wasm  
**Issue File:** [.github/ISSUES/DEP-02-TYPED-WASM.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-02-TYPED-WASM.adoc)

#### Completed Tasks

1. ✅ **paint-type slice closed** (#127 closed)
2. ✅ **#130 slice green** via PR#165 (`ba3c7d9`)
3. ✅ **Hand-modelled `.twasm` schemas** — `src/bridges/paint-type-{tile,layer}.twasm`

#### Verification

- All `.twasm` schemas compile successfully
- All schemas pass typed-wasm type checking
- paint-type CI includes typed-wasm verification
- WASM modules consumed by paint-type runtime

#### Remaining Maintenance

- Keep `tw`/`tw-verify` pinned in paint-type
- Full #130 corpus
- D6 human-readable errors (#126)

**Status: ✅ DONE — All critical tasks complete**

---

### DEP-03: AffineScript — UI Language + ABI→.twasm Generator

**Repository:** https://github.com/hyperpolymath/affinescript  
**Live Path:** `/home/hyperpolymath/developer/worktrees/affinescript/`  
**Issue File:** [.github/ISSUES/DEP-03-AFFINESCRIPT.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-03-AFFINESCRIPT.adoc)

#### Critical Blocker: NO WORKING COMPILER

This is a **fundamental implementation gap**, not an automation or documentation problem.

**Repository State:**
- Language: OCaml
- Build system: dune
- Test suite: 257 tests (green per justfile)
- Version: Alpha (0.1.1)
- **Compilable: ❌ NO**

#### Impact

- Blocks paint-type v0.2→v0.3 shipping
- Blocks UI chrome compilation
- Blocks typed-wasm widening to L1–L5
- Blocks DEP-03 completion

#### Alternative Approach

Consider using **Ephapax directly** for the UI chrome if AffineScript compiler completion is far off. This would be a **different architecture** requiring significant redesign.

**Note:** Ephapax is NOT AffineScript. They are separate languages with different purposes.

**Disambiguation:** https://github.com/hyperpolymath/nextgen-languages/blob/main/docs/disambiguation/ephapax-vs-affinescript.md

**Status: ❌ BLOCKED — Cannot proceed without working compiler**

---

### DEP-04: Gossamer — Desktop Shell / Webview Host

**Repository:** https://github.com/hyperpolymath/gossamer  
**Live Path:** `/home/hyperpolymath/developer/meta-repos/gossamer/`  
**Issue File:** [.github/ISSUES/DEP-04-GOSSAMER.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-04-GOSSAMER.adoc)

#### Completed Tasks

1. ✅ **Integrated into paint-type shell** — Replaced direct GTK3/WebKitGTK calls with Gossamer API
2. ✅ **v0.3.0 Desktop Shell closed** (issue #34) — Web UI chrome, tool primitives, file open/save
3. ✅ **Gossamer v0.3.1 released** (2026-04-03) — Mobile bug fixes, 173 integration tests

#### Ephapax Integration

Uses Ephapax for:
- Linearly-owned webview handles
- Typed IPC protocol
- Linear capability tokens

**Modules:** Shell.eph, Bridge.eph, Capabilities.eph

**Verification needed:** Confirm integration is **load-bearing** (not just namechecked). Tracked in DEP-01.

#### Remaining Verification

- Close Idris2-ABI vs Zig-FFI drift (audit `wqbb3mhzf`)
- Reconcile versioning (git tag `v0.1.0` vs README `v0.3.1`)
- Nix→Guix migration
- Android integration (gossamer#67, #68, #69, #71)

**Status: ✅ DONE — All critical tasks complete**

---

### DEP-05: cerro-torre — ML-DSA Plugin Signing

**Repository:** Standalone repo in meta-repos  
**Issue File:** Tracked in DEPENDENCY-SCHEDULER.adoc

#### Completed Tasks

1. ✅ **Graduated from `stapeln/container-stack/cerro-torre/` to standalone repo in meta-repos**
2. ✅ **Added ML-DSA-87 capability surface** (standalone repo with FFI bindings)

#### Remaining Tasks

1. ⏳ Full proven-pqc integration for production ML-DSA-87 signatures
2. ⏳ Decide thin paint-type-local interim signer (reuse `proven-pqc` ML-DSA)

**Status: In Progress — ML-DSA capability surface working**

---

### DEP-06: Burble — WebRTC Session, Sub-10ms Shared Canvas

**Repository:** https://github.com/hyperpolymath/burble  
**Status:** Todo

#### Tasks

- Prove the WebRTC data-channel can carry tile-edit traffic (voice-first today)
- Sub-10ms session/signalling API
- burble↔groove signalling wiring

**Impact:** Required for v0.5 Collaboration

---

### DEP-07: groove-protocol — Zero-Config Peer Discovery

**Repository:** https://github.com/hyperpolymath/groove-protocol  
**Status:** Todo

#### Tasks

- Finish the 8/10 untested FFI binding targets
- paint-type emits/consumes a Groove manifest as a peer

**Impact:** Required for v0.5 Collaboration

---

### DEP-08: boj-server — MCP LLM Channel + Plugin Package Index

**Repository:** https://github.com/hyperpolymath/boj-server  
**Status:** Todo

#### Tasks

- Stable cartridge surface for the paint-type LLM channel
- Package-index tier for the plugin browser

**Impact:** Required for v0.5 Collaboration LLM features

---

### DEP-09: standards — RSR Governance Multiplier

**Repository:** https://github.com/hyperpolymath/standards  
**Live Path:** `/home/hyperpolymath/developer/hyper-repos/standards/`  
**Issue File:** [.github/ISSUES/DEP-09-STANDARDS.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-09-STANDARDS.adoc)

#### Completed Tasks (All 3)

1. ✅ **Keep CI/CD reusables green** — Actions billing-wall / `startup_failure` monitoring active
2. ✅ **Contractile trident canon** — standards subdir is master (NOT rsr-template's stale copy)
3. ✅ **CRG + 6a2 manifest currency** — All manifests generated and current

#### Repository State

- Reusable workflows: 6+ (governance, codeql, hypatia, changelog, deno, elixir)
- Generation scripts: 3+ (registry, topology, scorecards)
- Contractile recipes: ✅ Active
- Machine-readable: ✅ Generated

#### Impact

DEP-09 is the **continuous multiplier** — every repo in the estate inherits its:
1. CI/CD reusables
2. Contractile recipes
3. CRG and 6a2 standards
4. Drift prevention mechanisms

**Status: ✅ DONE — All automation operational as of 2026-07-26**

---

### DEP-10: proven — Idris2 Verified-Foundations Library

**Repository:** https://github.com/hyperpolymath/proven  
**Live Path:** `/home/hyperpolymath/developer/hyper-repos/proven/`  
**Issue File:** [.github/ISSUES/DEP-10-PROVEN.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-10-PROVEN.adoc)

#### Completed Tasks

1. ✅ **Track-C security triage (proven#68)** — **Closed via PR #162**
2. ✅ **Build pipeline working** — Idris2 → RefC → C → Zig FFI
3. ✅ **Type checking working** — Full library and FFI packages
4. ✅ **Totality verification** — All totality checks passing

#### Repository State

- Language: Idris2
- Build system: just + pack + zig
- Lines of code: 3265 (per MODULE-STATUS-RAW.txt)
- Modules: 24+ (per proven.ipkg)
- Status: Working and consumable

#### Consumability

proven is consumable as the **Idris2 proof foundation** for:
- paint-type (ABI definitions)
- ephapax (formal proofs)
- Other estate repos

**ABI surface:** `src/abi/Ephapax/…` — 17 files, clean of `believe_me` / `sorry` / `postulate`

**Formal proofs:** `src/formal/Ephapax/…` — Region linearity, narrow no-escape proof, no `believe_me` / `sorry` / `assert_total`

**Status: ✅ DONE — Foundation solid and working**

---

### DEP-11: echo-types — Estate Proof Discipline

**Repository:** (Agda-based, local)  
**Status:** Todo

#### Tasks

- Keep the local Agda build env working (the `~/.agda/libraries` path-drift)
- Audit-and-record echo-types for each new paint-type proof

---

### DEP-12: panic-attack — Dangerous-Pattern Gate

**Repository:** https://github.com/hyperpolymath/panic-attack  
**Status:** In Progress

#### Automation Added 2026-07-26

- ✅ `panic-attack.toml` created for Ephapax
- ✅ `panic-attack.yml` CI workflow created for Ephapax

#### Task

- Keep the `assail` gate green in paint-type CI (no `believe_me`/`sorry`/`postulate`)

---

### DEP-13: Container / Supply-Chain Stack

**Status:** Todo

#### Tasks

- Signed container build pipeline (paint-type Containerfile = Wolfi + ML-DSA-87)
- cerro-torre ML-DSA integration (see DEP-05)
- Svalinn gateway + Vordr runtime wiring

---

### DEP-14: Hypatia (Hexadeca source) — 16-Protocol Unified Connector

**Status:** Todo

#### Tasks

- Keep the Hexadeca 16-protocol reference canonical
- Keep paint-type's connector copy conformant (no drift)

---

### DEP-15: Toolchain & Formats

**Status:** In Progress

#### Tasks

- Nix→Guix migration (estate-wide directive; Nix is now debt)
- Pin provers (Idris2 0.8.0 / Lean4 4.13.0 / Agda 2.8.0 / Coq / TLA+)
- Reconcile rsr-template's flat-contractile drift against the standards canon

---

## Critical Path Analysis

### Current Critical Path

```
DEP-01 Ephapax (In Progress: 2 research admits)
    ↓
DEP-03 AffineScript (BLOCKED: no compiler) ← PRIMARY BOTTLENECK
    ↓
v0.2→v0.3 Shipping BLOCKED
```

### Milestone Dependencies

| Milestone | DEP Dependencies | Status |
|----------|-------------------|--------|
| v0.2.0 Core Image Operations | DEP-01, DEP-02, DEP-03 | v0.2.0 **closed** (DEP-03 blocked but workarounds in place) |
| v0.3.0 Desktop Shell | DEP-04 | ✅ **closed** |
| v0.4.0 Plugin System | DEP-02, DEP-05, DEP-10 | ✅ **closed** |
| v0.5.0 Collaboration | DEP-06, DEP-07, DEP-08 | In progress, blocked on upstream |

### Continuous Multiplier

**DEP-09 standards** — Lifts all other DEPs by providing:
- Consistent CI/CD reusables
- Consistent governance (contractiles)
- Consistent grading (CRG + 6a2)
- Drift prevention (generated files fail CI if stale)

**Status:** ✅ **DONE and operational**

---

## Automation Status

For detailed automation coverage, see [DEP-AUTOMATION-TRACKING.adoc](https://github.com/metadatastician/paint-type/blob/main/DEP-AUTOMATION-TRACKING.adoc).

### Coverage Summary

| Category | Full | Partial | None | Total |
|----------|------|---------|------|-------|
| Spine | 2 | 1 | 1 | 4 |
| Proof-Foundation | 2 | 0 | 1 | 3 |
| Plugin/Supply-Chain | 0 | 1 | 1 | 2 |
| Collaboration | 0 | 1 | 2 | 3 |
| Tooling | 1 | 1 | 0 | 2 |
| Additional | 0 | 1 | 0 | 1 |
| **Total** | **5** | **5** | **5** | **15** |

### 2026-07-26 Automation Additions

- ✅ DEP-01: panic-attack.toml + panic-attack.yml CI workflow
- ✅ DEP-12: panic-attack automation operational for Ephapax
- ✅ DEP-AUTOMATION-TRACKING.adoc created
- ✅ DEP-IMPLEMENTATION-STATUS.adoc created

---

## Standing Items (Continuous)

These are **not** one-off make-great tasks — they are continuous obligations addressed "as they arise" each session.

| ID | Item | Owner | Source of Truth |
|----|------|-------|----------------|
| STAND-01 | Code-scanning / Scorecard remediation | hyperpolymath | `…/paint-type/security/code-scanning` |
| STAND-02 | Wiki development | Joshua / shared | `paint-type.wiki` |
| STAND-03 | Standards-doc + machine-readable currency | hyperpolymath | `.machine_readable/`, TEMPLATE-STANDARDS-AUDIT.adoc |

**STAND-03 Status:** ✅ **FULLY RESOLVED 2026-07-26** — All generated files current, automation working

---

## Quick Reference

### For Humans

| Question | Answer |
|----------|--------|
| What's the biggest blocker? | DEP-03 AffineScript has no working compiler |
| What DEPs are done? | DEP-02, DEP-04, DEP-09, DEP-10 |
| What's in progress? | DEP-01 (2 admits), DEP-05, DEP-12, DEP-15 |
| What's the critical path? | DEP-03 → v0.2→v0.3 shipping |
| What's the multiplier? | DEP-09 standards (all repos inherit from it) |

### For Bots/AI

```bash
# Check current DEP status
cat DEPENDENCY-SCHEDULER.adoc

# Check detailed implementation status
cat DEP-IMPLEMENTATION-STATUS.adoc

# Check automation coverage
cat DEP-AUTOMATION-TRACKING.adoc

# Focus on P1/In Progress
# DEP-01: Ephapax — 2 open research admits
# DEP-03: AffineScript — BLOCKED (skip)
# DEP-05: cerro-torre — ML-DSA integration
# DEP-12: panic-attack — keep gates green
# DEP-15: toolchain — Nix→Guix migration
```

---

## References

### In-Repo Documentation

- [DEPENDENCY-SCHEDULER.adoc](https://github.com/metadatastician/paint-type/blob/main/DEPENDENCY-SCHEDULER.adoc) — Live kanban mirror
- [DEP-IMPLEMENTATION-STATUS.adoc](https://github.com/metadatastician/paint-type/blob/main/DEP-IMPLEMENTATION-STATUS.adoc) — Detailed status of all 15 DEPs
- [DEP-AUTOMATION-TRACKING.adoc](https://github.com/metadatastician/paint-type/blob/main/DEP-AUTOMATION-TRACKING.adoc) — Automation coverage matrix

### DEP Issue Files

All DEP issue files are in `.github/ISSUES/`:
- [DEP-01-EPHAPAX.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-01-EPHAPAX.adoc)
- [DEP-02-TYPED-WASM.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-02-TYPED-WASM.adoc)
- [DEP-03-AFFINESCRIPT.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-03-AFFINESCRIPT.adoc)
- [DEP-04-GOSSAMER.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-04-GOSSAMER.adoc)
- [DEP-09-STANDARDS.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-09-STANDARDS.adoc)
- [DEP-10-PROVEN.adoc](https://github.com/metadatastician/paint-type/blob/main/.github/ISSUES/DEP-10-PROVEN.adoc)

### Upstream Repositories

- [ephapax](https://github.com/hyperpolymath/ephapax) — DEP-01
- [typed-wasm](https://github.com/hyperpolymath/typed-wasm) — DEP-02
- [affinescript](https://github.com/hyperpolymath/affinescript) — DEP-03
- [gossamer](https://github.com/hyperpolymath/gossamer) — DEP-04
- [standards](https://github.com/hyperpolymath/standards) — DEP-09
- [proven](https://github.com/hyperpolymath/proven) — DEP-10

---

*This document was last updated on 2026-07-26. For the most current information, refer to the in-repo DEP files as the Single Source of Truth.*

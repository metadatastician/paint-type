# paint.type Agent Guide

**Source of Truth:** This guide is for AI assistants, bots, and automated agents working on paint.type.
**Last Updated:** 2026-07-26
**Maintainer:** hyperpolymath

---

## Overview

This guide provides instructions for AI assistants, bots, and automated agents working on the paint.type repository. It covers session startup, task prioritization, automation, and best practices.

**Target Audience:**
- AI coding assistants (Claude Code, Codex, Gemini CLI, etc.)
- Automated bots and scripts
- CI/CD agents
- Coding agents (Mistral Vibe, etc.)

---

## Session Startup Protocol

### Step 1: Read the Source of Truth

On session start, **ALWAYS** read these files in order:

```bash
# 1. Repository root files
cat README.adoc              # Project overview and status
cat DEPENDENCY-SCHEDULER.adoc # Current priorities

# 2. DEP tracking files (2026-07-26 additions)
cat DEP-IMPLEMENTATION-STATUS.adoc  # Detailed status of all 15 DEPs
cat DEP-AUTOMATION-TRACKING.adoc   # Automation coverage matrix

# 3. Roadmap
cat ROADMAP.adoc              # Milestone plan

# 4. AGENTS.md (if exists)
cat AGENTS.md                 # Repository-specific instructions
```

### Step 2: Check Current State

```bash
# Check git status
git status

# Check recent commits
git log --oneline -20

# Check CI status (if accessible)
gh api repos/metadatastician/paint-type/actions/runs

# Check for open issues
gh issue list --state open
```

### Step 3: Identify Working Set

Based on DEPENDENCY-SCHEDULER.adoc, identify:
1. **P1/In Progress** items — these are the working set
2. **BLOCKED** items — do NOT start work on these
3. **DONE** items — reference for context, no action needed

---

## Task Prioritization

### Priority Order

When multiple tasks are available, work in this order:

```
1. P1 + BLOCKED items that can be unblocked
   └─ Current: DEP-03 AffineScript (cannot be unblocked — no compiler)
   
2. P1 + In Progress items
   ├─ DEP-01 Ephapax — 2 open research admits (L1 eliminator fork)
   ├─ DEP-03 AffineScript — BLOCKED (skip)
   └─ DEP-15 Toolchain — Nix→Guix migration
   
3. P1 + Todo items (if any)
   
4. P2 + In Progress items
   ├─ DEP-05 cerro-torre — ML-DSA integration
   └─ DEP-12 panic-attack — keep assail gate green
   
5. STAND items (standing/continuous)
   ├─ STAND-01: Code-scanning / Scorecard remediation
   └─ STAND-02: Wiki development
```

### Current Working Set (2026-07-26)

| DEP | Task | Priority | Status | Action |
|-----|------|----------|--------|--------|
| DEP-01 | Close 2 open research admits | P1 | In Progress | **ACTIVE: Research L1 eliminator fork** |
| DEP-01 | Verify Gossamer integration | P1 | In Progress | **ACTIVE: Audit integration** |
| DEP-01 | Service-handle region modelling | P1 | In Progress | **ACTIVE: Create gossamer#69 design doc** |
| DEP-05 | Full proven-pqc integration | P2 | In Progress | **ACTIVE: Integrate ML-DSA-87** |
| DEP-12 | Keep assail gate green | Maintain | In Progress | **ACTIVE: Monitor panic-attack CI** |
| DEP-15 | Nix→Guix migration | P1 | In Progress | **ACTIVE: Migrate from Nix** |

### Blocked Items (Do NOT Start)

| DEP | Blocker | Reason |
|-----|---------|--------|
| DEP-03 | No working compiler | Fundamental implementation gap |

**Verification:** Check git-reticulator/PROOF-NEEDS.md and rust-spark-stance.adoc for confirmation.

---

## Automation Protocol

### Always Run Before Committing

```bash
# 1. Build
just build

# 2. Tests
just test

# 3. Type checking
just typecheck

# 4. Verification
just verify-ffi

# 5. Proof count check (Ephapax)
./scripts/status-gate.sh --proofs

# 6. panic-attack scan (Ephapax)
panic-attack assail --config panic-attack.toml
```

### Drift Detection

**Before making changes**, check for drift:

```bash
# Check if generated files are current
./scripts/build-registry.sh --check
./scripts/build-scorecards.sh --check
./scripts/verify-manifests.sh

# Check TEMPLATE-STANDARDS-AUDIT.adoc
# This is now generated, so it should never be stale
./scripts/build-standards-audit.sh --check 2>/dev/null || true
```

### Post-Change Verification

After making changes, **ALWAYS** verify:

```bash
# 1. Rebuild
just build

# 2. Rerun tests
just test

# 3. Verify proof count (if Coq files changed)
./scripts/status-gate.sh --proofs

# 4. Verify no new admits (if formal files changed)
grep -R --include='*.v' -n '^[[:space:]]*Admitted\.' formal

# 5. Update PROOF-NEEDS.md if admits changed
#    (This is tracked by status-gate.yml CI)
```

---

## DEP-Specific Protocols

### DEP-01: Ephapax

**When working on DEP-01:**

1. **Read first:**
   - `formal/PRESERVATION-DESIGN.md` — Design doctrine
   - `formal/L1-ELIMINATOR-FORK.md` — Research plan for L1 admits
   - `.github/ISSUES/DEP-01-EPHAPAX.adoc` — Current status

2. **Research questions (2 open admits):**
   - `formal/Semantics_L1.v:3318` — `step_pop_disjoint_from_type_l1`
   - `formal/Semantics_L1.v:3337` — `preservation_l1` (gated on step_pop)

3. **The challenge:**
   - Preservation needs: if `R ; G ⊢ e : T -| R' ; G'` and `(μ,R,e) → (μ',R₂,e')`, then `R₂ ; G ⊢ e' : T -| …`
   - It fails on a single shape where a subterm depends on region `rv`, but an eliminator erases `rv` from the result type, and a step exits `rv`

4. **Proposed solution:**
   - Type region-liveness **choreographically**, across time segments
   - Replace "liveness = membership in a snapshot" with "liveness = a position in a global protocol over time segments"

5. **Deciding experiment:**
   - Formulate the minimal choreography that types `EDrop (EVar j : TString rv)` over two segments
   - Ask: Does the choreography's subject-reduction step for `S_Region_Exit rv` transform the global type into a coherent global type without a snapshot premise?
   - If yes → ADMIT 3 closes choreographically
   - If no → Problem has relocated, not closed

6. **Automation already in place:**
   - `panic-attack.toml` — Created 2026-07-26
   - `panic-attack.yml` — CI workflow created 2026-07-26
   - Status gate CI — Already prevents proof count drift

### DEP-02: typed-wasm

**Status:** ✅ DONE — No action needed unless maintenance requested.

**Maintenance tasks (low priority):**
- Keep `tw`/`tw-verify` pinned in paint-type
- Full #130 corpus
- D6 human-readable errors (#126)

**Do NOT:**
- Break existing `.twasm` schemas
- Change typed-wasm version without updating paint-type

### DEP-03: AffineScript

**Status:** ❌ BLOCKED — **DO NOT START WORK**

**Blocker:** No working compiler exists.

**Evidence to verify:**
```bash
# Check git-reticulator sources
grep -r "AffineScript has no working compiler" /home/hyperpolymath/developer/hyper-repos/git-reticulator/ 2>/dev/null || echo "Source not found locally"

# The blocker is confirmed in:
# - git-reticulator/PROOF-NEEDS.md
# - git-reticulator/docs/decisions/rust-spark-stance.adoc
```

**Completed work (do not duplicate):**
- ✅ ABI→`.twasm` generator (paint-type#39) — Already done

**Blocked work (cannot proceed):**
- ❌ Widen typed-wasm enforcement (L1–L5)
- ❌ Compile UI chrome (layer-panel, tool-bar, canvas-viewport)

**Alternative approach (if requested):**
If asked to explore alternatives, document that using **Ephapax directly** for UI chrome would be a different architecture requiring significant redesign. Ephapax is NOT AffineScript.

### DEP-04: Gossamer

**Status:** ✅ DONE — Integration complete.

**Verification tasks (if requested):**
- Audit Idris2-ABI vs Zig-FFI surface drift (`wqbb3mhzf`)
- Reconcile versioning (git tag `v0.1.0` vs README `v0.3.1`)
- Nix→Guix migration
- Android integration (gossamer#67, #68, #69, #71)

**Integration to verify:**
- Confirm Ephapax integration is **load-bearing** (tracked in DEP-01)
- Modules: Shell.eph, Bridge.eph, Capabilities.eph

### DEP-05: cerro-torre

**Status:** In Progress

**Completed:**
- ✅ Graduated to standalone repo in meta-repos
- ✅ ML-DSA-87 capability surface added

**Remaining:**
- Full proven-pqc integration
- Interim signer decision (reuse `proven-pqc` ML-DSA)

### DEP-06: Burble

**Status:** Todo

**Tasks:**
- Prove WebRTC data-channel can carry tile-edit traffic
- Sub-10ms session/signalling API
- burble↔groove signalling wiring

**Note:** Currently voice-first, needs data-channel proof.

### DEP-07: groove-protocol

**Status:** Todo

**Tasks:**
- Finish 8/10 untested FFI binding targets
- paint-type emits/consumes Groove manifest as a peer

### DEP-08: boj-server

**Status:** Todo

**Tasks:**
- Stable cartridge surface for paint-type LLM channel
- Package-index tier for plugin browser

### DEP-09: standards

**Status:** ✅ DONE — Multiplier operational.

**Do NOT:**
- Hand-maintain TEMPLATE-STANDARDS-AUDIT.adoc (it's generated now)
- Let generated files go stale (they have --check variants)

**Use:**
- Reusable workflows (governance, codeql, hypatia, changelog, deno, elixir)
- Generation scripts (build-registry.sh, build-scorecards.sh, verify-claims.sh)

### DEP-10: proven

**Status:** ✅ DONE — Foundation solid.

**Use as:**
- Idris2 proof foundation for paint-type
- ABI definitions (`src/abi/Ephapax/…` — 17 files)
- Formal proofs (`src/formal/Ephapax/…`)

**Build pipeline:** Idris2 → RefC → C → Zig FFI

### DEP-11: echo-types

**Status:** Todo

**Tasks:**
- Keep Agda build env working
- Audit-and-record echo-types for new proofs

**Watch for:** `~/.agda/libraries` path-drift

### DEP-12: panic-attack

**Status:** In Progress

**2026-07-26 additions:**
- ✅ `panic-attack.toml` created for Ephapax
- ✅ `panic-attack.yml` CI workflow created for Ephapax

**Task:**
- Keep the `assail` gate green in paint-type CI

**Patterns to flag:**
- Critical: `unsafe`, `unwrap`, `expect`, `panic!` (Rust)
- Critical: `believe_me`, `assert_total`, `postulate` (Idris2)
- High: `TODO`, `FIXME`, `XXX`, `HACK`, `unimplemented!`
- Info: `Admitted.`, `admit.` (Coq — allowed in formal/*.v)

### DEP-13: Container / Supply-Chain Stack

**Status:** Todo

**Tasks:**
- Signed container build pipeline (Wolfi + ML-DSA-87)
- cerro-torre ML-DSA integration
- Svalinn gateway + Vordr runtime wiring

### DEP-14: Hypatia (Hexadeca source)

**Status:** Todo

**Tasks:**
- Keep Hexadeca 16-protocol reference canonical
- Keep paint-type connector copy conformant

### DEP-15: Toolchain & Formats

**Status:** In Progress

**Tasks:**
- Nix→Guix migration (estate-wide)
- Pin provers (Idris2 0.8.0, Lean4 4.13.0, Agda 2.8.0, Coq, TLA+)
- Reconcile rsr-template drift against standards canon

---

## Bot Protocol for DEPENDENCY-SCHEDULER.adoc

### When a DEP Task Completes

1. **Update the DEP issue file** in `.github/ISSUES/DEP-NN.adoc`:
   - Mark task as completed
   - Update status if all critical tasks done
   - Update last modified date

2. **Update DEPENDENCY-SCHEDULER.adoc:**
   - Tick the task in the make-great tasks section
   - Update the Status column if DEP transitions to Done
   - Update the Readiness column if changed

3. **Update DEP-IMPLEMENTATION-STATUS.adoc:**
   - Update the detailed status section
   - Update summary statistics
   - Verify acceptance criteria

4. **Update DEP-AUTOMATION-TRACKING.adoc:**
   - Update automation coverage if new automation added
   - Verify drift detection is in place

5. **Sync with GitHub Project #41** (if accessible):
   - Move the matching DEP-NN item on the kanban

### When Starting a New DEP Task

1. **Read the DEP issue file** first
2. **Check for blockers** (especially DEP-03)
3. **Verify automation** is in place before making changes
4. **Update the task** to In Progress in the issue file
5. **Create a branch** with naming: `dep-nn-task-description`

### IDs Are Stable

- DEP-NN IDs are **stable** — reference them in commits, issues, and bot prompts
- Do NOT rename or renumber DEPs
- Always reference the DEP number in commit messages

---

## Proof Handling Protocol

### For Coq Proofs (Ephapax)

1. **Before adding `Admitted.`:**
   - Document in `PROOF-NEEDS.md` §4
   - Document in `formal/L1-ELIMINATOR-FORK.md` if research-related
   - Update proof count in PROOF-NEEDS.md

2. **After adding `Admitted.`:**
   - Run `./scripts/status-gate.sh --proofs` to verify count sync
   - Verify `grep -R --include='*.v' -n '^[[:space:]]*Admitted\.' formal` matches PROOF-NEEDS.md

3. **When closing an admit:**
   - Remove from PROOF-NEEDS.md
   - Update proof count
   - Run status-gate to verify

4. **Research admits:**
   - DEP-01 has 2 open research admits (L1 eliminator fork)
   - These are **documented research questions**, not implementation tasks
   - Do NOT attempt to close them without the choreographic typing experiment

### For Idris2 Proofs (proven)

1. **NEVER use:**
   - `believe_me`
   - `assert_total`
   - `postulate`

2. **Always use REAL proofs**

3. **Verify:**
   - `idris2 --check --total paint-type.ipkg` passes
   - No warnings from Idris2

---

## Machine-Readable Metadata

### a2ml Files

paint-type uses **a2ml** (Agent Machine Language) for machine-readable metadata:

| File | Purpose | Location |
|------|---------|----------|
| 0-AI-MANIFEST.a2ml | Repository-level AI manifest | Root |
| 0.1-AI-MANIFEST.a2ml | Docs-level manifest | docs/ |
| REGISTRY.a2ml | Repository registry | .machine_readable/ |
| STATE.a2ml | Repository state | .machine_readable/ |
| ECOSYSTEM.a2ml | Ecosystem metadata | .machine_readable/ |
| META.a2ml | Metadata | .machine_readable/ |

### Reading a2ml

```bash
# List all a2ml files
find . -name "*.a2ml" -type f

# Read a specific a2ml file
cat 0-AI-MANIFEST.a2ml

# Validate a2ml syntax (if validator available)
a2ml-validate 0-AI-MANIFEST.a2ml 2>/dev/null || echo "Validator not available"
```

### Using Machine-Readable Data

**For task prioritization:**
- Check `0-AI-MANIFEST.a2ml` for priority hints
- Check `DEPENDENCY-SCHEDULER.adoc` for current DEP status

**For automation:**
- Check `DEP-AUTOMATION-TRACKING.adoc` for CI/CD coverage
- Check `.github/workflows/` for existing workflows

---

## Error Handling

### When Encountering Errors

1. **Read the error message carefully**
2. **Check if it's a known issue:**
   ```bash
   grep -r "error message text" .github/ISSUES/ 2>/dev/null
   grep -r "error message text" docs/ 2>/dev/null
   ```
3. **Check PROOF-NEEDS.md** for admitted proofs
4. **Check TEST-NEEDS.adoc** for known test gaps
5. **If stuck:**
   - Document the error in a temporary file
   - Ask for clarification with full context
   - Include: command, error message, file location, line number

### Common Error Patterns

| Error | Likely Cause | Solution |
|-------|--------------|----------|
| `Admitted.` not documented | Missing PROOF-NEEDS.md entry | Add to PROOF-NEEDS.md §4 |
| Proof count mismatch | status-gate.yml failure | Update PROOF-NEEDS.md or close admits |
| `believe_me` found | Idris2 proof hole | Replace with REAL proof |
| `unsafe` found | Rust unsafe block | Justify with `// SAFETY:` or remove |
| `unwrap()` found | Rust panic risk | Replace with proper error handling |
| Build failed | Dependency issue | Check tool versions, run `just clean` |

---

## Reporting

### End-of-Session Report

When completing a session, provide a **structured report**:

```markdown
## Session Summary — YYYY-MM-DD

### Time
- Start: HH:MM UTC
- End: HH:MM UTC
- Duration: X hours Y minutes

### Tasks Completed
1. [DEP-01] Updated panic-attack.toml configuration
2. [DEP-01] Created panic-attack.yml CI workflow
3. [DEP-12] Verified assail gate is green

### Tasks In Progress
1. [DEP-01] Researching L1 eliminator fork (2 admits remaining)
2. [DEP-05] Integrating proven-pqc ML-DSA

### Tasks Blocked
1. [DEP-03] Cannot proceed — no working AffineScript compiler

### Files Modified
- `hyper-repos/_LANGUAGES _SET/_NEXTGEN_LANGUAGES _SET/ephapax/panic-attack.toml` (created)
- `hyper-repos/_LANGUAGES _SET/_NEXTGEN_LANGUAGES _SET/ephapax/.github/workflows/panic-attack.yml` (created)
- `meta-repos/paint-type/DEPENDENCY-SCHEDULER.adoc` (updated)

### Verification
- [x] Build passes (`just build`)
- [x] Tests pass (`just test`)
- [x] Type checking passes (`just typecheck`)
- [x] Proof count in sync (`status-gate.sh --proofs`)
- [x] panic-attack scan clean

### Issues Found
1. None

### Next Steps
1. Continue research on L1 eliminator fork
2. Audit Gossamer's Ephapax integration
3. Create gossamer#69 design document
```

### Structure Matters

- Use **clear section headers**
- Use **checklists** for verification
- Reference **file paths** (absolute if possible)
- Reference **DEP numbers**
- Include **commands used**

---

## Best Practices

### Do

✅ **Always read first:** DEPENDENCY-SCHEDULER.adoc, DEP issue files  
✅ **Verify before committing:** Build, tests, type checking  
✅ **Document admits:** Update PROOF-NEEDS.md when adding `Admitted.`  
✅ **Use machine-readable:** Check .machine_readable/ for structured data  
✅ **Reference DEP numbers:** In commits, issues, and reports  
✅ **Check for blockers:** Especially DEP-03 before starting work  
✅ **Keep generated files current:** They have --check variants  

### Don't

❌ **Don't start on BLOCKED items:** DEP-03 has no working compiler  
❌ **Don't hand-maintain generated files:** TEMPLATE-STANDARDS-AUDIT.adoc is generated  
❌ **Don't add `believe_me`:** Always use REAL proofs  
❌ **Don't use `unsafe` without justification:** Always add `// SAFETY:` comment  
❌ **Don't break CI:** Always verify before pushing  
❌ **Don't duplicate work:** Check what's already done in DEP issue files  
❌ **Don't ignore drift:** Always check --check variants  

---

## Quick Reference Commands

### Essential Commands

```bash
# Build
just build

# Test
just test

# Type check
just typecheck

# Verify FFI
just verify-ffi

# Clean
just clean

# Proof count check (Ephapax)
./scripts/status-gate.sh --proofs

# panic-attack scan (Ephapax)
panic-attack assail --config panic-attack.toml

# Registry check (standards)
./scripts/build-registry.sh --check

# Scorecards check (standards)
./scripts/build-scorecards.sh --check

# Verify manifests (standards)
./scripts/verify-manifests.sh
```

### Grep Commands

```bash
# Find admits (Coq)
grep -R --include='*.v' -n '^[[:space:]]*Admitted\.' formal

# Find believes (Idris2)
git grep -n "believe_me\|assert_total\|postulate" src/

# Find unsafe (Rust)
git grep -n "unsafe" src/

# Find unwrap/expect (Rust)
git grep -n "\.unwrap\|\.expect" src/

# Find panic (Rust)
git grep -n "panic!" src/

# Find TODO/FIXME
git grep -n "TODO\|FIXME\|XXX\|HACK" src/
```

### File Locations

```bash
# DEP files
ls meta-repos/paint-type/.github/ISSUES/DEP-*.adoc

# Tracking files
ls meta-repos/paint-type/DEP-*.adoc

# Ephapax files
hyper-repos/_LANGUAGES _SET/_NEXTGEN_LANGUAGES _SET/ephapax/

# Standards files
hyper-repos/standards/

# Proven files
hyper-repos/proven/
```

---

## Resources

### In-Repo Documentation

| Document | Purpose | Priority |
|----------|---------|----------|
| README.adoc | Project overview | Must read first |
| DEPENDENCY-SCHEDULER.adoc | Current priorities | Must read first |
| DEP-IMPLEMENTATION-STATUS.adoc | Detailed DEP status | Must read for DEP work |
| DEP-AUTOMATION-TRACKING.adoc | Automation coverage | Reference for automation |
| ROADMAP.adoc | Milestone plan | Reference |
| PROOF-NEEDS.md | Proof requirements | Reference for proofs |
| PROOF-STATUS.adoc | Proof status | Reference for proofs |

### External Resources

| Resource | URL | Purpose |
|----------|-----|---------|
| GitHub Issues | https://github.com/metadatastician/paint-type/issues | Issue tracking |
| GitHub Discussions | https://github.com/metadatastician/paint-type/discussions | General discussion |
| hyperpolymath estate | https://github.com/hyperpolymath | Upstream repositories |
| nextgen-languages | https://github.com/hyperpolymath/nextgen-languages | Language disambiguation |
| standards repo | https://github.com/hyperpolymath/standards | CI/CD reusables |
| panic-attack | https://github.com/hyperpolymath/panic-attack | Dangerous pattern detection |

---

## Glossary

| Term | Definition |
|------|------------|
| DEP | Dependency Epic — critical upstream dependency |
| CRG | Component Readiness Grade — A/B/C/D/Unknown/Missing |
| P1/P2/P3 | Priority — P1 = critical path, P2 = important, P3 = nice to have |
| Maintain | Priority — ongoing maintenance, not a project |
| RSR | Really Simple Repository standard |
| a2ml | Agent Machine Language — machine-readable metadata format |
| BIQ | Build Integration Quality |
| LWW | Last-Write-Wins (CRDT strategy) |
| ML-DSA | Multi-Layered Digital Signature Algorithm |
| FFI | Foreign Function Interface |
| ABI | Application Binary Interface |
| IPC | Inter-Process Communication |
| WASM | WebAssembly |

---

*This guide was last updated on 2026-07-26. Always refer to the in-repo documentation for the most current information.*

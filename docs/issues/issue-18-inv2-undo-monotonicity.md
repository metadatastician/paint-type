# GitHub Issue #18 - INV-2 Undo Graph Monotonicity

## ✅ INV-2 Undo Graph Monotonicity — PROVEN / DONE

**Statement:** History only grows; no silent discard. CRDT merge must preserve the no-silent-discard property.

### Proof Status: COMPLETE

All four contract clauses of the monotonicity invariant have been formally proven:

| Clause | Statement | Prover | File | Status |
|--------|-----------|--------|------|--------|
| 1 | Length monotonic non-decreasing (grows by exactly 1 per commit) | Lean4 | `verification/proofs/lean4/UndoGraph.lean` | ✅ Done |
| 2 | Old revisions survive (append-only, existing entries never mutated) | Lean4 | `verification/proofs/lean4/UndoGraph.lean` | ✅ Done |
| 3 | Parent edges immutable & point strictly to lower IDs | Lean4 | `verification/proofs/lean4/UndoGraph.lean` | ✅ Done |
| 4 | Ancestry acyclic and terminates at root | Lean4 | `verification/proofs/lean4/UndoGraph.lean` | ✅ Done |

### Re-verification Under Concurrency: COMPLETE

The no-silent-discard property has been re-verified under concurrent commits:

| Verification | Prover | File | Status |
|--------------|--------|------|--------|
| CRDT merge preserves monotonicity | Agda | `verification/proofs/agda/TileCRDT.agda` | ✅ Done |
| Session delivery monotonicity | TLA+ | `verification/proofs/tlaplus/BurbleSession.tla` | ✅ Done |

**Proof Date:** 2026-06-14 (Lean4 core proof)  
**Re-verification Date:** 2026-07-25 (Agda + TLA+ under v0.5.0 collaboration)  
**Verification Commands:**
```bash
# Lean4 proof (pure core, no dependencies)
lean verification/proofs/lean4/UndoGraph.lean

# Agda proof (builtin-only)
agda --no-libraries verification/proofs/agda/TileCRDT.agda

# TLA+ model checking
tlc verification/proofs/tlaplus/BurbleSession.tla
```

### Implementation

- **Rust implementation:** `src/paint_core/src/undo.rs` (UndoGraph<T>, RevId)
- **FFI bindings:** `src/interface/ffi/src/main.zig`
- **Introduced in:** PR #21 (2026-06-01)
- **Wrapped up in:** Issue #24 (v0.2.0 completion)

### Re-verification Context (v0.5.0 Collaboration)

The INV-2 invariant was re-verified under the v0.5.0 collaboration layer to confirm that CRDT merge operations preserve the no-silent-discard property. This addresses the requirement that "CRDT merge must preserve the no-silent-discard property" under concurrent commits.

**Key properties verified:**
- `⊔-upper-l` / `⊔-upper-r` lemmas in Agda show merged cell DOMINATES both inputs
- `Monotone` property in TLA+ ensures peer's applied-set only GROWS
- No committed mutation is ever silently discarded during merge

### Commits

- `37dbf86` feat(ephapax,ffi): pt_tile_write_pixel + non-uniform composite_over + UndoGraph (#21)
- `3a3894e` docs(status): v0.2.0 wrap-up — reflect 4 PRs of compositing/undo/layer/proof (#24)
- `e4132c5` feat(v0.3.0): shell slice — FFI layer stack, ptype_format, Gossamer shell, UI, brand (#13) (#98)
- `66d872e` feat(v1.0.0): Start SEC-1/2 security proofs for plugin system

### References

- PROOF-NEEDS.adoc: INV-2 marked as **Done**
- PROOF-STATUS.adoc: INV-2 listed as completed 2026-06-14
- docs/architecture/COLLABORATION.adoc: INV-2 re-verification documented
- ROADMAP.adoc: Non-destructive undo graph (persistent, O(1) branch) — PR #21

---

**Action for GitHub:** 
- Mark this issue as **DONE** / **COMPLETE**
- Add label: `proof complete`
- Close issue with comment referencing this file and the proof artifacts

**Note:** The `proof complete` label has been added to `.github/settings.yml` and will be automatically available in the repository once the settings sync.

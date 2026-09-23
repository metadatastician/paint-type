<!--
SPDX-License-Identifier: AGPL-3.0-or-later
SPDX-FileCopyrightText: 2024-2026 Joshua Jewell <paint-type@pm.me>
-->
<!-- Hand-authored notes may go ABOVE or BELOW the generated region. -->
<!-- The region between the ARRIVAL-PACK markers is normally generated from this
     repo's a2ml by `just claude-md`. It is HAND-AUTHORED here because that
     generator cannot run: see the note at the end of this file. -->

<!-- ARRIVAL-PACK:BEGIN — hand-authored 2026-09-23; regenerate with `just claude-md` once the descriptiles parse -->

# You are in paint.type — orient before acting

`paint.type` is the **project** name; `paint-type` is the **GitHub repository**
name (GitHub does not allow dots). Use `paint.type` in prose.

If you are unsure what something is, **read the canon; do not guess.**

## Read these first, in this order

1. `paint-type_chora.deed` — the repo deed: identity, clade, forges, lineage, status.
2. `README.adoc` — what this is and where it is going.
3. `EXPLAINME.adoc` — how it is built and what the evidence is.
4. `AFFIRMATION.adoc` — what was true and checkable at a stamped instant.
5. `.machine_readable/descriptiles/STATE.a2ml` — current position. **Caution:** see
   the defect note below; this file does not currently parse.

## Doctrine

1. **Holes before anything else** — fix soundness holes before features, perf or docs.
2. **Fixes first, on firm foundations** — ground-truth by running the tool, not by
   trusting status docs. This repo has shipped stale counts before (see below).
3. **Fail loudly, seal soundly** — no silent green; ABI/FFI seams sealed and proven.
4. **Distrust the neural for exactness** — licences, invariants and equivalence
   belong to Idris2, not to an LLM.
5. **Squabble, don't bypass** — reach green by satisfying the gate, never by
   admin-override or `continue-on-error`.
6. **No automated licence edits, ever** — manual, owner-only; third-party untouchable.
7. **No deletion by access-recency** — cold is not disposable.
8. **Wire first** — unwired is not done.
9. **Always sign** commits.
10. **Report faithfully — no overclaim** (the AFFIRMATION ethos).

## Licence — this repo is NOT the estate default

Code is **AGPL-3.0-or-later**, docs are **CC-BY-SA-4.0**. The root `LICENSE` and
`CLADE.a2ml` both say so. `rsr-template-repo`'s AI rules say "MPL-2.0, never
AGPL-3.0" — **that rule is false for this repo; do not "fix" the SPDX headers.**

## Architecture

| Layer | Language | Where |
|---|---|---|
| Image core (Ephapax) | Rust, linear types | `src/paint_core/` |
| ABI (dependent types) | Idris2 | `src/interface/Abi/` |
| FFI bridge (C ABI) | Zig → `libpt.a` | `src/interface/ffi/` |
| Browser bridge | AffineScript → typed-wasm | `src/bridges/` |
| Collaboration | CRDT tiles, Burble, Groove | `src/paint_collab/` |
| File format | `.ptype` | `src/ptype_format/` |
| Desktop shell | Gossamer (vendored) | `third_party/gossamer/` |
| Proofs | Idris2 | `verification/proofs/idris2/` |

**The build order is load-bearing.** `src/paint_core/build.rs` shells out to
`zig build` and links `-lpt`. Without Zig on `PATH` (or `PT_LIB_DIR` set),
`cargo test` fails at link time with `cannot find -lpt`. Build Zig first:

```
cd src/interface/ffi && zig build && zig build test
cargo test --manifest-path src/paint_core/Cargo.toml
```

Measured 2026-09-23 on rustc 1.85.1 / Zig 0.15.1 / Idris2 0.8.0: **23/23** Zig
tests, **104** Rust tests, and **8 of 9** Idris2 proof modules type-check.
Toolchain pins are in `.tool-versions` and `mise.toml`.

## Known defects — do not make these worse

* **`META.a2ml`, `ECOSYSTEM.a2ml`, `STATE.a2ml` parse as nothing.** They are
  hand-written s-expressions; `deed_lint` rejects them (`invalid doc-head 'meta'`)
  and `parse_a2ml` rejects them as TOML. Write new metadata in real a2ml/TOML.
  Consequence: `paint-type_chora.deed` carries only identity/clade/forges/
  lineage/status — no `(manifest …)` clause and no `(ply …)` tree.
* **269 of 340 `.a2ml` files are Markdown prose** with no recognised identity.
  `.github/workflows/verify-manifests.yml` only checks that files *exist* at the
  right depth and name, so it cannot see this.
* **`verification/proofs/idris2/ABI/Pointers.idr` does not type-check**, on
  `handlePtrEq` alone. The other eight modules do. Do not claim "all proofs pass".
* **`verification/proofs/` carries `agda/`, `lean4/` and `tlaplus/`** against a
  language policy that permits Idris2 alone. Write new proofs in `idris2/`.
* **`session/custom-checks.self-validating` is YAML, not Nickel**, under a name
  the root allowlist does not list. `coordination.k9.ncl` now exists as a
  faithful Nickel conversion of `coordination.self-validating`, but the YAML is
  still present because `session/dispatch.sh` may read it.
* **Stale counts have shipped here before.** `READINESS.adoc`, `README.adoc` and
  `EXPLAINME.adoc` all claimed `cargo test 98/98` and `zig 29/29`; the measured
  figures are **104** and **23/23**. Re-measure; never copy a number forward.
* **`container/Containerfile` is an unfinished template.** It still carries ten
  unsubstituted `{{PLACEHOLDER}}` tokens, so `just container-build` cannot work.
  The root `Containerfile` is further along but also has TODO build steps.

## Build

Every task runs through `just`. The root `Justfile` is ~109 recipes and is meant
to be thin, delegating to `build/just/*.just` — that split is not done, and
there is no `build/` directory yet.

<!-- ARRIVAL-PACK:END -->

---

**Why this file is hand-authored.** `rsr-template-repo` generates `CLAUDE.md`
from the repo's descriptiles via `just claude-md`. That generator reads
`CLADE.a2ml`, `ECOSYSTEM.a2ml`, `AGENTIC.a2ml`, `STATE.a2ml` and `ANCHOR.a2ml`
and records their content hashes in the marker above. Three of those files do
not parse in any format the estate tooling accepts, so the generator cannot run.
Once `META`/`ECOSYSTEM`/`STATE` are rewritten as real a2ml/TOML, regenerate this
file and let the machine own the region between the markers.

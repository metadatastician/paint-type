// SPDX-License-Identifier: CC-BY-SA-4.0
// AGENTS.md — Canonical instructions for all coding agents working on paint-type
//
// Source of Truth: This file overrides any default agent behavior.
// Last Updated: 2026-07-26
// Maintainer: hyperpolymath (estate owner)
//
// FOR ALL AGENTS: Read this file FIRST on every session start.
// This file is specifically for paint-type repository. For estate-wide
// instructions, see /home/hyperpolymath/developer/AGENTS.md

= paint-type AGENTS.md — Canonical Agent Instructions

**IMPORTANT:** This file contains **overridable defaults** for the paint-type repository.
The estate-wide `/home/hyperpolymath/developer/AGENTS.md` contains non-overridable
critical instructions. Both apply, with this file taking precedence for paint-type.

== Session Startup Protocol

On EVERY session start, perform these steps in order:

. Read `/home/hyperpolymath/developer/AGENTS.md` (estate-wide, non-overridable)
. Read THIS file (`paint-type/AGENTS.md`)
. Read `DEPENDENCY-SCHEDULER.adoc` — current priorities
. Read `DEP-IMPLEMENTATION-STATUS.adoc` — detailed DEP status
. Read `DEP-AUTOMATION-TRACKING.adoc` — automation coverage
. Read `ROADMAP.adoc` — milestone plan

**Time limit:** This startup reading should take < 2 minutes. Memorize, don't dwell.

== Critical Instructions (Cannot Be Overridden)

These are **never overridable**, even by this file:

- **Blast radius awareness:** Treat `git push`, `git reset --hard`, `git clean -fd`,
  `rm -rf`, migrations, deploys, and production API calls with extreme care.
  Always ask for explicit authorization.
- **Read before edit:** Never edit a file you have not read in this session.
- **No emoji:** Never use emoji in code, commits, or documentation.
- **No author headers:** Never add author or license headers to files.
- **Minimal diff:** Change only what is necessary. Remove completely when removing.
- **Match existing style:** Indentation, naming, error handling density.
- **Prove it works:** Tests pass, code runs, acceptance criteria met.

== Repository-Specific Overrides

=== Working Directory

**ALWAYS work from:** `/home/hyperpolymath/developer/meta-repos/paint-type/`

```bash
cd /home/hyperpolymath/developer/meta-repos/paint-type
```

=== Read Before You Act

**MUST READ (in this order):**

[cols="1,3"]
|=== 
| Priority | File |
| 1 | `DEPENDENCY-SCHEDULER.adoc` |
| 2 | `DEP-IMPLEMENTATION-STATUS.adoc` |
| 3 | `DEP-AUTOMATION-TRACKING.adoc` |
| 4 | `.github/ISSUES/DEP-*.adoc` (for the DEP you're working on) |
| 5 | `ROADMAP.adoc` |
| 6 | `PROOF-NEEDS.adoc` |
| 7 | `PROOF-STATUS.adoc` |
|=== 

**Then read the specific file(s) you will modify.**

=== Do Not Start Here

**BLOCKED DEPs — DO NOT START WORK:**

[cols="1,3,1"]
|=== 
| DEP | Reason | Verification |
| DEP-03 | No working AffineScript compiler | `git-reticulator/PROOF-NEEDS.md` |
|=== 

**Before starting ANY DEP work, verify it's not BLOCKED in DEPENDENCY-SCHEDULER.adoc.**

=== Current Working Set (2026-07-26)

Focus on these DEPs when no specific task is given:

[cols="1,3,1,1"]
|=== 
| DEP | Task | Priority | Status |
| DEP-01 | Research: L1 eliminator fork (2 admits) | P1 | In Progress |
| DEP-01 | Verify Gossamer integration is load-bearing | P1 | In Progress |
| DEP-01 | Service-handle region modelling (gossamer#69) | P1 | In Progress |
| DEP-05 | Full proven-pqc integration | P2 | In Progress |
| DEP-12 | Keep panic-attack assail gate green | Maintain | In Progress |
| DEP-15 | Nix→Guix migration | P1 | In Progress |
|=== 

== Task Prioritization

When the user says "what should I work on?" or similar, use this order:

. **P1 + BLOCKED that can be unblocked** — None currently (DEP-03 cannot be unblocked)
. **P1 + In Progress** — DEP-01, DEP-15
. **P1 + Todo** — None currently
. **P2 + In Progress** — DEP-05, DEP-12
. **STAND items** — STAND-01, STAND-02

**Report structure:**
```
Current working set (P1 In Progress):
1. DEP-01 Ephapax — 2 open research admits (L1 eliminator fork)
2. DEP-15 Toolchain — Nix→Guix migration

Next in queue (P2 In Progress):
1. DEP-05 cerro-torre — proven-pqc integration
2. DEP-12 panic-attack — keep assail gate green

BLOCKED (do not start):
1. DEP-03 AffineScript — no working compiler
```

== Automation Protocol

=== Before Every Commit

**ALWAYS run (in this order):**

```bash
# 1. Build
just build

# 2. Tests
just test

# 3. Type checking
just typecheck

# 4. FFI verification
just verify-ffi

# 5. Proof count (if Coq files changed)
./scripts/status-gate.sh --proofs

# 6. panic-attack (Ephapax only)
cd /home/hyperpolymath/developer/hyper-repos/_LANGUAGES _SET/_NEXTGEN_LANGUAGES _SET/ephapax
panic-attack assail --config panic-attack.toml
cd /home/hyperpolymath/developer/meta-repos/paint-type
```

**If ANY of these fail, do NOT commit.** Fix the failure first.

=== Before Pushing

**ALWAYS:**

. Verify all of the above pass
. Check `git status` — no unintended files
. Check `git diff` — only intended changes
. Ask: "Can I push this?" if uncertain about blast radius

== Proof Handling

=== Coq Proofs (Ephapax)

**NEVER add `Admitted.` without:**

. Documenting it in `PROOF-NEEDS.md` §4
. Documenting it in the appropriate research file
. Updating the proof count

**After adding `Admitted.`:**

```bash
# Verify count sync
./scripts/status-gate.sh --proofs

# Manual check
grep -R --include='*.v' -n '^[[:space:]]*Admitted\.' formal | wc -l
# Should match count in PROOF-NEEDS.md §4
```

**Research admits (DEP-01):**

The 2 open admits in `formal/Semantics_L1.v` are **documented research questions**:

1. Line 3318: `step_pop_disjoint_from_type_l1`
2. Line 3337: `preservation_l1` (gated on step_pop)

**DO NOT attempt to close these** without:
- Reading `formal/L1-ELIMINATOR-FORK.md`
- Performing the choreographic typing experiment described in §6
- Confirming the deciding experiment outcome

=== Idris2 Proofs (proven)

**NEVER use:**

- `believe_me`
- `assert_total`
- `postulate`

**ALWAYS:**

. Use REAL proofs only
. Verify with `idris2 --check --total paint-type.ipkg`
. No warnings accepted

== File Modification Rules

=== Files You Can Modify

[cols="1,3,1"]
|=== 
| Category | Examples | Permission |
| Documentation | *.adoc, *.md in docs/ | ✅ Yes |
| DEP tracking | DEP-*.adoc, .github/ISSUES/DEP-*.adoc | ✅ Yes |
| Configuration | Justfile, *.toml, *.yml, *.json | ✅ Yes (carefully) |
| Scripts | scripts/*.sh | ✅ Yes |
| Source code | src/**/*.rs, src/**/*.zig, src/**/*.idr | ⚠️ Only with task |
| Formal proofs | formal/*.v | ⚠️ Only with research task |
|=== 

=== Files You Should NOT Modify

[cols="1,3,1"]
|=== 
| Category | Examples | Reason |
| Generated files | TEMPLATE-STANDARDS-AUDIT.adoc | Generated from live tree |
| Upstream repos | hyper-repos/* | Separate repositories |
| Binary artifacts | zig-out/, target/ | Build outputs |
| Git history | .git/ | Version control |
|=== 

=== When Modifying Source Code

**ALWAYS:**

. Have a specific, approved task
. Read the file AND its tests
. Read related files (callers, callees)
. Match existing style exactly
. Add tests for new functionality
. Update documentation if behavior changes

**NEVER:**

. Modify files without reading them first
. Remove tests
. Break existing functionality
. Add `unsafe` without `// SAFETY:` justification
. Use `unwrap()` or `expect()`

== DEP-Specific Instructions

=== DEP-01: Ephapax

**Live path:** `/home/hyperpolymath/developer/hyper-repos/_LANGUAGES _SET/_NEXTGEN_LANGUAGES _SET/ephapax/`

**Read first:**
- `.github/ISSUES/DEP-01-EPHAPAX.adoc`
- `formal/PRESERVATION-DESIGN.md`
- `formal/L1-ELIMINATOR-FORK.md`

**2026-07-26 automation:**
- `panic-attack.toml` — created
- `panic-attack.yml` — CI workflow created

**Open tasks:**
1. Verify Gossamer's Ephapax integration is load-bearing
2. Create gossamer#69 design document (service-handle region modelling)
3. Research: L1 eliminator fork (2 admits)

**Research note:** The 2 L1 admits require a **choreographic typing experiment**. Do not attempt to close them with traditional methods.

=== DEP-02: typed-wasm

**Status:** ✅ DONE — Maintenance only

**Do NOT:**
- Break `.twasm` schemas
- Change version without updating paint-type

**Maintenance (low priority):**
- Keep `tw`/`tw-verify` pinned
- Full #130 corpus
- D6 human-readable errors

=== DEP-03: AffineScript

**Status:** ❌ BLOCKED — **DO NOT TOUCH**

**Blocker:** No working compiler exists (confirmed in git-reticulator/PROOF-NEEDS.md).

**If asked about DEP-03:**
```
DEP-03 AffineScript is BLOCKED. There is no working compiler.
Evidence: git-reticulator/PROOF-NEEDS.md states "AffineScript has no working compiler yet"
Completed: ABI→.twasm generator (paint-type#39)
Blocked: Widen typed-wasm enforcement, compile UI chrome
Alternative: Use Ephapax directly for UI chrome (different architecture, significant redesign)
```

**DO NOT:**
- Attempt to build AffineScript
- Modify AffineScript source
- Create workarounds that duplicate done work

=== DEP-04: Gossamer

**Status:** ✅ DONE — Integration complete

**Live path:** `/home/hyperpolymath/developer/meta-repos/gossamer/`

**Verification (if requested):**
- Audit `wqbb3mhzf` for Idris2-ABI vs Zig-FFI drift
- Reconcile versioning (v0.1.0 tag vs v0.3.1 README)
- Nix→Guix migration
- Android integration

**Ephapax integration:**
- Verify is load-bearing (not just namechecked)
- Modules: Shell.eph, Bridge.eph, Capabilities.eph

=== DEP-05: cerro-torre

**Status:** In Progress

**Completed:**
- Graduated to standalone repo in meta-repos
- ML-DSA-87 capability surface added

**Remaining:**
- Full proven-pqc integration
- Interim signer decision

=== DEP-06 through DEP-15

See DEPENDENCY-SCHEDULER.adoc and DEP-IMPLEMENTATION-STATUS.adoc for details.

== Reporting Protocol

=== End-of-Session Report

**ALWAYS provide a structured report:**

```markdown
## Session Summary — YYYY-MM-DD HH:MM UTC

### Duration
X hours Y minutes

### Tasks Completed
- [DEP-01] Task description — result
- [DEP-01] Task description — result

### Files Modified
- `path/to/file1.ext` — change description
- `path/to/file2.ext` — change description

### Verification
- [x] `just build` — passed
- [x] `just test` — passed
- [x] `just typecheck` — passed
- [x] `status-gate.sh --proofs` — in sync
- [x] `panic-attack assail` — clean

### Blockers Found
- [ ] None
- [x] Blocker description — mitigation

### Next Steps
1. Next task
2. Next task
```

**NEVER say:**
- "Does this look good?"
- "Anything else?"
- "I'm done" (without verification)

**ALWAYS say:**
- What was done
- How it was verified
- What remains

=== Error Reports

When encountering errors, report with:

```
File: path/to/file.ext
Line: N
Command: the command that failed
Error: full error message
Context: surrounding code (5 lines before/after)
Action taken: what you tried
```

== Machine-Readable Metadata

=== a2ml Files

Use these for tooling and automation:

[cols="1,3"]
|=== 
| File | Purpose |
| 0-AI-MANIFEST.a2ml | Repository AI manifest |
| docs/0.1-AI-MANIFEST.a2ml | Docs-level manifest |
| .machine_readable/REGISTRY.a2ml | Repository registry |
| .machine_readable/STATE.a2ml | Repository state |
| .machine_readable/ECOSYSTEM.a2ml | Ecosystem metadata |
| .machine_readable/META.a2ml | Metadata |
|=== 

=== Using a2ml

**For task discovery:**
```bash
# List tasks from AI manifest
grep -E "^task[[:space:]]*=" 0-AI-MANIFEST.a2ml | head -20
```

**For priority:**
```bash
# Check priority hints
grep -E "^priority[[:space:]]*=" 0-AI-MANIFEST.a2ml
```

== Query Protocol

When the user asks a question, use this decision tree:

```
Is it a DEP status question?
  └─ YES → Read DEPENDENCY-SCHEDULER.adoc, DEP-IMPLEMENTATION-STATUS.adoc
      └─ Answer from those files
      
Is it a proof question?
  └─ YES → Read PROOF-STATUS.adoc, PROOF-NEEDS.md
      └─ Answer from those files
      
Is it a development question?
  └─ YES → Read EXPLAINME.adoc, OPERATIONAL-STATUS.adoc, ARCHITECTURE.md
      └─ Answer from those files
      
Is it a "what should I work on" question?
  └─ YES → Use Task Prioritization section above
      └─ Report current working set
      
Otherwise → Search in-repo documentation first
```

== Verification Commands

Memorize these. Run them frequently.

[cols="3,1"]
|=== 
| Command | Purpose |
| `just build` | Full build |
| `just test` | All tests |
| `just typecheck` | Idris2 type checking |
| `just verify-ffi` | FFI verification |
| `just clean` | Clean build artifacts |
| `./scripts/status-gate.sh --proofs` | Proof count verification |
| `panic-attack assail --config panic-attack.toml` | Dangerous pattern scan |
| `./scripts/build-registry.sh --check` | Registry drift check |
| `./scripts/build-scorecards.sh --check` | Scorecards drift check |
| `./scripts/verify-manifests.sh` | Manifest verification |
|=== 

== Grep Commands

Use these for code analysis:

[cols="3,1"]
|=== 
| Command | Purpose |
| `git grep -n "Admitted\." formal/` | Find Coq admits |
| `git grep -n "believe_me\|assert_total\|postulate" src/` | Find Idris2 holes |
| `git grep -n "unsafe" src/` | Find Rust unsafe |
| `git grep -n "\.unwrap\|\.expect" src/` | Find Rust panics |
| `git grep -n "panic!" src/` | Find Rust panics |
| `git grep -n "TODO\|FIXME\|XXX\|HACK" src/` | Find tech debt |
|=== 

== Commit Message Format

Use Conventional Commits format:

```
type(scope): subject

body (optional)

footer (optional: Closes #123)
```

**Types:** feat, fix, docs, style, refactor, test, chore, revert

**Examples:**
```
feat(paint_core): implement multiply blend mode

Implements the multiply compositing operator for tile blending.

Closes #456

---

fix(ffi): correct pointer alignment in TileLayout

Fixes alignment issue that caused FFI boundary corruption.

---

docs(readme): update status badges and v0.3.0 milestone

Updates README.adoc to reflect v0.3.0 Desktop Shell closure.

---

refactor(composite): optimize over operator

Reduces allocations in Porter-Duff over by 40%.

---

chore(ci): add panic-attack workflow to Ephapax

Adds dangerous pattern detection CI for Ephapax repository.
```

**NEVER:**
- Use emoji in commit messages
- Use vague messages like "fix stuff" or "update things"
- Forget to reference issue numbers

== Branch Naming

Use this format: `type/scope-description`

**Examples:**
```
feat/dep-01-l1-eliminator-research
fix/dep-04-version-reconciliation
 docs/dep-tracking-automation
docs/wiki-architecture-page
chore/ci-panic-attack-integration
```

== Communication Protocol

=== Be Direct

**Do:**
```
I read DEPENDENCY-SCHEDULER.adoc. DEP-03 is BLOCKED by lack of working AffineScript compiler. The other spine DEPs (01, 02, 04) are In Progress or Done. DEP-01 has 2 open research admits that require choreographic typing. Do you want me to work on the gossamer#69 design document for DEP-01?
```

**Don't:**
```
Hey! So I was looking at the DEPs and there's this one that seems blocked? Maybe we should do something about it? What do you think?
```

=== Be Structured

Use markdown formatting:
```markdown
## Current Status

- **DEP-01:** In Progress (2 research admits)
- **DEP-02:** DONE
- **DEP-03:** BLOCKED (no compiler)
- **DEP-04:** DONE

## Blockers

None that can be resolved this session.

## Recommendation

Focus on DEP-01: create gossamer#69 design document for service-handle region modelling.
```

=== Be Concise

**Target:** < 150 words for most responses.

**Exception:** Complex explanations or multi-step plans may need more.

== File Creation Rules

**NEVER create:**
- Documentation files (*.md, *.adoc) unless explicitly requested
- License headers
- Author headers
- Emoji in files

**ONLY create in repo:**
- Files explicitly requested by the user
- Tests for features you implemented
- Configuration files needed for automation

**Use scratchpad for:**
- Temporary prototypes
- Working notes
- Intermediate results

**Scratchpad location:** `/tmp/vibe-scratchpad-*/` (provided at session start)

== Open Questions Protocol

When genuinely ambiguous (one question per session max):

```
The DEPENDENCY-SCHEDULER.adoc lists DEP-01 as In Progress with 2 open research admits. The DEP-01-EPHAPAX.adoc file documents these as requiring a choreographic typing experiment. Should I:

1. [Recommended] Create the gossamer#69 design document for service-handle region modelling (DEP-01 task 3)
2. Attempt to understand the L1 eliminator fork research (DEP-01 tasks 1-2)
3. Something else (please specify)
```

**Format:**
- State what you read
- Ask one specific question
- Provide 2-4 options
- Mark the recommended option

**NEVER:**
- Ask more than one question per session
- Ask without stating what you read
- Ask vague questions like "what should I do?"

== Tool Usage

=== Prefer Dedicated Tools

[cols="2,1,1"]
|=== 
| Task | Use | Not |
| Read file | read_file | cat, head, tail |
| Search | grep | find, rg, ag |
| Edit file | edit | sed -i, awk |
| Create file | write_file | echo >, touch |
|=== 

=== Bash Usage

**Always:**
- Use absolute paths
- Set timeouts
- Capture output

**Example:**
```bash
bash --command="cd /home/hyperpolymath/developer/meta-repos/paint-type && just build 2>&1", timeout=120
```

**Never:**
```bash
bash --command="cd paint-type && just build"  # Relative path
bash --command="just build"  # No timeout, no cd
```

== References

=== In-Repo (Read These First)

[cols="1,3"]
|=== 
| File | Purpose |
| DEPENDENCY-SCHEDULER.adoc | Current DEP priorities |
| DEP-IMPLEMENTATION-STATUS.adoc | Detailed DEP status |
| DEP-AUTOMATION-TRACKING.adoc | Automation coverage |
| ROADMAP.adoc | Milestone plan |
| README.adoc | Project overview |
| PROOF-NEEDS.adoc | Proof requirements |
| PROOF-STATUS.adoc | Proof status |
| EXPLAINME.adoc | Repository structure |
| OPERATIONAL-STATUS.adoc | Component architecture |
|=== 

=== Wiki Pages

[cols="1,3"]
|=== 
| Page | Purpose |
| docs/wiki/Home.md | Project orientation hub |
| docs/wiki/Architecture.md | Architecture overview |
| docs/wiki/DEPs.md | DEP status summary |
| docs/wiki/Development.md | Development guide |
| docs/wiki/Agent-Guide.md | Agent-specific instructions |
|=== 

=== Upstream

[cols="1,2,1"]
|=== 
| Repository | URL | DEP |
| ephapax | https://github.com/hyperpolymath/ephapax | DEP-01 |
| typed-wasm | https://github.com/hyperpolymath/typed-wasm | DEP-02 |
| affinescript | https://github.com/hyperpolymath/affinescript | DEP-03 |
| gossamer | https://github.com/hyperpolymath/gossamer | DEP-04 |
| standards | https://github.com/hyperpolymath/standards | DEP-09 |
| proven | https://github.com/hyperpolymath/proven | DEP-10 |
| panic-attack | https://github.com/hyperpolymath/panic-attack | DEP-12 |
|=== 

== Glossary

[cols="1,3"]
|=== 
| Term | Definition |
| DEP | Dependency Epic — critical upstream dependency |
| CRG | Component Readiness Grade |
| P1/P2/P3 | Priority levels |
| Maintain | Ongoing maintenance priority |
| RSR | Really Simple Repository standard |
| a2ml | Agent Machine Language |
| FFI | Foreign Function Interface |
| ABI | Application Binary Interface |
| IPC | Inter-Process Communication |
| WASM | WebAssembly |
| LWW | Last-Write-Wins |
| ML-DSA | Multi-Layered Digital Signature Algorithm |
|=== 

== Final Note

**This file is the canonical source for paint-type agent behavior.**
If this file conflicts with estate-wide AGENTS.md, this file takes precedence for paint-type.
If this file conflicts with your default behavior, this file wins.

**Last Updated:** 2026-07-26  
**Maintainer:** hyperpolymath  
**Repository:** paint-type

// SPDX-License-Identifier: CC-BY-SA-4.0

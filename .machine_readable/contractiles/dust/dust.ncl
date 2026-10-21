# SPDX-License-Identifier: AGPL-3.0-or-later
# Dust — exnovation / retirement runner for paint-type
let base = import "../_base.ncl" in

{
  pedigree =
    base.pedigree_schema
    & {
      contractile_verb = "dust",
      semantics = "exnovation / removal",
      security = {
        leash = 'Kennel,
        trust_level = "proposes deletion; --apply required to execute",
        allow_network = false,
        allow_filesystem_write = true,
        allow_subprocess = true,
        destructive_mode_requires_flag = "--apply",
      },
      metadata = {
        name = "dust-runner",
        version = "1.0.0",
        description = "Tracks legacy bridge artifacts and safe retirement paths for paint-type.",
        paired_xfile = "dust.k9.ncl#declaration",
        author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
      },
    },

  schema = {
    removal_candidates
      | Array {
        id | String,
        description | String,
        target | String,
        reason | String,
        probe | String | optional,
        status | [| 'declared, 'proposed, 'approved, 'removed |] | default = 'declared,
        approver | String | optional,
        notes | String | optional,
      },
  },

  run =
    base.run_defaults
    & {
      on_any_fail = "continue-with-warnings",
      report_format = "json",
      emit_summary = true,
      apply_requires_approval = true,
    },
}

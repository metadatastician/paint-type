# SPDX-License-Identifier: AGPL-3.0-or-later
# Intend — north-star runner for paint-type
let base = import "../_base.ncl" in

{
  pedigree =
    base.pedigree_schema
    & {
      contractile_verb = "intend",
      semantics = "north-star (commitments + aspirations)",
      security = {
        leash = 'Kennel,
        trust_level = "read-only reporting",
        allow_network = false,
        allow_filesystem_write = false,
        allow_subprocess = true,
      },
      metadata = {
        name = "intend-runner",
        version = "2.0.0",
        description = "Reports progress toward committed paint-type milestones and lists horizon aspirations. Non-gating.",
        paired_xfile = "intend.k9.ncl#declaration",
        author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
      },
    },

  schema = {
    intents
      | Array {
        id | String,
        description | String,
        probe | String | optional,
        status | [| 'declared, 'in_progress, 'done, 'deferred, 'retired |] | default = 'declared,
        notes | String | optional,
        target_date | String | optional,
      },
    wishes
      | Array {
        id | String,
        description | String,
        horizon | [| 'near, 'mid, 'far |] | default = 'mid,
        why | String | optional,
        status | [| 'declared, 'in_progress, 'achieved, 'abandoned |] | default = 'declared,
        notes | String | optional,
      }
      | optional,
  },

  run =
    base.run_defaults
    & {
      on_pass = "continue",
      on_any_fail = "continue",
      report_format = "json",
      emit_summary = true,
      emit_diff = true,
      emit_grouped_by_horizon = true,
    },
}

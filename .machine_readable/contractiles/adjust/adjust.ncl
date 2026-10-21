# SPDX-License-Identifier: AGPL-3.0-or-later
# Adjust — accessibility + drift tolerance runner for paint-type
let base = import "../_base.ncl" in

{
  pedigree =
    base.pedigree_schema
    & {
      contractile_verb = "adjust",
      semantics = "accessibility compliance",
      security = {
        leash = 'Kennel,
        trust_level = "fixes allowed where deterministic",
        allow_network = false,
        allow_filesystem_write = true,
        allow_subprocess = true,
      },
      metadata = {
        name = "adjust-runner",
        version = "1.0.0",
        description = "Evaluates accessibility and drift-tolerance requirements for paint-type.",
        paired_xfile = "adjust.k9.ncl#declaration",
        author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
      },
    },

  schema = {
    requirements
      | Array {
        id | String,
        description | String,
        probe | String,
        status | [| 'declared, 'partial, 'verified, 'failing |] | default = 'declared,
        compliance | String | optional,
        notes | String | optional,
        fix | String | optional,
      },
  },

  run =
    base.run_defaults
    & {
      on_any_fail = "continue-with-warnings",
      report_format = "json",
      emit_summary = true,
      auto_fix_when_available = true,
    },
}

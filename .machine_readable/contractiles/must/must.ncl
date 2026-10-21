# SPDX-License-Identifier: AGPL-3.0-or-later
# Must — invariants runner for paint-type
let base = import "../_base.ncl" in

{
  pedigree =
    base.pedigree_schema
    & {
      contractile_verb = "must",
      semantics = "invariant",
      security = {
        leash = 'Kennel,
        trust_level = "read-only verification",
        allow_network = false,
        allow_filesystem_write = false,
        allow_subprocess = true,
      },
      metadata = {
        name = "must-runner",
        version = "1.0.0",
        description = "Evaluates every invariant in paint-type's must.k9.ncl declaration as a hard gate.",
        paired_xfile = "must.k9.ncl#declaration",
        author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
      },
    },

  schema = {
    invariants
      | Array {
        id | String,
        description | String,
        probe | String,
        status | [| 'declared, 'verified, 'failing |] | default = 'declared,
        severity | [| 'critical, 'high, 'medium |] | default = 'critical,
        notes | String | optional,
        fix | String | optional,
      },
  },

  run =
    base.run_defaults
    & {
      on_any_fail = "exit-nonzero",
      report_format = "json",
      emit_summary = true,
    },
}

# SPDX-License-Identifier: AGPL-3.0-or-later
# Trust — security + safe-hacking runner for paint-type
let base = import "../_base.ncl" in

{
  pedigree =
    base.pedigree_schema
    & {
      contractile_verb = "trust",
      semantics = "security + provenance + safe-hacking",
      security = {
        leash = 'Kennel,
        trust_level = "verification + authorised-probe",
        allow_network = false,
        allow_filesystem_write = false,
        allow_subprocess = true,
        authorised_probes_only = true,
      },
      metadata = {
        name = "trust-runner",
        version = "1.0.0",
        description = "Security + provenance verifications plus authorised repo-scoped safe-hacking probes for paint-type.",
        paired_xfile = "trust.k9.ncl#declaration",
        author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
      },
    },

  schema = {
    verifications
      | Array {
        id | String,
        description | String,
        probe | String,
        status | [| 'declared, 'verified, 'failing |] | default = 'declared,
        severity | [| 'critical, 'high, 'medium, 'low |] | default = 'high,
        notes | String | optional,
      },
    safe_hacking
      | {
        scope | String,
        allowed_probe_classes
          | Array [| 'fuzz, 'property_test, 'chaos, 'auth_bypass, 'injection, 'timing |]
          | default
          = [],
        probes
          | Array {
            id | String,
            class | [| 'fuzz, 'property_test, 'chaos, 'auth_bypass, 'injection, 'timing |],
            description | String,
            probe | String,
            expected_outcome | [| 'probe_blocks_attempt, 'probe_finds_no_issue |],
            timeout_seconds | Number | default = 300,
            notes | String | optional,
          }
          | default
          = [],
      }
      | default
      = { scope = "this-repo-only", allowed_probe_classes = [], probes = [] },
  },

  run =
    base.run_defaults
    & {
      on_any_fail = "exit-nonzero",
      safe_hacking_on_unexpected_outcome = "exit-nonzero",
      report_format = "json",
      emit_summary = true,
    },
}

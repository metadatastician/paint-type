# SPDX-License-Identifier: AGPL-3.0-or-later
# Bust — error-handling / failure-recovery runner for paint-type
let base = import "../_base.ncl" in

{
  pedigree =
    base.pedigree_schema
    & {
      contractile_verb = "bust",
      semantics = "error handling + failure recovery",
      security = {
        leash = 'Kennel,
        trust_level = "controlled failure injection; scoped to system-under-test",
        allow_network = false,
        allow_filesystem_write = true,
        allow_subprocess = true,
        injection_scope = "system-under-test-only",
      },
      metadata = {
        name = "bust-runner",
        version = "1.0.0",
        description = "Exercises declared failure modes and verifies recovery paths for paint-type.",
        paired_xfile = "bust.k9.ncl#declaration",
        author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
      },
    },

  schema = {
    failure_modes
      | Array {
        id | String,
        description | String,
        class | [| 'network, 'disk_full, 'oom, 'timeout, 'partial_write, 'panic, 'crash, 'rollback, 'concurrency |],
        injection_probe | String,
        recovery_probe | String,
        expected_recovery_time_seconds | Number | default = 30,
        status | [| 'declared, 'drilled, 'verified, 'failing |] | default = 'declared,
        notes | String | optional,
      },
  },

  run =
    base.run_defaults
    & {
      on_any_fail = "exit-nonzero",
      report_format = "json",
      emit_summary = true,
      record_recovery_times = true,
    },
}

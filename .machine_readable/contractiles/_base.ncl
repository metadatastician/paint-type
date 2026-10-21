# SPDX-License-Identifier: AGPL-3.0-or-later
#
# _base.ncl — Shared contractile base for paint-type
#
# Provides four named schema fragments imported by every verb runner and
# verb K9 component:
#
#   pedigree_schema   — canonical pedigree block shape (K9 Contract v1.0.0)
#   status_core_doc   — documentation of the shared status trio
#   probe_schema      — structured probe form
#   run_defaults      — default runner behaviour
{
  pedigree_schema = {
    schema_version | String | default = "1.0.0",
    contractile_verb | String | default = "UNSET",
    semantics | String | default = "UNSET",
    security = {
      leash | [| 'Kennel, 'Yard, 'Hunt |] | default = 'Kennel,
      trust_level | String | default = "UNSET",
      allow_network | Bool | default = false,
      allow_filesystem_write | Bool | default = false,
      allow_subprocess | Bool | default = true,
    },
    metadata = {
      name | String | default = "UNSET",
      version | String | default = "1.0.0",
      description | String | default = "UNSET",
      paired_xfile | String | default = "UNSET",
      author | String | default = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },
  },

  status_core_doc = "status_core values: declared | verified | failing — extended per verb",

  probe_schema = {
    command | String,
    timeout_seconds | Number | default = 300,
    allowed_exit_codes | Array Number | default = [0],
    permission_class
      | [| 'read_only, 'filesystem_write, 'subprocess, 'network |]
      | default
      = 'read_only,
  },

  run_defaults = {
    on_pass = "continue",
    on_any_fail = "exit-nonzero",
    report_format = "json",
    emit_summary = true,
  },
}

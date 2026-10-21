K9!
# SPDX-License-Identifier: AGPL-3.0-or-later
# must.k9.ncl — K9 trust-tier component of the must trident for paint-type
# Author: Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
#
# Conforms to K9 Contract Specification v1.0.0 (standards#1058, ruling D173).

let base = import "../_base.ncl" in

{
  pedigree = base.pedigree_schema & {
    contractile_verb = "must",
    schema_version   = "1.0.0",
    component_type   = "contractile:must",
    semantics        = "invariant assertion — release-blocking",
    side_effects     = [],
    paired_xfile     = "../must/must.k9.ncl#declaration",
    paired_runner    = "../must/must.ncl",

    tier      = 'Yard,
    authority = 'blocking,

    metadata = {
      name = "paint-type-must-k9",
      version = "1.0.0",
      description = "Evaluates paint-type release-blocking invariants across ABI/FFI soundness, DEED/K9 conformance, and workflow lock integrity.",
      paired_xfile = "must.k9.ncl#declaration",
      paired_runner = "must.ncl",
      author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },

    security = {
      leash = 'Yard,
      signature_required = false,
      trust_level = "read-only invariant verification with subprocess",
      allow_network = false,
      allow_filesystem_write = false,
      allow_subprocess = false,
      probe_scope = 'read_only,
      probe_kinds_allowed = [
        'file_existence,
        'pattern_presence,
        'pattern_absence,
        'schema_match,
        'version_equality,
        'count_threshold,
      ],
      probe_kinds_denied = [
        'network_call,
        'filesystem_mutation,
        'external_api,
        'exploit_attempt,
      ],
    },

    trident = {
      notes = "Hard-gating invariant trident for paint-type.",
      trident_version = "1.0.0",
      semantics = "invariant assertion — release-blocking",
      cardinality = "one per repo",
      authority = "blocking (hard gate)",
      files = [],
      signed_by = {
        user = "Jonathan D.A. Jewell",
        date = "2026-10-21",
        context = "paint-type must trident under canon 2.1.2 and K9 contract v1.0.0.",
      },
      history = [
        { date = "2026-10-21", event = "k9-v1-cutover", note = "Migrated paint-type invariants into K9 v1.0.0 must trident." },
      ],
    },
  },

  declaration = {
    preamble = "# SPDX-License-Identifier: AGPL-3.0-or-later\n# Mustfile — physical and architectural invariants for paint-type\n",
    abstract = "Release-blocking invariants for paint-type spanning Idris2 soundness, Zig/Rust ABI parity, canon 2.1.2 DEED/K9 contracts, and GitHub Actions lockfile closure.",
    prose = [],
    invariants = [
      {
        id = "license-present",
        section = "File Presence",
        description = "LICENSE file must exist and carry GNU AGPLv3",
        probe = "test -f LICENSE && grep -q 'GNU AFFERO GENERAL PUBLIC LICENSE' LICENSE",
        severity = 'critical,
      },
      {
        id = "repo-deed-present",
        section = "File Presence",
        description = "Single canonical paint-type_chora.deed exists with (canon), (anchor), (neurosym), and (playbook) clauses",
        probe = "d=$(bash scripts/deed-field.sh --find .) && bash scripts/deed-field.sh --has \"$d\" canon && bash scripts/deed-field.sh --has \"$d\" anchor && bash scripts/deed-field.sh --has \"$d\" neurosym && bash scripts/deed-field.sh --has \"$d\" playbook",
        severity = 'critical,
      },
      {
        id = "coordination-nerve-present",
        section = "File Presence",
        description = "Both coordination.k9 (nerve receptor) and coordination.k9.ncl (session contract) exist at repo root",
        probe = "test -f coordination.k9 && test -f coordination.k9.ncl",
        severity = 'critical,
      },
      {
        id = "six-contractile-tridents",
        section = "Directory Structure",
        description = "All six K9 v1.0.0 verb tridents exist with both <verb>.k9.ncl and <verb>.ncl",
        probe = "for v in intend must trust adjust dust bust; do test -f .machine_readable/contractiles/$v/$v.k9.ncl && test -f .machine_readable/contractiles/$v/$v.ncl || exit 1; done",
        severity = 'critical,
      },
      {
        id = "no-idris-soundness-escapes",
        section = "Formal Soundness",
        description = "Zero believe_me, assert_total, or really_believe_me in src/interface/Abi/",
        probe = "test -z \"$(grep -rn 'believe_me\\|assert_total\\|really_believe_me' src/interface/Abi/ 2>/dev/null)\"",
        severity = 'critical,
      },
      {
        id = "actions-lock-synchronized",
        section = "CI Integrity",
        description = ".github/workflows/actions.lock is strictly synchronized with every workflow file",
        probe = "bash scripts/check-lock-sync.sh",
        severity = 'critical,
      },
      {
        id = "gitbot-enrolment-present",
        section = "Fleet Integration",
        description = ".gitbot-fleet/enrolment.json declares applicable reporters for the Hypatia verdict grid",
        probe = "test -f .gitbot-fleet/enrolment.json && jq -e '.schema == 1 and (.reporters | type == \"object\")' .gitbot-fleet/enrolment.json >/dev/null",
        severity = 'critical,
      },
      {
        id = "no-template-placeholders",
        section = "Template Integrity",
        description = "Zero unresolved {{PLACEHOLDER}} tokens in paint-type_chora.deed or .machine_readable/contractiles/",
        probe = "test -z \"$(grep -r '{{' paint-type_chora.deed .machine_readable/contractiles/ 2>/dev/null)\"",
        severity = 'critical,
      },
    ],
  },

  variance_schema | not_exported = {
    entry_id                        | String,
    reason                          | String,
    approved_by                     | String,
    scope                           | String,
    expires                         | String,
    review_notes                    | String | optional,
    severity_acknowledged           | [| 'critical, 'high, 'medium |],
    waived_consequence_description  | String,
  },

  execution = {
    triggers = [ 'session_close, 'on_demand, 'pre_push, 'pre_merge ],
    per_invariant = {
      run_probe = true,
      record_outcome = true,
      respect_variance = true,
      on_unmet = 'fail,
      severity_escalation = 'honour,
      track_per_session_trend = true,
      flag_suspicious_regressions = true,
    },
    evidence_sinks = [
      {
        kind       = 'verisimdb,
        table      = "contractile_executions",
        schema     = "contractile_execution_v1",
        aux_tables = [ "must_invariant_history" ],
      },
      {
        kind        = 'drift_log,
        path        = ".machine_readable/descriptiles/drift_praxis.deed",
        append_only = true,
      },
    ],
    on_close = {
      re_execute_all_invariants = true,
      diff_against_last_ratification = true,
      emit_drift_entries_for_new_failures = true,
      surface_expired_variances = true,
      block_session_close_on_critical_drift = true,
      flag_silent_regression = true,
    },
    on_open = {
      render_summary = 'plain_language,
      include_drift_log_from_last_close = true,
      include_active_variances = true,
      include_recent_anchors = true,
      anchor_lookback_weeks = 8,
      include_silent_regressions = true,
      negotiation = {
        required = true,
        ai_required_inputs = [
          'timeline_realism,
          'industry_standards,
          'audience_feasibility,
          'resulting_invariants,
          'ecosystem_dependencies,
        ],
        user_engagement_required = true,
        user_engagement_mode = 'per_input_response,
        specification_translation = {
          ai_produces_spec_form = true,
          user_reviews_in_domain_language = true,
          schema_authoring_is_ai_responsibility = true,
          translation_faithfulness_auditable = true,
        },
      },
      accountability_pledge = {
        required = true,
        parties = [
          {
            role = 'user,
            pledge = "I accept accountability for declared invariants and will raise a variance rather than disabling probes.",
            signature_required = true,
          },
          {
            role = 'ai_agent,
            pledge = "I will hold declared invariants and refuse to weaken probes to unblock merges.",
            signature_required = true,
          },
        ],
        signed_record_destination = ".machine_readable/descriptiles/ratification-<session-id>_praxis.deed",
        must_precede_work = true,
      },
      ratification_record_shape = {
        includes_negotiation_transcript = true,
        includes_both_pledges = true,
        includes_invariant_summary = true,
        signed = true,
        dated = true,
        session_id = 'required,
        contract_hash = 'required,
      },
    },
  },

  failure_mode_defenses = [
    'A1_enthusiasm_capture,
    'A5_grandiose_scale_hype,
    'C1_scope_creep,
    'C2_capability_collapse,
    'C3_helpfulness_inflation,
    'D1_lore_fabrication,
    'D2_completeness_illusion,
    'D3_test_theatre,
    'D4_error_hiding,
    'E1_refactor_stampede,
    'E3_premature_abstraction,
    'F1_across_session_forgetting,
  ],
}

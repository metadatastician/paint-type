K9!
# SPDX-License-Identifier: AGPL-3.0-or-later
# dust.k9.ncl — K9 trust-tier component of the dust trident for paint-type
# Author: Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
#
# Conforms to K9 Contract Specification v1.0.0 (standards#1058, ruling D173).

let base = import "../_base.ncl" in

{
  pedigree = base.pedigree_schema & {
    contractile_verb = "dust",
    schema_version   = "1.0.0",
    component_type   = "contractile:dust",
    semantics        = "rollback / recovery / deprecation / audit-trail preservation",
    side_effects     = [],
    paired_xfile     = "../dust/dust.k9.ncl#declaration",
    paired_runner    = "../dust/dust.ncl",

    tier      = 'Yard,
    authority = 'advisory,

    metadata = {
      name = "paint-type-dust-k9",
      version = "1.0.0",
      description = "Exnovation schedule and audit-trail preservation for retiring legacy .a2ml and pre-v1.0.0 templates in paint-type.",
      paired_xfile = "dust.k9.ncl#declaration",
      paired_runner = "dust.ncl",
      author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },

    security = {
      leash = 'Yard,
      trust_level = "audit-trail verification + structural checks",
      allow_network = false,
      allow_filesystem_write_conditional = true,
      allow_subprocess = false,
      destructive_action_gating = {
        default_mode = 'dry_run,
        requires_flag = "--apply",
        requires_per_item_approval = true,
        approval_mechanism = 'explicit_user_signature,
      },
    },

    trident = {
      notes = "Advisory exnovation and retirement trident for paint-type.",
      trident_version = "1.0.0",
      semantics = "rollback / recovery / deprecation / audit-trail preservation",
      cardinality = "one per repo",
      authority = "advisory (continue-with-warnings)",
      files = [],
      signed_by = {
        user = "Jonathan D.A. Jewell",
        date = "2026-10-21",
        context = "paint-type dust trident under canon 2.1.2 and K9 contract v1.0.0.",
      },
      history = [
        { date = "2026-10-21", event = "k9-v1-cutover", note = "Recorded explicit retirement schedule for transitional .a2ml bridges and legacy self-validating files." },
      ],
    },
  },

  declaration = {
    preamble = "# SPDX-License-Identifier: AGPL-3.0-or-later\n# Dustfile — exnovation and cleanup tracking for paint-type\n",
    abstract = "Planned retirement candidates with preserved rollback paths as paint-type completes the .a2ml -> .deed + K9 v1.0.0 transition.",
    prose = [],
    removal_candidates = [
      {
        id = "transitional-a2ml-manifests",
        section = "Format Cutover",
        description = "Retire transitional .a2ml manifests once scripts/validate-template.sh cuts over to scripts/deed-field.sh",
        probe = "test -f paint-type_chora.deed && test -f scripts/deed-field.sh",
        severity = 'info,
        notes = "Keep 0-AI-MANIFEST.a2ml and .machine_readable/*.a2ml until scripts/validate-template.sh drops phase-2 file assertions.",
      },
      {
        id = "coordination-self-validating-bridge",
        section = "Format Cutover",
        description = "Retire coordination.self-validating once session/dispatch.sh reads coordination.k9.ncl via nickel export",
        probe = "test -f coordination.k9.ncl && test -f coordination.k9",
        severity = 'info,
        notes = "session/dispatch.sh currently parses YAML in coordination.self-validating when Nickel is absent.",
      },
      {
        id = "no-legacy-contractile-a2ml",
        section = "Contractiles Hygiene",
        description = "Zero retired .a2ml, .contractile, or .hs files inside .machine_readable/contractiles/",
        probe = "test -z \"$(find .machine_readable/contractiles/ \\( -name '*.contractile' -o -name '*.hs' -o -name '*.a2ml' \\) 2>/dev/null)\"",
        severity = 'warning,
      },
    ],
  },

  variance_schema | not_exported = {
    entry_id                | String,
    reason                  | String,
    approved_by             | String,
    scope                   | String,
    expires                 | String,
    review_notes            | String | optional,
    rollback_path_preserved | Bool,
  },

  execution = {
    triggers = [ 'session_close, 'on_demand ],
    per_retirement = {
      verify_rollback_path_documented = true,
      verify_audit_trail_preserved = true,
      respect_variance = true,
      on_rollback_path_missing = 'warn,
      on_audit_trail_broken = 'warn,
      flag_unsafe_retirement = true,
    },
    evidence_sinks = [
      {
        kind       = 'verisimdb,
        table      = "contractile_executions",
        schema     = "contractile_execution_v1",
        aux_tables = [ "dust_retirement_history" ],
      },
      {
        kind        = 'drift_log,
        path        = ".machine_readable/descriptiles/drift_praxis.deed",
        append_only = true,
      },
    ],
    on_close = {
      re_verify_all_retirement_paths = true,
      diff_against_last_ratification = true,
      emit_drift_entries_for_missing_rollback = true,
      emit_drift_entries_for_broken_audit_trail = true,
      surface_expired_variances = true,
      block_session_close_on_any_drift = false,
    },
    on_open = {
      render_summary = 'plain_language,
      include_drift_log_from_last_close = true,
      include_active_variances = true,
      include_recent_anchors = true,
      anchor_lookback_weeks = 8,
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
            pledge = "I accept accountability for preserving rollback paths when retiring legacy files.",
            signature_required = true,
          },
          {
            role = 'ai_agent,
            pledge = "I will operate in dry-run mode by default and preserve rollback paths on every retirement.",
            signature_required = true,
          },
        ],
        signed_record_destination = ".machine_readable/descriptiles/ratification-<session-id>_praxis.deed",
        must_precede_work = true,
      },
      ratification_record_shape = {
        includes_negotiation_transcript = true,
        includes_both_pledges = true,
        includes_retirement_schedule = true,
        signed = true,
        dated = true,
        session_id = 'required,
        contract_hash = 'required,
      },
    },
  },

  failure_mode_defenses = [
    'A1_enthusiasm_capture,
    'C2_capability_collapse,
    'D5_sycophancy,
    'E1_refactor_stampede,
    'E2_cosmetic_churn,
    'F1_across_session_forgetting,
  ],
}

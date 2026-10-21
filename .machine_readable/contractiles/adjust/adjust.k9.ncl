K9!
# SPDX-License-Identifier: AGPL-3.0-or-later
# adjust.k9.ncl — K9 trust-tier component of the adjust trident for paint-type
# Author: Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
#
# Conforms to K9 Contract Specification v1.0.0 (standards#1058, ruling D173).

let base = import "../_base.ncl" in

{
  pedigree = base.pedigree_schema & {
    contractile_verb = "adjust",
    schema_version   = "1.0.0",
    component_type   = "contractile:adjust",
    semantics        = "drift tolerances + accessibility + corrective actions",
    side_effects     = [],
    paired_xfile     = "../adjust/adjust.k9.ncl#declaration",
    paired_runner    = "../adjust/adjust.ncl",

    tier      = 'Yard,
    authority = 'advisory,

    metadata = {
      name = "paint-type-adjust-k9",
      version = "1.0.0",
      description = "Accessibility compliance (WCAG 2.1 AA) and structural drift tolerances for paint-type.",
      paired_xfile = "adjust.k9.ncl#declaration",
      paired_runner = "adjust.ncl",
      author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },

    security = {
      leash = 'Yard,
      trust_level = "tolerance measurement + declared auto-fix",
      allow_network = false,
      allow_filesystem_write_conditional = true,
      allow_subprocess = false,
      probe_scope = 'measurement_plus_declared_fix,
    },

    trident = {
      notes = "Advisory tolerance and WCAG 2.1 AA accessibility trident for paint-type.",
      trident_version = "1.0.0",
      semantics = "drift tolerances + corrective actions",
      cardinality = "one per repo",
      authority = "advisory (continue-with-warnings)",
      files = [],
      signed_by = {
        user = "Jonathan D.A. Jewell",
        date = "2026-10-21",
        context = "paint-type adjust trident under canon 2.1.2 and K9 contract v1.0.0.",
      },
      history = [
        { date = "2026-10-21", event = "k9-v1-cutover", note = "Migrated paint-type accessibility and drift tolerances to K9 v1.0.0 adjust trident." },
      ],
    },
  },

  declaration = {
    preamble = "# SPDX-License-Identifier: AGPL-3.0-or-later\n# Adjustfile — drift tolerances and accessibility requirements for paint-type\n",
    abstract = "WCAG 2.1 AA UI ergonomics and repository drift tolerances for paint.type.",
    prose = [],
    requirements = [
      {
        id = "ui-accessible-shell",
        section = "Accessibility",
        description = "WebView UI shell exposes accessible markup and keyboard navigation surfaces",
        tolerance = "src/ui/ preserves semantic labels and keyboard shortcut handling",
        corrective = "Audit ARIA labels and keyboard handlers in src/ui/",
        severity = 'advisory,
      },
      {
        id = "contractile-runner-parity",
        section = "Structural Drift",
        description = "Every K9 verb declaration has its matching Nickel runner schema",
        tolerance = "6 <verb>.k9.ncl files matched 1:1 with 6 <verb>.ncl runners",
        corrective = "Restore any missing paired <verb>.ncl file",
        severity = 'advisory,
      },
      {
        id = "no-broken-symlinks",
        section = "Structural Drift",
        description = "Zero broken symbolic links in repository tree",
        tolerance = "0 broken symlinks outside build caches",
        corrective = "Remove or repair dangling symlinks",
        severity = 'advisory,
      },
      {
        id = "spdx-header-coverage",
        section = "Governance Drift",
        description = "All source and workflow files carry SPDX-License-Identifier headers",
        tolerance = "0 source/workflow files missing SPDX-License-Identifier",
        corrective = "Add AGPL-3.0-or-later SPDX header to newly created files",
        severity = 'advisory,
      },
    ],
  },

  variance_schema | not_exported = {
    entry_id               | String,
    reason                 | String,
    approved_by            | String,
    scope                  | String,
    expires                | String,
    review_notes           | String | optional,
    tolerance_band_widened | String,
    widened_to_value       | String,
  },

  execution = {
    triggers = [ 'session_close, 'on_demand, 'pre_push ],
    per_tolerance = {
      measure_drift = true,
      record_outcome = true,
      respect_variance = true,
      on_exceeded = 'warn_and_attempt_fix,
      on_auto_fix_applied = 'record_and_continue,
      on_auto_fix_unavailable = 'record_as_advisory_drift,
      track_drift_trend_over_sessions = true,
      flag_accelerating_drift = true,
    },
    evidence_sinks = [
      {
        kind       = 'verisimdb,
        table      = "contractile_executions",
        schema     = "contractile_execution_v1",
        aux_tables = [ "adjust_drift_history" ],
      },
      {
        kind        = 'drift_log,
        path        = ".machine_readable/descriptiles/drift_praxis.deed",
        append_only = true,
      },
    ],
    on_close = {
      re_measure_all_tolerances = true,
      diff_against_last_ratification = true,
      emit_drift_entries_for_tolerance_exceeded = true,
      surface_expired_variances = true,
      surface_accelerating_drift = true,
      block_session_close_on_any_drift = false,
    },
    on_open = {
      render_summary = 'plain_language,
      include_drift_log_from_last_close = true,
      include_active_variances = true,
      include_recent_anchors = true,
      anchor_lookback_weeks = 8,
      include_tolerance_trend_summary = true,
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
            pledge = "I accept accountability for reviewing tolerance drift and accessibility warnings.",
            signature_required = true,
          },
          {
            role = 'ai_agent,
            pledge = "I will surface drift trends and propose deterministic corrective actions rather than muting warnings.",
            signature_required = true,
          },
        ],
        signed_record_destination = ".machine_readable/descriptiles/ratification-<session-id>_praxis.deed",
        must_precede_work = true,
      },
      ratification_record_shape = {
        includes_negotiation_transcript = true,
        includes_both_pledges = true,
        includes_tolerance_bands_snapshot = true,
        signed = true,
        dated = true,
        session_id = 'required,
        contract_hash = 'required,
      },
    },
  },

  failure_mode_defenses = [
    'A1_enthusiasm_capture,
    'C3_helpfulness_inflation,
    'C4_modernization_drift,
    'E2_cosmetic_churn,
    'F2_context_window_erosion,
  ],
}

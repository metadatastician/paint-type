K9!
# SPDX-License-Identifier: AGPL-3.0-or-later
# bust.k9.ncl — K9 trust-tier component of the bust trident for paint-type
# Author: Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
#
# Conforms to K9 Contract Specification v1.0.0 (standards#1058, ruling D173).

let base = import "../_base.ncl" in

{
  pedigree = base.pedigree_schema & {
    contractile_verb = "bust",
    schema_version   = "1.0.0",
    component_type   = "contractile:bust",
    semantics        = "hard-stop / expiry / must-not-run declarations",
    side_effects     = [],
    paired_xfile     = "../bust/bust.k9.ncl#declaration",
    paired_runner    = "../bust/bust.ncl",

    tier      = 'Yard,
    authority = 'blocking,

    metadata = {
      name = "paint-type-bust-k9",
      version = "1.0.0",
      description = "Hard-stop failure-recovery drills for corrupt .ptype containers, truncated tiles, and headless shell lifecycle in paint-type.",
      paired_xfile = "bust.k9.ncl#declaration",
      paired_runner = "bust.ncl",
      author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },

    security = {
      leash = 'Yard,
      signature_required = false,
      trust_level = "read-only expiry + state-check with subprocess",
      allow_network = false,
      allow_filesystem_write = false,
      allow_subprocess = false,
      probe_scope = 'read_only,
    },

    trident = {
      notes = "Hard-gating failure-recovery trident for paint-type.",
      trident_version = "1.0.0",
      semantics = "hard-stop / expiry / must-not-run declarations",
      cardinality = "one per repo",
      authority = "blocking (hard gate)",
      files = [],
      signed_by = {
        user = "Jonathan D.A. Jewell",
        date = "2026-10-21",
        context = "paint-type bust trident under canon 2.1.2 and K9 contract v1.0.0.",
      },
      history = [
        { date = "2026-10-21", event = "k9-v1-cutover", note = "Migrated paint-type failure-recovery drills to K9 v1.0.0 bust trident." },
      ],
    },
  },

  declaration = {
    preamble = "# SPDX-License-Identifier: AGPL-3.0-or-later\n# Bustfile — failure modes and recovery paths for paint-type\n",
    abstract = "Failure modes across .ptype container parsing, tile checksum mismatches, and Gossamer headless shell clean shutdown.",
    prose = [],
    failure_modes = [
      {
        id = "corrupt-ptype-container-recovery",
        section = "Document Integrity",
        class = 'partial_write,
        description = "Corrupt or truncated .ptype header/tile stream returns typed PaintError without panicking",
        injection_probe = "cargo test --manifest-path src/paint_core/Cargo.toml ptype --quiet",
        recovery_probe = "cargo test --manifest-path src/paint_core/Cargo.toml --lib --quiet",
        expected_recovery_time_seconds = 30,
        status = 'verified,
        notes = "Exercised continuously by cargo test and Fuzz & Property Tests smoke targets.",
      },
      {
        id = "undo-graph-rollback-integrity",
        section = "Canvas State",
        class = 'rollback,
        description = "UndoGraph rollback restores exact immutable 64x64 RGBA16F tile hashes after cancelled stroke",
        injection_probe = "cargo test --manifest-path src/paint_core/Cargo.toml undo --quiet",
        recovery_probe = "cargo test --manifest-path src/paint_core/Cargo.toml document --quiet",
        expected_recovery_time_seconds = 30,
        status = 'verified,
      },
      {
        id = "lockfile-drift-detection",
        section = "CI Integrity",
        class = 'rollback,
        description = "check-lock-sync.sh immediately fails closed on any unpinned or missing GitHub Actions ref",
        injection_probe = "bash tests/workflows/check_lock_sync_test.sh",
        recovery_probe = "bash scripts/check-lock-sync.sh",
        expected_recovery_time_seconds = 10,
        status = 'verified,
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
    per_hard_stop = {
      run_probe = true,
      record_outcome = true,
      respect_variance = true,
      on_triggered = 'fail,
      severity_escalation = 'honour,
      flag_deprecated_reintroduction = true,
    },
    evidence_sinks = [
      {
        kind       = 'verisimdb,
        table      = "contractile_executions",
        schema     = "contractile_execution_v1",
        aux_tables = [ "bust_triggers_history" ],
      },
      {
        kind        = 'drift_log,
        path        = ".machine_readable/descriptiles/drift_praxis.deed",
        append_only = true,
      },
    ],
    on_close = {
      re_execute_all_hard_stops = true,
      diff_against_last_ratification = true,
      emit_drift_entries_for_new_triggers = true,
      surface_expired_variances = true,
      block_session_close_on_critical_bust = true,
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
            pledge = "I accept accountability for maintaining working recovery paths for every declared failure mode.",
            signature_required = true,
          },
          {
            role = 'ai_agent,
            pledge = "I will refuse changes that remove or bypass declared failure-recovery checks.",
            signature_required = true,
          },
        ],
        signed_record_destination = ".machine_readable/descriptiles/ratification-<session-id>_praxis.deed",
        must_precede_work = true,
      },
      ratification_record_shape = {
        includes_negotiation_transcript = true,
        includes_both_pledges = true,
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
    'C4_modernization_drift,
    'D4_error_hiding,
    'E1_refactor_stampede,
    'F1_across_session_forgetting,
  ],
}

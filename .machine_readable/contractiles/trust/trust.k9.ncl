K9!
# SPDX-License-Identifier: AGPL-3.0-or-later
# trust.k9.ncl — K9 trust-tier component of the trust trident for paint-type
# Author: Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
#
# Conforms to K9 Contract Specification v1.0.0 (standards#1058, ruling D173).

let base = import "../_base.ncl" in

{
  pedigree = base.pedigree_schema & {
    contractile_verb = "trust",
    schema_version   = "1.0.0",
    component_type   = "contractile:trust",
    semantics        = "security + provenance + safe-hacking",
    side_effects     = [],
    paired_xfile     = "../trust/trust.k9.ncl#declaration",
    paired_runner    = "../trust/trust.ncl",

    tier      = 'Yard,
    authority = 'blocking,

    metadata = {
      name = "paint-type-trust-k9",
      version = "1.0.0",
      description = "Security, provenance, supply-chain pinning, and repo-scoped fuzz/property verification contract for paint-type.",
      paired_xfile = "trust.k9.ncl#declaration",
      paired_runner = "trust.ncl",
      author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },

    security = {
      leash = 'Yard,
      signature_required = false,
      trust_level = "verification + authorised-probe + hard-gate",
      allow_network = false,
      allow_filesystem_write = false,
      allow_subprocess = false,
      authorised_probes_only = true,
      probe_scope_enforcement = 'this_repo_only,
    },

    trident = {
      notes = "Hard-gating security and supply-chain provenance trident for paint-type.",
      trident_version = "1.0.0",
      semantics = "security + provenance + safe-hacking",
      cardinality = "one per repo",
      authority = "blocking (hard gate)",
      files = [],
      signed_by = {
        user = "Jonathan D.A. Jewell",
        date = "2026-10-21",
        context = "paint-type trust trident under canon 2.1.2 and K9 contract v1.0.0.",
      },
      history = [
        { date = "2026-10-21", event = "k9-v1-cutover", note = "Migrated paint-type security verifications and fuzz/property probes to K9 v1.0.0 trust trident." },
      ],
    },
  },

  declaration = {
    preamble = "# SPDX-License-Identifier: AGPL-3.0-or-later\n# Trustfile — security and provenance invariants for paint-type\n",
    abstract = "Security, supply-chain pinning, secret hygiene, and deterministic scanner provenance checks for paint-type.",
    prose = [
      {
        section = "Trust Posture",
        text = "All CI security gates (Hypatia, panic-attack assail, pons-asinorum, OpenSSF Scorecard, secret-scanner) run deterministically at full-SHA pins with zero LLM execution inside CI.",
      },
    ],
    verifications = [
      {
        id = "no-secrets-committed",
        section = "Secrets Hygiene",
        description = "No .env or credential files committed in repository tree",
        probe = "test ! -f .env && test ! -f credentials.json && test ! -f .env.local",
        severity = 'critical,
      },
      {
        id = "no-private-keys",
        section = "Secrets Hygiene",
        description = "No private key material committed in repository tree",
        probe = "! find . -name '*.pem' -o -name '*.key' -o -name 'id_rsa' -o -name 'id_ed25519' 2>/dev/null | grep -v third_party | head -1 | grep -q .",
        severity = 'critical,
      },
      {
        id = "standards-reusables-single-sha",
        section = "Supply Chain",
        description = "Every reusable workflow call into hyperpolymath/standards is pinned to a 40-char commit SHA",
        probe = "! grep -rn 'hyperpolymath/standards/.github/workflows/.*@main' .github/workflows/ 2>/dev/null | grep -q .",
        severity = 'critical,
      },
      {
        id = "hypatia-and-static-gates-present",
        section = "Security Scanners",
        description = "Hypatia verdict grid workflow and Static Analysis Gate workflow are both present",
        probe = "test -f .github/workflows/hypatia-scan.yml && test -f .github/workflows/static-analysis-gate.yml",
        severity = 'critical,
      },
      {
        id = "rfc9116-security-txt",
        section = "Disclosure",
        description = "RFC 9116 .well-known/security.txt and SECURITY.md exist",
        probe = "test -f .well-known/security.txt && test -f SECURITY.md",
        severity = 'high,
      },
      {
        id = "no-dockerfile",
        section = "Container Security",
        description = "Podman Containerfile is used rather than Dockerfile",
        probe = "test -f Containerfile && test ! -f Dockerfile",
        severity = 'medium,
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
    severity_acknowledged   | [| 'critical, 'high, 'medium, 'low |],
    waived_risk_description | String,
  },

  execution = {
    triggers = [ 'session_close, 'on_demand, 'pre_push, 'pre_commit_security_adjacent ],
    per_verification = {
      run_probe = true,
      record_outcome = true,
      respect_variance = true,
      on_unmet = 'fail,
      severity_escalation = 'honour,
    },
    per_probe = {
      run_probe = true,
      record_outcome = true,
      honour_expected_outcome = true,
      on_unexpected_exploit_success = 'fail,
      scope_enforcement = 'this_repo_only,
      timeout_honouring = 'strict,
    },
    evidence_sinks = [
      {
        kind       = 'verisimdb,
        table      = "contractile_executions",
        schema     = "contractile_execution_v1",
        aux_tables = [ "trust_verifications", "trust_probes" ],
      },
      {
        kind        = 'drift_log,
        path        = ".machine_readable/descriptiles/drift_praxis.deed",
        append_only = true,
      },
    ],
    on_close = {
      re_execute_all_verifications = true,
      re_run_all_safe_hacking_probes = true,
      diff_against_last_ratification = true,
      emit_drift_entries_for_new_failures = true,
      surface_expired_variances = true,
      block_session_close_on_critical_drift = true,
    },
    on_open = {
      render_summary = 'plain_language,
      include_drift_log_from_last_close = true,
      include_active_variances = true,
      include_recent_anchors = true,
      anchor_lookback_weeks = 8,
      threat_model_foregrounding = {
        required = true,
        render_adversaries = true,
        render_stakes = true,
        render_compliance_regimes = true,
        render_audience_sensitivity = true,
        re_render_before_weakening_suggestion = true,
      },
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
            pledge = "I accept accountability for declared security and supply-chain obligations.",
            signature_required = true,
          },
          {
            role = 'ai_agent,
            pledge = "I will hold declared trust boundaries and refuse silent security weakening.",
            signature_required = true,
          },
        ],
        signed_record_destination = ".machine_readable/descriptiles/ratification-<session-id>_praxis.deed",
        must_precede_work = true,
      },
      ratification_record_shape = {
        includes_negotiation_transcript = true,
        includes_both_pledges = true,
        includes_threat_model_snapshot = true,
        signed = true,
        dated = true,
        session_id = 'required,
        contract_hash = 'required,
      },
    },
  },

  failure_mode_defenses = [
    'A1_enthusiasm_capture,
    'A2_metaphor_capture,
    'B1_threat_model_misclass,
    'B2_audience_sensitivity_collapse,
    'B3_compliance_prior_drift,
    'C2_capability_collapse,
    'C3_helpfulness_inflation,
    'C4_modernization_drift,
    'D4_error_hiding,
    'D5_sycophancy,
    'D6_false_pessimism,
    'E4_cargo_cult_security,
    'F1_across_session_forgetting,
  ],
}

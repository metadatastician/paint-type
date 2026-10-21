K9!
# SPDX-License-Identifier: AGPL-3.0-or-later
# intend.k9.ncl — K9 trust-tier component of the intend trident for paint-type
# Author: Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
#
# Conforms to K9 Contract Specification v1.0.0 (standards#1058, ruling D173).

let base = import "../_base.ncl" in

{
  pedigree = base.pedigree_schema & {
    contractile_verb = "intend",
    schema_version   = "1.0.0",
    component_type   = "contractile:intend",
    semantics        = "north-star (commitments + aspirations)",
    side_effects     = [],
    paired_xfile     = "../intend/intend.k9.ncl#declaration",
    paired_runner    = "../intend/intend.ncl",

    tier      = 'Yard,
    authority = 'reporting,

    metadata = {
      name = "paint-type-intend-k9",
      version = "2.0.0",
      description = "North-star commitments and horizon aspirations for paint.type. Non-gating reporting authority.",
      paired_xfile = "intend.k9.ncl#declaration",
      paired_runner = "intend.ncl",
      author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },

    security = {
      leash = 'Yard,
      signature_required = false,
      trust_level = "subprocess + filesystem-read",
      allow_network = false,
      allow_filesystem_write = false,
      allow_subprocess = false,
    },

    trident = {
      notes = "Canonical intend trident for paint-type. Pairs declaration in intend.k9.ncl with runner schema in intend.ncl.",
      trident_version = "1.0.0",
      semantics = "north-star (commitments + aspirations)",
      cardinality = "one per repo",
      files = [],
      signed_by = {
        user = "Jonathan D.A. Jewell",
        date = "2026-10-21",
        context = "paint-type intend trident under canon 2.1.2 and K9 contract v1.0.0.",
      },
      history = [
        { date = "2026-10-21", event = "k9-v1-cutover", note = "Migrated paint-type intend contractile to K9 v1.0.0 trident shape with repo-specific milestones and wishes." },
      ],
    },
  },

  declaration = {
    preamble = "# SPDX-License-Identifier: AGPL-3.0-or-later\n# Intentfile — north-star contractile for paint-type\n",
    abstract = "North-star commitments and horizon wishes for paint.type: the open, cross-platform image editor with formally verified Idris2 ABI tile invariants, zero-copy Zig C-ABI seam, linear-resource Rust paint_core, and Gossamer desktop shell.",
    prose = [
      {
        section = "Purpose",
        text = "Deliver a fast, intuitive, zero-surprise raster image editor with Paint.NET ergonomics, lossless 64x64 RGBA16F tiled compositing, unlimited bounded-memory undo, verified WASM plugins, and optional Groove collaboration.",
      },
      {
        section = "Anti-Purpose",
        text = "paint.type is NOT:\n- A GIMP or Photoshop menu maze\n- A browser-only cloud subscription editor\n- A generative AI toy that modifies canvases without explicit user intent\n- A system that runs LLMs inside CI gates",
      },
    ],
    intents = [
      {
        id = "verified-abi-ffi-tile-core",
        section = "Committed Next-Actions",
        description = "Maintain complete Idris2 ABI proofs, Zig C-ABI bridge, and Rust 64x64 RGBA16F tile engine",
        probe = "test -f src/interface/Abi/Types.idr && test -f src/interface/ffi/src/main.zig && test -f src/paint_core/Cargo.toml",
        status = 'done,
        notes = "Verified continuously by ABI/FFI & Safety Aspect Gate and E2E Canvas Harness.",
      },
      {
        id = "canon-2-1-2-verdict-grids",
        section = "Committed Next-Actions",
        description = "Ship canonical DEED, K9 v1.0.0 contractiles, coordination.k9 v1.2.0, K9 verdict grid, and Hypatia verdict grid",
        probe = "test -f paint-type_chora.deed && test -f coordination.k9 && test -f .gitbot-fleet/enrolment.json",
        status = 'done,
        notes = "Enforced on every PR via dogfood-gate.yml and hypatia-scan.yml.",
      },
      {
        id = "gossamer-interactive-tool-palette",
        section = "Committed Next-Actions",
        description = "Complete interactive tool palette and layer inspector polish in Gossamer WebKitGTK shell",
        probe = "test -d src/shell && test -d src/ui",
        status = 'in_progress,
      },
    ],
    wishes = [
      {
        id = "live-gitbot-report-backs",
        section = "Wishes",
        group = "Near Horizon",
        description = "Automated gitbot-fleet report-backs under .gitbot-fleet/report-backs/ turning orange rows blue/green on scheduled sweeps",
        horizon = 'near,
        status = 'declared,
      },
      {
        id = "gpu-compute-compositor-fastpath",
        section = "Wishes",
        group = "Mid Horizon",
        description = "Optional WebGPU/Vulkan tile compositing backend with bit-exact fallback to verified CPU Porter-Duff",
        horizon = 'mid,
        status = 'declared,
      },
      {
        id = "verified-wasm-effect-ecosystem",
        section = "Wishes",
        group = "Far Horizon",
        description = "Sandboxed typed-wasm filter ecosystem with capability manifests and zero ambient I/O",
        horizon = 'far,
        status = 'declared,
      },
    ],
  },

  variance_schema | not_exported = {
    entry_id     | String,
    reason       | String,
    approved_by  | String,
    scope        | String,
    expires      | String,
    review_notes | String | optional,
  },

  execution = {
    triggers = [ 'session_close, 'on_demand, 'pre_push ],
    per_intent = {
      run_probe = true,
      record_outcome = true,
      respect_variance = true,
      on_unmet = 'log_drift,
    },
    per_wish = {
      run_probe = false,
      emit_horizon_summary = true,
      check_alignment_with_META = true,
    },
    evidence_sinks = [
      {
        kind   = 'verisimdb,
        table  = "contractile_executions",
        schema = "contractile_execution_v1",
      },
      {
        kind        = 'drift_log,
        path        = ".machine_readable/descriptiles/drift_praxis.deed",
        append_only = true,
      },
    ],
    on_close = {
      re_execute_all_intents = true,
      diff_against_last_ratification = true,
      emit_drift_entries_for_new_failures = true,
      surface_expired_variances = true,
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
            pledge = "I accept accountability for declared invariants and milestones.",
            signature_required = true,
          },
          {
            role = 'ai_agent,
            pledge = "I will hold declared invariants and surface drift explicitly at session close.",
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
    'A2_metaphor_capture,
    'A3_allegory_drift,
    'C1_scope_creep,
    'C3_helpfulness_inflation,
    'C4_modernization_drift,
    'D5_sycophancy,
    'F1_across_session_forgetting,
  ],
}

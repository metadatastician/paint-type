K9!
# SPDX-License-Identifier: AGPL-3.0-or-later
# Thin coordination bindings for central session-management standards
#
# Upgraded to K9 Contract Specification v1.0.0 (standards#1058, ruling D173)
# while preserving all 12 canonical session commands and 12 signal bindings.
#
# Companion files:
#   - `coordination.k9` (repo-root K9 coordination nerve receptor, v1.2.0 ABNF)
#   - `coordination.self-validating` (legacy YAML binding retained for session/dispatch.sh)

{
  pedigree = {
    schema_version = "1.0.0",
    component_type = "coordination:session",
    semantics = "repo-local session-management command and signal bindings",
    metadata = {
      name = "paint-type-session-coordination",
      version = "1.0.0",
      description = "Thin session-management bindings mapping the 12 canonical session lifecycle commands onto paint-type signals",
      author = "Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>",
    },
    security = {
      leash = 'Kennel,
      capabilities = [],
      side_effects = [],
    },
  },

  session_management = {
    source_of_truth = "standards/3-practice/session-management-standards",
    canonical_commands = [
      "intake repo <path>",
      "checkpoint change <path>",
      "verify maintenance <path>",
      "verify substantial <path>",
      "verify release <path>",
      "close planned <path>",
      "close urgent <path>",
      "recover repo <path>",
      "handover full <path>",
      "handover split <path>",
      "handover model <path>",
      "handover human <path>",
    ],
  },

  signals = [
    { name = "session.intake", command = "intake repo <path>" },
    { name = "session.checkpoint", command = "checkpoint change <path>" },
    { name = "session.verify.maintenance", command = "verify maintenance <path>" },
    { name = "session.verify.substantial", command = "verify substantial <path>" },
    { name = "session.verify.release", command = "verify release <path>" },
    { name = "session.close.planned", command = "close planned <path>" },
    { name = "session.close.urgent", command = "close urgent <path>" },
    { name = "session.recover", command = "recover repo <path>" },
    { name = "session.handover.full", command = "handover full <path>" },
    { name = "session.handover.split", command = "handover split <path>" },
    { name = "session.handover.model", command = "handover model <path>" },
    { name = "session.handover.human", command = "handover human <path>" },
  ],
}

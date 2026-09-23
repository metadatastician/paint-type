K9!
# SPDX-License-Identifier: AGPL-3.0-or-later
# Thin coordination bindings for central session-management standards
#
# ADDED 2026-09-23. rsr-template-repo/.machine_readable/root-allow.txt requires
# `coordination.k9.ncl` at the repo root ("repo-local session binding
# (template-mandated)"). paint-type had `coordination.self-validating` instead —
# the same bindings, but YAML rather than Nickel and under a name the allowlist
# does not list.
#
# This is a faithful conversion of that YAML: the same 12 canonical commands and
# the same 12 signals, verified programmatically to be identical. One value was
# corrected in the process:
#
#   source_of_truth was "standards/session-management-standards"
#   now             "standards/3-practice/session-management-standards"
#
# The path in hyperpolymath/standards is under 3-practice/, so the old value
# pointed at a directory that does not exist.
#
# The old `coordination.self-validating` is deliberately LEFT IN PLACE:
# `session/dispatch.sh` may read it, and removing a file something reads is not
# a rename. Retiring it is a separate change with its own check.

{
  pedigree = {
    schema_version = "1.0.0",
    metadata = {
      name = "session-coordination",
      version = "0.1.0",
    },
    security = {
      leash = 'Kennel,
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

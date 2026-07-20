// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Optional LLM assistant channel for collaborative paint.type sessions.
//
// The milestone is explicit: the LLM channel is "an assistant rather than an
// authority", "off-by-default", and "assistant messages never bypass the
// permission model". This module encodes exactly those three constraints:
//
//   1. OFF BY DEFAULT  — `LlmChannel::default()` is disabled; every assistant
//      action errors with `Disabled` until explicitly `enable()`d.
//   2. NOT AN AUTHORITY — the assistant acts as a normal peer with its own
//      `PeerId`. Any canvas mutation it proposes is produced as an ordinary
//      `Op` that the session applies through the SAME permission gate as a
//      human peer (`Session::receive`). It holds only the capabilities the
//      host granted its peer id — typically none, or `Read` for "suggest only".
//   3. NEVER BYPASSES PERMISSIONS — `propose_paint` checks the assistant's
//      `Paint` capability up front and refuses to even form an `Op` if absent,
//      so a denied assistant cannot enqueue work that some later code path
//      might apply unchecked.
//
// ─── STATUS: gating is real & tested; the live backend is a stub ────────────
// The live assistant rides the boj-server MCP gateway. That gateway is
// upstream and (per the issue) deferred "until [it] exposes a stable cartridge
// surface for paint-type", so `BojGateway` here is a documented stub that
// returns `GatewayUnavailable`. The PERMISSION GATING — the security-relevant
// part — is fully implemented and tested independent of the backend.

use crate::crdt::{PeerId, Rgba};
use crate::permission::{Capability, CapabilityError, PermissionTable};
use crate::session::Op;

/// A request the assistant makes of the session. Assistant output is never
/// applied directly; it is turned into a normal `Op` only after passing the
/// permission gate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssistantAction {
    /// Suggest painting a pixel. Becomes an `Op::Paint` iff the assistant peer
    /// holds `Paint`.
    Paint {
        coord: (i32, i32),
        index: usize,
        value: Rgba,
        lamport: u64,
    },
}

/// Why an assistant action did not produce an applicable op.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LlmError {
    /// The channel is disabled (the default). Enable it explicitly to use it.
    Disabled,
    /// The assistant peer lacks the capability the action needs.
    Denied(CapabilityError),
    /// The live boj-server MCP gateway is not wired in this build.
    GatewayUnavailable,
}

impl core::fmt::Display for LlmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LlmError::Disabled => write!(f, "LLM channel is disabled (off by default)"),
            LlmError::Denied(e) => write!(f, "assistant action denied: {e}"),
            LlmError::GatewayUnavailable => {
                write!(f, "boj-server MCP gateway not wired in this build")
            }
        }
    }
}

impl std::error::Error for LlmError {}

/// The optional assistant channel. The assistant is modelled as a peer with a
/// stable `PeerId`; whatever it does is gated by that peer's capabilities.
#[derive(Clone, Debug)]
pub struct LlmChannel {
    enabled: bool,
    assistant_peer: PeerId,
}

impl LlmChannel {
    /// Create a channel for an assistant identified by `assistant_peer`.
    /// DISABLED until `enable()` is called (off by default).
    pub fn new(assistant_peer: PeerId) -> LlmChannel {
        LlmChannel {
            enabled: false,
            assistant_peer,
        }
    }

    /// Is the channel enabled?
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// The assistant's peer identity (whose capabilities gate its actions).
    pub fn assistant_peer(&self) -> PeerId {
        self.assistant_peer
    }

    /// Turn the channel on. Even enabled, the assistant can only do what its
    /// peer id is granted in the `PermissionTable`.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Turn the channel off.
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Translate an assistant action into a permission-checked `Op`.
    ///
    /// Order of checks (fail-closed):
    ///   * disabled            ⇒ `Disabled`
    ///   * assistant lacks cap ⇒ `Denied` (no op is formed)
    ///   * otherwise           ⇒ `Ok(op)` for the session to broadcast/apply
    ///     through its own gate as well (defence in depth).
    pub fn propose(
        &self,
        perms: &PermissionTable,
        action: AssistantAction,
    ) -> Result<Op, LlmError> {
        if !self.enabled {
            return Err(LlmError::Disabled);
        }
        match action {
            AssistantAction::Paint {
                coord,
                index,
                value,
                lamport,
            } => {
                perms
                    .check(self.assistant_peer, Capability::Paint)
                    .map_err(LlmError::Denied)?;
                Ok(Op::Paint {
                    coord,
                    index,
                    write: crate::crdt::VPixel::write(lamport, self.assistant_peer, value),
                })
            }
        }
    }
}

/// The live boj-server MCP gateway binding — a stub. See module docs.
pub struct BojGateway {
    endpoint: String,
}

impl BojGateway {
    pub fn new(endpoint: impl Into<String>) -> BojGateway {
        BojGateway {
            endpoint: endpoint.into(),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Request an assistant action from the gateway.
    ///
    /// TODO(live): open an MCP session to boj-server, send the canvas context,
    /// receive a proposed `AssistantAction`. Returns `GatewayUnavailable`
    /// until the cartridge surface stabilises upstream.
    pub fn request_action(&self) -> Result<AssistantAction, LlmError> {
        Err(LlmError::GatewayUnavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::CapabilitySet;

    fn perms_with(peer: PeerId, caps: CapabilitySet) -> PermissionTable {
        let mut t = PermissionTable::new();
        t.grant(peer, caps);
        t
    }

    #[test]
    fn off_by_default() {
        let ch = LlmChannel::new(PeerId(900));
        assert!(!ch.is_enabled());
        let perms = perms_with(PeerId(900), CapabilitySet::editor());
        let r = ch.propose(
            &perms,
            AssistantAction::Paint {
                coord: (0, 0),
                index: 0,
                value: [1, 1, 1, 1],
                lamport: 1,
            },
        );
        assert_eq!(r, Err(LlmError::Disabled));
    }

    #[test]
    fn enabled_but_unpermissioned_assistant_is_denied() {
        let mut ch = LlmChannel::new(PeerId(900));
        ch.enable();
        // Assistant peer granted Read only — no Paint.
        let perms = perms_with(PeerId(900), CapabilitySet::observer());
        let r = ch.propose(
            &perms,
            AssistantAction::Paint {
                coord: (0, 0),
                index: 0,
                value: [1, 1, 1, 1],
                lamport: 1,
            },
        );
        assert!(matches!(r, Err(LlmError::Denied(_))));
    }

    #[test]
    fn enabled_and_permitted_assistant_produces_gated_op() {
        let mut ch = LlmChannel::new(PeerId(900));
        ch.enable();
        let perms = perms_with(PeerId(900), CapabilitySet::editor());
        let op = ch
            .propose(
                &perms,
                AssistantAction::Paint {
                    coord: (2, 3),
                    index: 7,
                    value: [5, 6, 7, 8],
                    lamport: 4,
                },
            )
            .expect("permitted assistant should produce an op");
        // The op is attributed to the assistant peer, so the session's own
        // receive-gate will re-check it too.
        assert_eq!(op.originator(), PeerId(900));
    }

    #[test]
    fn gateway_is_unavailable_stub() {
        let g = BojGateway::new("mcp://localhost/boj");
        assert_eq!(g.endpoint(), "mcp://localhost/boj");
        assert_eq!(g.request_action(), Err(LlmError::GatewayUnavailable));
    }
}

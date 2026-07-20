// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Live transport binding for collaborative sessions: WebRTC over Burble.
//
// ─── STATUS: SCAFFOLD ───────────────────────────────────────────────────────
// The session layer (`session.rs`) is transport-agnostic and is fully
// exercised in-process by `session::sim::SimTransport`. This module is the
// place the LIVE transport plugs in: a `Transport` whose `broadcast`/
// `drain_inbox` move `Op`s across a Burble-brokered WebRTC data channel
// (the same encrypted P2P link Burble already carries voice + the AI channel
// over — see `hyperpolymath/burble`).
//
// It is a SCAFFOLD on purpose. Standing up a real sub-10ms two-peer WebRTC
// session needs a running Burble signaling endpoint, a browser/native WebRTC
// stack, and ICE/DTLS negotiation — none of which can be stood up or
// latency-measured inside this build/test sandbox. Rather than fake that,
// `BurbleTransport` returns a clear `NotWired` error from any operation that
// would require the live link, and documents exactly what each method must do
// once the binding lands. NO method panics; the type is safe to construct and
// hold. The acceptance criterion "two-peer session establishes within 2s,
// <10ms p95 edit latency" is therefore explicitly OUT of scope for this pass
// and tracked as live-integration work (see the PR body / ROADMAP).
//
// Upstream contract this scaffold targets (Burble `docs/PROTOCOL.md`):
//   * Signaling/presence : Phoenix Channels (`burble_web/channels/`)
//   * Media/data plane   : WebRTC SRTP / DataChannel (SFU-blind)
//   * Local bridge       : `client/web/burble-ai-bridge.js` on ws://127.0.0.1:6475
// paint-type would open a dedicated `paint-edit` DataChannel alongside the
// existing AI channel and frame `Op`s onto it.

use crate::session::{Op, Transport};

/// Connection parameters for a Burble-brokered WebRTC session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BurbleConfig {
    /// Burble signaling base URL (Phoenix Channels endpoint).
    pub signaling_url: String,
    /// Room code shared out-of-band with the other peer(s).
    pub room_code: String,
    /// Local AI/data bridge websocket (default `ws://127.0.0.1:6475`).
    pub bridge_ws: String,
}

impl Default for BurbleConfig {
    fn default() -> Self {
        BurbleConfig {
            signaling_url: "https://localhost/socket".to_string(),
            room_code: String::new(),
            bridge_ws: "ws://127.0.0.1:6475".to_string(),
        }
    }
}

/// Why a live-transport operation could not complete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransportError {
    /// The live Burble/WebRTC binding is not present in this build. Use
    /// `session::sim::SimTransport` for in-process collaboration.
    NotWired,
}

impl core::fmt::Display for TransportError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TransportError::NotWired => write!(
                f,
                "Burble WebRTC transport is not wired in this build; \
                 the live data channel requires a running Burble bridge \
                 (ws://127.0.0.1:6475) and signaling endpoint"
            ),
        }
    }
}

impl std::error::Error for TransportError {}

/// The live WebRTC-over-Burble transport. Holds config and a local out/in
/// staging buffer; the actual data-channel wiring is the unimplemented part.
pub struct BurbleTransport {
    config: BurbleConfig,
    /// Ops the session asked us to broadcast but that we could not ship
    /// because the channel is not wired. Retained (not dropped) so a future
    /// `flush()` after the channel is established can drain them.
    pending_out: Vec<Op>,
}

impl BurbleTransport {
    /// Construct an (unconnected) transport from config. Infallible and
    /// panic-free; connection is attempted lazily.
    pub fn new(config: BurbleConfig) -> BurbleTransport {
        BurbleTransport {
            config,
            pending_out: Vec::new(),
        }
    }

    /// The configured connection parameters.
    pub fn config(&self) -> &BurbleConfig {
        &self.config
    }

    /// Whether the live data channel is established. Always `false` in this
    /// scaffold build.
    pub fn is_connected(&self) -> bool {
        false
    }

    /// Establish the WebRTC data channel via Burble signaling.
    ///
    /// TODO(live): perform Phoenix-Channel join on `signaling_url`, exchange
    /// SDP offer/answer + ICE candidates for the room, open the `paint-edit`
    /// DataChannel, and mark connected. Returns `NotWired` until implemented.
    pub fn connect(&mut self) -> Result<(), TransportError> {
        Err(TransportError::NotWired)
    }

    /// Ops that were broadcast while disconnected (would be flushed on connect).
    pub fn pending(&self) -> &[Op] {
        &self.pending_out
    }
}

impl Transport for BurbleTransport {
    fn broadcast(&mut self, op: Op) {
        // Not connected: stage the op rather than drop it, so semantics are
        // honest (no silent loss) and a future flush can ship it.
        self.pending_out.push(op);
    }

    fn drain_inbox(&mut self) -> Vec<Op> {
        // No live channel ⇒ nothing ever arrives in this build.
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaffold_is_constructible_and_not_connected() {
        let t = BurbleTransport::new(BurbleConfig {
            room_code: "abc-123".into(),
            ..Default::default()
        });
        assert!(!t.is_connected());
        assert_eq!(t.config().room_code, "abc-123");
    }

    #[test]
    fn connect_reports_not_wired_without_panicking() {
        let mut t = BurbleTransport::new(BurbleConfig::default());
        assert_eq!(t.connect(), Err(TransportError::NotWired));
    }

    #[test]
    fn broadcast_while_disconnected_stages_not_drops() {
        use crate::crdt::{PeerId, VPixel};
        let mut t = BurbleTransport::new(BurbleConfig::default());
        t.broadcast(Op::Paint {
            coord: (0, 0),
            index: 0,
            write: VPixel::write(1, PeerId(1), [1, 2, 3, 4]),
        });
        assert_eq!(t.pending().len(), 1);
        assert!(t.drain_inbox().is_empty());
    }
}

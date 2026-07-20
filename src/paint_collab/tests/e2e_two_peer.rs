// SPDX-License-Identifier: AGPL-3.0-or-later
//
// End-to-end two-peer collaboration scenario (TEST-NEEDS P3:
// "Collaboration session tests: two peers, tile mutation, CRDT merge
// verification").
//
// This exercises the FULL session path that does not require a live network,
// over the in-process `SimTransport`:
//   1. Host announces a session; peer discovers it via Groove (`.well-known`).
//   2. Host grants the joining peer editor capabilities.
//   3. Both peers paint concurrently (conflicting writes to the same cell, and
//      independent writes to others).
//   4. Ops are exchanged in a deliberately reordered + duplicated interleaving.
//   5. Both replicas converge to identical canvases (CRDT merge).
//   6. A read-only peer's paint is denied with a capability error (no silent
//      success, nothing broadcast).
//   7. The optional LLM assistant is off by default and, when enabled, still
//      passes the permission gate.
//
// The live, latency-measured WebRTC variant (Burble transport, <10ms p95) is
// out of scope for CI and tracked separately; this scenario validates the
// collaboration *semantics* deterministically.

use paint_collab::crdt::PeerId;
use paint_collab::groove::{paint_type_manifest, Discovery, SimDiscovery};
use paint_collab::llm::{AssistantAction, LlmChannel, LlmError};
use paint_collab::permission::{Capability, CapabilitySet, PermissionTable};
use paint_collab::session::{
    sim::{Network, SimTransport},
    Op, Session, SessionError, Transport,
};

const HOST: u64 = 1;
const GUEST: u64 = 2;
const OBSERVER: u64 = 3;
const ASSISTANT: u64 = 900;

fn session_perms() -> PermissionTable {
    let mut t = PermissionTable::new();
    t.grant(PeerId(HOST), CapabilitySet::host());
    t.grant(PeerId(GUEST), CapabilitySet::editor());
    t.grant(PeerId(OBSERVER), CapabilitySet::observer());
    t
}

#[test]
fn two_peer_session_converges_under_reordering() {
    // ── 1. Discovery ───────────────────────────────────────────────────────
    let mut directory = SimDiscovery::new();
    directory.announce("room-paint-42", paint_type_manifest());
    let found = directory.discover();
    assert_eq!(found.len(), 1, "guest should discover the host's session");
    assert_eq!(found[0].service_id, "paint-type");
    assert!(found[0].manifest.validate().is_ok());

    // ── 2/3. Two editor peers paint concurrently ───────────────────────────
    let net = Network::new();
    let mut host = Session::new(PeerId(HOST), session_perms());
    let mut guest = Session::new(PeerId(GUEST), session_perms());
    let mut host_tx = SimTransport::new(HOST, vec![HOST, GUEST], net.clone());
    let mut guest_tx = SimTransport::new(GUEST, vec![HOST, GUEST], net.clone());

    let coord = (0, 0);
    // Conflict: both write cell 100.
    let host_op = host
        .local_paint(coord, 100, [0xFF, 0, 0, 0xFF], &mut host_tx)
        .expect("host may paint");
    let guest_op = guest
        .local_paint(coord, 100, [0, 0xFF, 0, 0xFF], &mut guest_tx)
        .expect("guest may paint");
    // Independent writes elsewhere.
    host.local_paint(coord, 1, [1, 1, 1, 1], &mut host_tx)
        .unwrap();
    guest
        .local_paint(coord, 4096 - 1, [2, 2, 2, 2], &mut guest_tx)
        .unwrap();

    // ── 4. Reordered + duplicated delivery ─────────────────────────────────
    // Re-inject the conflicting ops a second time (duplicate delivery) to prove
    // idempotence end-to-end.
    host_tx.inject(guest_op);
    guest_tx.inject(host_op);
    host.pump(&mut host_tx);
    guest.pump(&mut guest_tx);
    // Second pump round (any stragglers) — must be a no-op for convergence.
    host.pump(&mut host_tx);
    guest.pump(&mut guest_tx);

    // ── 5. Convergence ─────────────────────────────────────────────────────
    assert_eq!(
        host.canvas_digest(),
        guest.canvas_digest(),
        "replicas must converge after exchanging all ops"
    );
    // Deterministic conflict winner: same lamport (1), GUEST has the higher
    // PeerId, so guest's green pixel wins on both replicas.
    assert_eq!(host.read_pixel(coord, 100).unwrap(), [0, 0xFF, 0, 0xFF]);
    assert_eq!(guest.read_pixel(coord, 100).unwrap(), [0, 0xFF, 0, 0xFF]);

    // ── 6. Permission denial is loud, not silent ───────────────────────────
    let mut observer = Session::new(PeerId(OBSERVER), session_perms());
    let mut obs_tx = SimTransport::new(OBSERVER, vec![HOST, GUEST, OBSERVER], net.clone());
    let denied = observer.local_paint(coord, 0, [9, 9, 9, 9], &mut obs_tx);
    match denied {
        Err(SessionError::Denied(e)) => {
            assert_eq!(e.required, Capability::Paint);
            assert_eq!(e.peer, PeerId(OBSERVER));
        }
        other => panic!("observer paint must be denied, got {other:?}"),
    }
    assert!(
        observer.canvas_digest().is_empty(),
        "denied paint must not mutate"
    );
    // And nothing was broadcast on the observer's behalf.
    let mut sink_tx = SimTransport::new(HOST, vec![HOST], net.clone());
    assert!(
        sink_tx.drain_inbox().is_empty(),
        "denied paint must not enqueue an op"
    );

    // ── 7. LLM assistant: off by default, gated when on ────────────────────
    let mut llm = LlmChannel::new(PeerId(ASSISTANT));
    let action = AssistantAction::Paint {
        coord,
        index: 50,
        value: [7, 7, 7, 7],
        lamport: 99,
    };
    // Off by default.
    assert_eq!(
        llm.propose(host.permissions_mut(), action.clone()),
        Err(LlmError::Disabled)
    );
    // Enabled but assistant peer has no grant ⇒ still denied.
    llm.enable();
    assert!(matches!(
        llm.propose(host.permissions_mut(), action.clone()),
        Err(LlmError::Denied(_))
    ));
    // Granted Paint ⇒ produces an op attributed to the assistant, which the
    // session would itself re-gate on receive.
    host.permissions_mut()
        .grant(PeerId(ASSISTANT), CapabilitySet::editor());
    let op = llm
        .propose(host.permissions_mut(), action)
        .expect("permitted assistant produces an op");
    assert_eq!(op.originator(), PeerId(ASSISTANT));
    if let Op::Paint { write, .. } = op {
        assert_eq!(write.value, [7, 7, 7, 7]);
    } else {
        panic!("expected a paint op");
    }
}

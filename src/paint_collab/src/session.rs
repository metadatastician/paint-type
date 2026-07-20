// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Collaborative session layer for paint.type.
//
// A `Session` is one peer's view of a shared canvas. It owns:
//   * a stable `PeerId` and a Lamport clock,
//   * the local CRDT replica (a map of `CrdtTile`s),
//   * the `PermissionTable` gating every mutating action,
//   * a pluggable `Transport` carrying `Op`s to/from the other peers.
//
// The edit flow is:
//   local_paint ──gate(Paint)──▶ stamp(lamport++,peer) ──▶ apply locally
//                                                       └─▶ broadcast Op
//   receive(Op) ──gate(sender's Paint/LayerMutate)──▶ merge into replica
//
// Because the underlying tile merge is a join-semilattice (`crdt.rs`), the
// receive path is order- and duplicate-insensitive: peers that have seen the
// same set of `Op`s converge, no matter the delivery interleaving. That is the
// property the TLA+ liveness model (`BurbleSession.tla`, CONC-3) abstracts and
// the `proptest` convergence test exercises.
//
// TRANSPORT INDEPENDENCE. The session never names WebRTC, Burble, or Groove.
// It talks to a `Transport` (move `Op`s) and a `Discovery` (find peers). The
// in-process `SimTransport` here is fully runnable and drives the e2e test;
// the live WebRTC binding (`transport::BurbleTransport`) is a documented stub.

use crate::crdt::{CrdtTile, Dot, PeerId, Rgba, VPixel};
use crate::permission::{Capability, CapabilityError, PermissionTable};
use std::collections::BTreeMap;

/// A single replicated mutation, broadcast to every peer. Self-describing:
/// the receiver re-checks the originator's capability before applying, so a
/// forged or over-reaching `Op` cannot bypass the permission model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    /// Write a pixel in a tile. `write.dot.peer` is the claimed originator.
    Paint {
        coord: (i32, i32),
        index: usize,
        write: VPixel,
    },
    /// A layer-stack mutation. Modelled abstractly (the concrete layer op is
    /// `paint_core`'s `pt_layer_*`); here it only needs to be permission-gated
    /// and attributed to an originator for the acceptance test.
    LayerMutate { by: PeerId, seq: u64 },
}

impl Op {
    /// The peer that originated this op (used for the receive-side gate).
    pub fn originator(&self) -> PeerId {
        match self {
            Op::Paint { write, .. } => write.dot.peer,
            Op::LayerMutate { by, .. } => *by,
        }
    }

    /// The capability required to apply this op.
    pub fn required_capability(&self) -> Capability {
        match self {
            Op::Paint { .. } => Capability::Paint,
            Op::LayerMutate { .. } => Capability::LayerMutate,
        }
    }
}

/// Abstract message transport. A real implementation (WebRTC over Burble)
/// ships `Op`s over the encrypted data channel; the in-process `SimTransport`
/// moves them through a shared queue. Kept deliberately tiny so the session
/// logic is testable without a network.
pub trait Transport {
    /// Broadcast `op` to all other peers in the session.
    fn broadcast(&mut self, op: Op);
    /// Drain any `Op`s that have arrived since the last call.
    fn drain_inbox(&mut self) -> Vec<Op>;
}

/// One peer's session state.
pub struct Session {
    peer: PeerId,
    lamport: u64,
    tiles: BTreeMap<(i32, i32), CrdtTile>,
    perms: PermissionTable,
    /// Count of applied layer mutations — a stand-in for the real layer stack,
    /// enough to assert the permission gate fires on `LayerMutate`.
    layer_seq: u64,
}

/// What went wrong applying a mutation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionError {
    /// The acting peer lacked the required capability.
    Denied(CapabilityError),
    /// Pixel index out of range / coord mismatch.
    Tile(crate::crdt::TileMergeError),
}

impl From<CapabilityError> for SessionError {
    fn from(e: CapabilityError) -> Self {
        SessionError::Denied(e)
    }
}
impl From<crate::crdt::TileMergeError> for SessionError {
    fn from(e: crate::crdt::TileMergeError) -> Self {
        SessionError::Tile(e)
    }
}

impl Session {
    /// Start a session as `peer`, with the given permission table.
    pub fn new(peer: PeerId, perms: PermissionTable) -> Session {
        Session {
            peer,
            lamport: 0,
            tiles: BTreeMap::new(),
            perms,
            layer_seq: 0,
        }
    }

    /// This peer's id.
    pub fn peer(&self) -> PeerId {
        self.peer
    }

    /// Mutable access to the permission table (host-side admin).
    pub fn permissions_mut(&mut self) -> &mut PermissionTable {
        &mut self.perms
    }

    /// Read a resolved pixel value, gated on `Read`.
    pub fn read_pixel(
        &self,
        coord: (i32, i32),
        index: usize,
    ) -> Result<Rgba, SessionError> {
        self.perms.check(self.peer, Capability::Read)?;
        match self.tiles.get(&coord) {
            Some(t) => Ok(t.value(index)?),
            None => Ok(crate::crdt::TRANSPARENT),
        }
    }

    fn tile_mut(&mut self, coord: (i32, i32)) -> &mut CrdtTile {
        self.tiles
            .entry(coord)
            .or_insert_with(|| CrdtTile::blank(coord))
    }

    /// Apply a LOCAL paint: gate on `Paint`, stamp a fresh dot, apply locally,
    /// and return the `Op` to broadcast. Returns `Denied` without mutating
    /// anything if this peer may not paint.
    pub fn local_paint(
        &mut self,
        coord: (i32, i32),
        index: usize,
        value: Rgba,
        transport: &mut dyn Transport,
    ) -> Result<Op, SessionError> {
        self.perms.check(self.peer, Capability::Paint)?;
        // Bounds-check before consuming a lamport tick.
        if index >= crate::crdt::TILE_PIXEL_COUNT {
            return Err(SessionError::Tile(
                crate::crdt::TileMergeError::IndexOutOfRange,
            ));
        }
        self.lamport += 1;
        let write = VPixel::write(self.lamport, self.peer, value);
        self.tile_mut(coord).apply(index, write)?;
        let op = Op::Paint {
            coord,
            index,
            write,
        };
        transport.broadcast(op);
        Ok(op)
    }

    /// Apply a LOCAL layer mutation: gate on `LayerMutate`, bump the local
    /// sequence, broadcast.
    pub fn local_layer_mutate(
        &mut self,
        transport: &mut dyn Transport,
    ) -> Result<Op, SessionError> {
        self.perms.check(self.peer, Capability::LayerMutate)?;
        self.layer_seq += 1;
        let op = Op::LayerMutate {
            by: self.peer,
            seq: self.layer_seq,
        };
        transport.broadcast(op);
        Ok(op)
    }

    /// Apply a REMOTE op. Re-checks the *originator's* capability (defence in
    /// depth: a peer that loses `Paint` cannot have queued ops applied), then
    /// merges. Lamport clock advances past observed remote dots so subsequent
    /// local writes dominate — standard Lamport receive rule.
    pub fn receive(&mut self, op: Op) -> Result<(), SessionError> {
        self.perms.check(op.originator(), op.required_capability())?;
        match op {
            Op::Paint {
                coord,
                index,
                write,
            } => {
                if write.dot.lamport > self.lamport {
                    self.lamport = write.dot.lamport;
                }
                self.tile_mut(coord).apply(index, write)?;
            }
            Op::LayerMutate { seq, .. } => {
                if seq > self.layer_seq {
                    self.layer_seq = seq;
                }
            }
        }
        Ok(())
    }

    /// Drain a transport's inbox and apply each op. Ops whose originator has
    /// been denied are dropped (returned in the error list) rather than
    /// applied — the receive gate never silently lets them through.
    pub fn pump(
        &mut self,
        transport: &mut dyn Transport,
    ) -> Vec<(Op, SessionError)> {
        let mut rejected = Vec::new();
        for op in transport.drain_inbox() {
            if let Err(e) = self.receive(op) {
                rejected.push((op, e));
            }
        }
        rejected
    }

    /// A stable fingerprint of the replica's painted state, for convergence
    /// assertions. Two sessions agree iff they have applied the same set of
    /// writes (regardless of order).
    pub fn canvas_digest(&self) -> Vec<((i32, i32), usize, Dot, Rgba)> {
        let mut out = Vec::new();
        for (&coord, tile) in &self.tiles {
            for index in 0..crate::crdt::TILE_PIXEL_COUNT {
                if let Ok(cell) = tile.cell(index) {
                    if cell.dot != Dot::BOTTOM {
                        out.push((coord, index, cell.dot, cell.value));
                    }
                }
            }
        }
        out
    }
}

/// An in-process transport for tests and local multi-peer simulation. Each
/// peer's inbox is a queue; `broadcast` pushes to every *other* peer. A
/// shared `Network` owns the queues so delivery order can be permuted to model
/// reordering and duplication.
pub mod sim {
    use super::Op;
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    /// Shared mailbox set keyed by peer index.
    #[derive(Default)]
    pub struct Network {
        inboxes: BTreeMap<u64, Vec<Op>>,
    }

    impl Network {
        pub fn new() -> Rc<RefCell<Network>> {
            Rc::new(RefCell::new(Network::default()))
        }
    }

    /// A `Transport` endpoint bound to one peer over a shared `Network`.
    pub struct SimTransport {
        me: u64,
        peers: Vec<u64>,
        net: Rc<RefCell<Network>>,
    }

    impl SimTransport {
        /// Create an endpoint for `me` that broadcasts to `peers` (excluding
        /// `me`). Registers `me`'s inbox.
        pub fn new(me: u64, peers: Vec<u64>, net: Rc<RefCell<Network>>) -> SimTransport {
            net.borrow_mut().inboxes.entry(me).or_default();
            SimTransport { me, peers, net }
        }

        /// Inject a raw op directly into `me`'s inbox (used to model duplicate
        /// or out-of-order delivery in tests).
        pub fn inject(&self, op: Op) {
            self.net
                .borrow_mut()
                .inboxes
                .entry(self.me)
                .or_default()
                .push(op);
        }
    }

    impl super::Transport for SimTransport {
        fn broadcast(&mut self, op: Op) {
            let mut net = self.net.borrow_mut();
            for &peer in &self.peers {
                if peer != self.me {
                    net.inboxes.entry(peer).or_default().push(op);
                }
            }
        }

        fn drain_inbox(&mut self) -> Vec<Op> {
            let mut net = self.net.borrow_mut();
            net.inboxes
                .get_mut(&self.me)
                .map(std::mem::take)
                .unwrap_or_default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sim::{Network, SimTransport};
    use super::*;
    use crate::crdt::PeerId;
    use crate::permission::CapabilitySet;

    fn editor_table(peers: &[u64]) -> PermissionTable {
        let mut t = PermissionTable::new();
        for &p in peers {
            t.grant(PeerId(p), CapabilitySet::editor());
        }
        t
    }

    #[test]
    fn two_peer_paint_converges() {
        let net = Network::new();
        let mut a = Session::new(PeerId(1), editor_table(&[1, 2]));
        let mut b = Session::new(PeerId(2), editor_table(&[1, 2]));
        let mut ta = SimTransport::new(1, vec![1, 2], net.clone());
        let mut tb = SimTransport::new(2, vec![1, 2], net.clone());

        a.local_paint((0, 0), 10, [100, 0, 0, 0], &mut ta).unwrap();
        b.local_paint((0, 0), 10, [200, 0, 0, 0], &mut tb).unwrap();

        // Exchange.
        a.pump(&mut ta);
        b.pump(&mut tb);

        // Both peers resolve the same winner (peer 2's write: same lamport 1,
        // higher PeerId).
        assert_eq!(a.canvas_digest(), b.canvas_digest());
        assert_eq!(a.read_pixel((0, 0), 10).unwrap(), [200, 0, 0, 0]);
    }

    #[test]
    fn denied_paint_does_not_mutate_or_broadcast() {
        let net = Network::new();
        let mut t = PermissionTable::new();
        t.grant(PeerId(1), CapabilitySet::observer()); // read-only
        let mut a = Session::new(PeerId(1), t);
        let mut ta = SimTransport::new(1, vec![1, 2], net.clone());

        let r = a.local_paint((0, 0), 0, [1, 1, 1, 1], &mut ta);
        assert!(matches!(r, Err(SessionError::Denied(_))));
        // Nothing painted, nothing queued for peer 2.
        assert!(a.canvas_digest().is_empty());
        let mut tb = SimTransport::new(2, vec![1, 2], net.clone());
        assert!(tb.drain_inbox().is_empty());
    }

    #[test]
    fn receive_gate_rejects_revoked_originator() {
        let net = Network::new();
        // Peer 1 is an editor and paints; peer 2 has NOT granted peer 1 paint.
        let mut a = Session::new(PeerId(1), editor_table(&[1]));
        let mut b = Session::new(PeerId(2), {
            let mut t = PermissionTable::new();
            t.grant(PeerId(2), CapabilitySet::editor());
            // peer 1 is unknown to peer 2 ⇒ no capabilities
            t
        });
        let mut ta = SimTransport::new(1, vec![1, 2], net.clone());
        let mut tb = SimTransport::new(2, vec![1, 2], net.clone());

        a.local_paint((0, 0), 0, [9, 9, 9, 9], &mut ta).unwrap();
        let rejected = b.pump(&mut tb);
        assert_eq!(rejected.len(), 1);
        assert!(matches!(rejected[0].1, SessionError::Denied(_)));
        // peer 2's replica stayed blank — the op was not silently applied.
        assert!(b.canvas_digest().is_empty());
    }
}

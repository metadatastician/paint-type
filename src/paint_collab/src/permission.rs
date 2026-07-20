// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Per-peer permission model for collaborative paint.type sessions.
//
// Every session-mutating action a peer can take is gated by a CAPABILITY.
// The session owner grants each peer a `CapabilitySet`; any action whose
// capability the peer does not hold is REJECTED with a `CapabilityError`
// rather than silently succeeding. This is the v0.5.0 acceptance criterion
// "denied actions return capability errors rather than silently succeeding",
// and it is what keeps the optional LLM assistant an assistant and not an
// authority (`llm.rs`): assistant actions run through the exact same gate.
//
// The capability set is intentionally a small fixed lattice (a bitset), so
// "does peer P hold capability C" is a constant-time, allocation-free check —
// suitable to call on the hot edit path before every applied mutation.

use core::fmt;

/// A capability a peer may hold within a session. These are exactly the five
/// gated actions named in the v0.5.0 acceptance criteria.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    /// Observe canvas state and the edit stream.
    Read,
    /// Apply pixel/brush writes to tiles.
    Paint,
    /// Mutate the layer stack (add/remove/reorder/opacity/visibility).
    LayerMutate,
    /// Invite a new peer into the session.
    Invite,
    /// Remove a peer from the session.
    Kick,
}

impl Capability {
    /// All five capabilities, for iteration/tests.
    pub const ALL: [Capability; 5] = [
        Capability::Read,
        Capability::Paint,
        Capability::LayerMutate,
        Capability::Invite,
        Capability::Kick,
    ];

    #[inline]
    const fn bit(self) -> u8 {
        match self {
            Capability::Read => 1 << 0,
            Capability::Paint => 1 << 1,
            Capability::LayerMutate => 1 << 2,
            Capability::Invite => 1 << 3,
            Capability::Kick => 1 << 4,
        }
    }

    /// Human-readable name used in error messages.
    pub const fn name(self) -> &'static str {
        match self {
            Capability::Read => "read",
            Capability::Paint => "paint",
            Capability::LayerMutate => "layer-mutate",
            Capability::Invite => "invite",
            Capability::Kick => "kick",
        }
    }
}

/// The set of capabilities a single peer holds. A compact bitset over the
/// five `Capability` variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub struct CapabilitySet(u8);

impl CapabilitySet {
    /// The empty set — a peer that may do nothing (not even read).
    pub const NONE: CapabilitySet = CapabilitySet(0);

    /// Build a set from a slice of capabilities.
    #[must_use]
    pub fn of(caps: &[Capability]) -> CapabilitySet {
        let mut bits = 0u8;
        for c in caps {
            bits |= c.bit();
        }
        CapabilitySet(bits)
    }

    /// A read-only observer: `read` only.
    #[must_use]
    pub fn observer() -> CapabilitySet {
        CapabilitySet::of(&[Capability::Read])
    }

    /// A collaborating editor: read + paint + layer-mutate, but no peer admin.
    #[must_use]
    pub fn editor() -> CapabilitySet {
        CapabilitySet::of(&[
            Capability::Read,
            Capability::Paint,
            Capability::LayerMutate,
        ])
    }

    /// The session host: everything.
    #[must_use]
    pub fn host() -> CapabilitySet {
        CapabilitySet::of(&Capability::ALL)
    }

    /// Does this set contain `cap`?
    #[inline]
    #[must_use]
    pub fn has(self, cap: Capability) -> bool {
        self.0 & cap.bit() != 0
    }

    /// Add a capability (returns the updated set).
    #[must_use]
    pub fn with(mut self, cap: Capability) -> CapabilitySet {
        self.0 |= cap.bit();
        self
    }

    /// Remove a capability (returns the updated set).
    #[must_use]
    pub fn without(mut self, cap: Capability) -> CapabilitySet {
        self.0 &= !cap.bit();
        self
    }
}

/// The error returned when a peer attempts an action it lacks the capability
/// for. Carries enough context to surface a precise diagnostic — never a
/// silent failure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapabilityError {
    /// The peer that attempted the action.
    pub peer: crate::crdt::PeerId,
    /// The capability that was required but absent.
    pub required: Capability,
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "peer {} denied: missing capability `{}`",
            self.peer.0,
            self.required.name()
        )
    }
}

impl std::error::Error for CapabilityError {}

/// The session-wide grant table: which capabilities each peer holds.
#[derive(Clone, Debug, Default)]
pub struct PermissionTable {
    grants: std::collections::BTreeMap<crate::crdt::PeerId, CapabilitySet>,
}

impl PermissionTable {
    /// An empty table — no peer may do anything until granted.
    #[must_use]
    pub fn new() -> PermissionTable {
        PermissionTable {
            grants: std::collections::BTreeMap::new(),
        }
    }

    /// Grant (replace) a peer's capability set.
    pub fn grant(&mut self, peer: crate::crdt::PeerId, caps: CapabilitySet) {
        self.grants.insert(peer, caps);
    }

    /// Revoke all of a peer's capabilities (e.g. on kick).
    pub fn revoke(&mut self, peer: crate::crdt::PeerId) {
        self.grants.remove(&peer);
    }

    /// The capabilities a peer currently holds (empty set if unknown).
    #[must_use]
    pub fn caps(&self, peer: crate::crdt::PeerId) -> CapabilitySet {
        self.grants.get(&peer).copied().unwrap_or(CapabilitySet::NONE)
    }

    /// The gate. `Ok(())` iff `peer` holds `required`; otherwise a precise
    /// `CapabilityError`. This is the single chokepoint every mutating action
    /// — local, remote, or LLM-originated — must pass through.
    pub fn check(
        &self,
        peer: crate::crdt::PeerId,
        required: Capability,
    ) -> Result<(), CapabilityError> {
        if self.caps(peer).has(required) {
            Ok(())
        } else {
            Err(CapabilityError { peer, required })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crdt::PeerId;

    #[test]
    fn observer_can_read_but_not_paint() {
        let mut t = PermissionTable::new();
        t.grant(PeerId(1), CapabilitySet::observer());
        assert!(t.check(PeerId(1), Capability::Read).is_ok());
        assert_eq!(
            t.check(PeerId(1), Capability::Paint),
            Err(CapabilityError {
                peer: PeerId(1),
                required: Capability::Paint
            })
        );
    }

    #[test]
    fn editor_paints_but_cannot_kick() {
        let mut t = PermissionTable::new();
        t.grant(PeerId(2), CapabilitySet::editor());
        assert!(t.check(PeerId(2), Capability::Paint).is_ok());
        assert!(t.check(PeerId(2), Capability::LayerMutate).is_ok());
        assert!(t.check(PeerId(2), Capability::Kick).is_err());
        assert!(t.check(PeerId(2), Capability::Invite).is_err());
    }

    #[test]
    fn host_can_do_everything() {
        let mut t = PermissionTable::new();
        t.grant(PeerId(0), CapabilitySet::host());
        for c in Capability::ALL {
            assert!(t.check(PeerId(0), c).is_ok(), "host missing {:?}", c);
        }
    }

    #[test]
    fn unknown_peer_is_denied_everything() {
        let t = PermissionTable::new();
        for c in Capability::ALL {
            assert!(t.check(PeerId(99), c).is_err());
        }
    }

    #[test]
    fn revoke_removes_all_capabilities() {
        let mut t = PermissionTable::new();
        t.grant(PeerId(3), CapabilitySet::host());
        t.revoke(PeerId(3));
        assert!(t.check(PeerId(3), Capability::Read).is_err());
    }

    #[test]
    fn error_message_names_the_capability() {
        let e = CapabilityError {
            peer: PeerId(7),
            required: Capability::LayerMutate,
        };
        assert_eq!(e.to_string(), "peer 7 denied: missing capability `layer-mutate`");
    }
}

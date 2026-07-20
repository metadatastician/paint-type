// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Property-based convergence tests for the CRDT tile merge (`proptest`).
//
// These are the executable counterpart of the Agda proofs in
// `verification/proofs/agda/TileCRDT.agda`:
//   * CONC-1 commutativity   A ⊕ B = B ⊕ A
//   * CONC-2 associativity   A ⊕ (B ⊕ C) = (A ⊕ B) ⊕ C
//   * idempotence / Strong Eventual Consistency: replicas that have observed
//     the same set of writes converge regardless of delivery order OR
//     multiplicity (duplicates).
//
// The Agda proves the laws for all inputs; proptest hammers the actual Rust
// implementation with thousands of randomised cases, guarding against the
// proof and the code drifting apart.

use paint_collab::crdt::{CrdtTile, PeerId, VPixel, TILE_PIXEL_COUNT};
use proptest::prelude::*;

/// A single write: which cell, and a versioned pixel.
#[derive(Clone, Debug)]
struct Write {
    index: usize,
    vpixel: VPixel,
}

/// Strategy for a single write into a tile.
fn write_strategy() -> impl Strategy<Value = Write> {
    (
        0..TILE_PIXEL_COUNT,
        1u64..6,                 // lamport (>=1, small range to force contention)
        0u64..4,                 // peer
        any::<[u16; 4]>(),       // pixel value
    )
        .prop_map(|(index, lamport, peer, value)| Write {
            index,
            vpixel: VPixel::write(lamport, PeerId(peer), value),
        })
}

/// Build a tile by applying writes in the given order.
fn tile_from(writes: &[Write]) -> CrdtTile {
    let mut t = CrdtTile::blank((0, 0));
    for w in writes {
        t.apply(w.index, w.vpixel).expect("index in range");
    }
    t
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(2000))]

    /// Applying the same writes in any order yields the same tile.
    /// (Order-independence ⇒ the merge over a write set is well-defined.)
    #[test]
    fn order_independent(mut writes in proptest::collection::vec(write_strategy(), 0..40),
                         seed in any::<u64>()) {
        let a = tile_from(&writes);
        // Deterministic shuffle driven by `seed`.
        let mut s = seed | 1;
        for i in (1..writes.len()).rev() {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            let j = (s >> 33) as usize % (i + 1);
            writes.swap(i, j);
        }
        let b = tile_from(&writes);
        prop_assert_eq!(a, b);
    }

    /// Duplicate delivery is a no-op (idempotence at the tile level).
    #[test]
    fn duplicate_delivery_is_idempotent(
        writes in proptest::collection::vec(write_strategy(), 0..40)
    ) {
        let once = tile_from(&writes);
        let mut twice = once.clone();
        for w in &writes {
            twice.apply(w.index, w.vpixel).unwrap();   // replay every write
        }
        prop_assert_eq!(once, twice);
    }

    /// CONC-1: tile merge is commutative.
    #[test]
    fn merge_commutative(
        wa in proptest::collection::vec(write_strategy(), 0..30),
        wb in proptest::collection::vec(write_strategy(), 0..30),
    ) {
        let a = tile_from(&wa);
        let b = tile_from(&wb);
        prop_assert_eq!(a.merged(&b), b.merged(&a));
    }

    /// CONC-2: tile merge is associative.
    #[test]
    fn merge_associative(
        wa in proptest::collection::vec(write_strategy(), 0..20),
        wb in proptest::collection::vec(write_strategy(), 0..20),
        wc in proptest::collection::vec(write_strategy(), 0..20),
    ) {
        let a = tile_from(&wa);
        let b = tile_from(&wb);
        let c = tile_from(&wc);
        let left = a.merged(&b).merged(&c);   // (A ⊕ B) ⊕ C
        let right = a.merged(&b.merged(&c));  // A ⊕ (B ⊕ C)
        prop_assert_eq!(left, right);
    }

    /// The combined SEC statement: three replicas that each saw a different
    /// subset of writes, then exchanged everything in arbitrary pairwise
    /// merges, all reach the same state.
    #[test]
    fn three_replica_convergence(
        wa in proptest::collection::vec(write_strategy(), 0..25),
        wb in proptest::collection::vec(write_strategy(), 0..25),
        wc in proptest::collection::vec(write_strategy(), 0..25),
    ) {
        let a = tile_from(&wa);
        let b = tile_from(&wb);
        let c = tile_from(&wc);

        // Replica X: ((a ⊕ b) ⊕ c)
        let x = a.merged(&b).merged(&c);
        // Replica Y: ((c ⊕ a) ⊕ b)
        let y = c.merged(&a).merged(&b);
        // Replica Z: (b ⊕ (c ⊕ a))
        let z = b.merged(&c.merged(&a));

        prop_assert_eq!(&x, &y);
        prop_assert_eq!(&y, &z);
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Conflict-free tile merging for collaborative paint.type sessions.
//
// ============================================================================
// MODEL — state-based CRDT (CvRDT), a product of last-writer-wins registers
// ============================================================================
//
// A collaborative tile is a fixed grid of `TILE_PIXEL_COUNT` cells.  Each cell
// is a *last-writer-wins register* (`VPixel`): it carries the winning write's
// logical timestamp (`Dot`) alongside the pixel value.  The merge of two cells
// keeps the write with the greater key under a TOTAL ORDER on
// `(dot.lamport, dot.peer, value)`:
//
//     a ⊔ b  =  if key(a) >= key(b) then a else b
//
// `⊔` is therefore `max` over a total order, which is — unconditionally —
//   * commutative   a ⊔ b = b ⊔ a            (CONC-1)
//   * associative   a ⊔ (b ⊔ c) = (a ⊔ b) ⊔ c (CONC-2)
//   * idempotent    a ⊔ a = a                 (duplicate-delivery safe)
//
// A tile merge is the POINTWISE lift of `⊔` across all cells.  Pointwise lift
// of a join-semilattice is again a join-semilattice, so tile merge inherits
// all three laws.  Commutativity + associativity + idempotence are exactly the
// Strong-Eventual-Consistency conditions (Shapiro et al. 2011): replicas that
// have observed the same set of writes converge to the same state regardless
// of the order or multiplicity in which merges are applied.
//
// The total-order key is the crux: ties (equal keys) occur ONLY for writes
// that agree on lamport clock, originating peer, AND value — i.e. literally the
// same write — so the tiebreak is irrelevant to convergence.  Including the
// value in the key makes the order total even in the degenerate case, which is
// what lets the algebraic laws hold WITHOUT assuming "equal dot ⇒ equal value".
//
// Mechanised proofs of CONC-1 / CONC-2 live in
// `verification/proofs/agda/TileCRDT.agda` (Agda, builtin-only, no postulates).
// This module is their executable ground truth; `tests/convergence.rs` checks
// the same laws empirically over random write permutations with `proptest`.

/// Width/height of a square tile in pixels (matches `paint_core::TILE_SIZE`).
pub const TILE_SIZE: usize = 64;

/// Number of pixels (cells) in one tile.
pub const TILE_PIXEL_COUNT: usize = TILE_SIZE * TILE_SIZE;

/// A pixel value: four f16 bit patterns (R, G, B, A), matching the
/// `[u16; 4]` representation used throughout `paint_core`.
pub type Rgba = [u16; 4];

/// The fully-transparent pixel (all channels zero), used as the blank value
/// for never-written cells.
pub const TRANSPARENT: Rgba = [0, 0, 0, 0];

/// A peer's stable identity within a session. Distinct peers MUST have
/// distinct ids; this is what guarantees two different replicas can never
/// mint colliding dots for distinct writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PeerId(pub u64);

/// A logical timestamp for a write: a Lamport clock paired with the
/// originating peer. Ordered lexicographically (lamport first, peer second),
/// which is a total order because `PeerId` is totally ordered.
///
/// `lamport == 0` is reserved for the synthetic "never written" dot of a
/// blank cell, so any genuine write (lamport >= 1) dominates a blank.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Dot {
    pub lamport: u64,
    pub peer: PeerId,
}

impl Dot {
    /// The bottom dot — dominated by every genuine write.
    pub const BOTTOM: Dot = Dot {
        lamport: 0,
        peer: PeerId(0),
    };
}

/// A last-writer-wins register for a single pixel cell.
///
/// The CRDT join key is `(dot.lamport, dot.peer, value)` taken lexicographically
/// — a TOTAL order. `Ord` is derived in exactly that field order, so the
/// derived comparison *is* the join key; `merge` is `core::cmp::max` over it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VPixel {
    pub dot: Dot,
    pub value: Rgba,
}

impl VPixel {
    /// The blank cell: transparent, stamped with the bottom dot.
    pub const BLANK: VPixel = VPixel {
        dot: Dot::BOTTOM,
        value: TRANSPARENT,
    };

    /// A cell written by `peer` at logical time `lamport` with `value`.
    #[inline]
    pub fn write(lamport: u64, peer: PeerId, value: Rgba) -> VPixel {
        VPixel {
            dot: Dot { lamport, peer },
            value,
        }
    }

    /// The CRDT join `⊔`: keep the register with the greater total-order key.
    /// `max` is taken over the *derived* `Ord`, i.e. lexicographically over
    /// `(lamport, peer, value)`. Total order ⇒ commutative, associative,
    /// idempotent (see module docs; mechanised in `TileCRDT.agda`).
    #[inline]
    #[must_use]
    pub fn merge(self, other: VPixel) -> VPixel {
        core::cmp::max(self, other)
    }
}

/// A collaborative tile: its grid position plus a register per cell.
///
/// Two tiles are mergeable iff they describe the same grid coordinate. The
/// merge is pointwise `VPixel::merge`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrdtTile {
    /// Grid coordinate (tile-space, not pixel-space).
    pub coord: (i32, i32),
    /// One LWW register per pixel, row-major, length `TILE_PIXEL_COUNT`.
    cells: Vec<VPixel>,
}

/// Error returned when a write or merge is mis-addressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileMergeError {
    /// Pixel index `>= TILE_PIXEL_COUNT`.
    IndexOutOfRange,
    /// Attempted to merge tiles at different grid coordinates.
    CoordMismatch,
}

impl CrdtTile {
    /// A blank tile at `coord` — every cell transparent and stamped bottom.
    #[must_use]
    pub fn blank(coord: (i32, i32)) -> CrdtTile {
        CrdtTile {
            coord,
            cells: vec![VPixel::BLANK; TILE_PIXEL_COUNT],
        }
    }

    /// Read the register at `index`.
    #[inline]
    pub fn cell(&self, index: usize) -> Result<VPixel, TileMergeError> {
        self.cells
            .get(index)
            .copied()
            .ok_or(TileMergeError::IndexOutOfRange)
    }

    /// Read the resolved pixel value at `index`.
    #[inline]
    pub fn value(&self, index: usize) -> Result<Rgba, TileMergeError> {
        Ok(self.cell(index)?.value)
    }

    /// Apply a local or remote write to `index`, joining it with whatever is
    /// already there. Because the cell update is itself the join `⊔`, applying
    /// the same write twice (duplicate delivery) is a no-op, and applying a
    /// stale write (lower key) leaves the cell unchanged.
    pub fn apply(&mut self, index: usize, write: VPixel) -> Result<(), TileMergeError> {
        let slot = self
            .cells
            .get_mut(index)
            .ok_or(TileMergeError::IndexOutOfRange)?;
        *slot = slot.merge(write);
        Ok(())
    }

    /// Merge another replica of this tile into `self`, pointwise. Returns
    /// `CoordMismatch` if the tiles describe different grid coordinates.
    pub fn merge(&mut self, other: &CrdtTile) -> Result<(), TileMergeError> {
        if self.coord != other.coord {
            return Err(TileMergeError::CoordMismatch);
        }
        for (slot, &incoming) in self.cells.iter_mut().zip(other.cells.iter()) {
            *slot = slot.merge(incoming);
        }
        Ok(())
    }

    /// Functional merge — `self ⊔ other` as a fresh tile, leaving both inputs
    /// untouched. Convenient for the algebraic tests.
    #[must_use]
    pub fn merged(&self, other: &CrdtTile) -> CrdtTile {
        let mut out = self.clone();
        // Coord equality is the caller's contract for the algebraic laws; on
        // mismatch we keep `self` unchanged (the laws are stated per-coord).
        let _ = out.merge(other);
        out
    }

    /// Iterator over `(index, value)` for cells that have been written
    /// (i.e. carry a non-bottom dot).
    pub fn painted_cells(&self) -> impl Iterator<Item = (usize, Rgba)> + '_ {
        self.cells.iter().enumerate().filter_map(|(i, c)| {
            if c.dot == Dot::BOTTOM {
                None
            } else {
                Some((i, c.value))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(n: u64) -> PeerId {
        PeerId(n)
    }

    #[test]
    fn later_lamport_wins() {
        let a = VPixel::write(1, p(1), [10, 0, 0, 0]);
        let b = VPixel::write(2, p(1), [20, 0, 0, 0]);
        assert_eq!(a.merge(b).value, [20, 0, 0, 0]);
        assert_eq!(b.merge(a).value, [20, 0, 0, 0]); // commutative
    }

    #[test]
    fn equal_lamport_breaks_by_peer_then_value() {
        // Same lamport, different peers: higher PeerId wins, deterministically.
        let a = VPixel::write(5, p(1), [10, 0, 0, 0]);
        let b = VPixel::write(5, p(2), [20, 0, 0, 0]);
        assert_eq!(a.merge(b), b);
        assert_eq!(b.merge(a), b);
    }

    #[test]
    fn merge_is_idempotent() {
        let a = VPixel::write(3, p(7), [1, 2, 3, 4]);
        assert_eq!(a.merge(a), a);
    }

    #[test]
    fn blank_is_dominated_by_any_write() {
        let w = VPixel::write(1, p(1), [1, 1, 1, 1]);
        assert_eq!(VPixel::BLANK.merge(w), w);
        assert_eq!(w.merge(VPixel::BLANK), w);
    }

    #[test]
    fn tile_pointwise_merge_converges() {
        let mut t1 = CrdtTile::blank((0, 0));
        let mut t2 = CrdtTile::blank((0, 0));
        t1.apply(0, VPixel::write(1, p(1), [9, 0, 0, 0])).unwrap();
        t2.apply(0, VPixel::write(2, p(2), [3, 0, 0, 0])).unwrap();
        t2.apply(5, VPixel::write(1, p(2), [4, 0, 0, 0])).unwrap();

        let a = t1.merged(&t2);
        let b = t2.merged(&t1);
        assert_eq!(a, b); // commutative at tile level
        assert_eq!(a.value(0).unwrap(), [3, 0, 0, 0]); // lamport 2 wins
        assert_eq!(a.value(5).unwrap(), [4, 0, 0, 0]);
    }

    #[test]
    fn merge_rejects_coord_mismatch() {
        let mut t1 = CrdtTile::blank((0, 0));
        let t2 = CrdtTile::blank((1, 0));
        assert_eq!(t1.merge(&t2), Err(TileMergeError::CoordMismatch));
    }

    #[test]
    fn apply_rejects_out_of_range() {
        let mut t = CrdtTile::blank((0, 0));
        assert_eq!(
            t.apply(TILE_PIXEL_COUNT, VPixel::write(1, p(1), [1, 1, 1, 1])),
            Err(TileMergeError::IndexOutOfRange)
        );
    }
}

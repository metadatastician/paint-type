-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
--
-- ============================================================================
-- CONC-1 / CONC-2 : conflict-free tile-merge commutativity & associativity.
-- ============================================================================
--
-- Ground truth:
--   src/paint_collab/src/crdt.rs
--   A collaborative tile is a grid of last-writer-wins registers.  Each cell
--   carries the winning write's order key and value; the merge keeps the cell
--   with the greater key under the TOTAL ORDER on (lamport, peer, value):
--
--       a ⊔ b  =  if key a ≥ key b then a else b          (`VPixel::merge`)
--
--   A tile merge is the POINTWISE lift of ⊔ across all cells
--   (`CrdtTile::merge`).
--
-- Obligations:
--   CONC-1  tile merge is commutative   A ⊕ B ≡ B ⊕ A
--   CONC-2  tile merge is associative   A ⊕ (B ⊕ C) ≡ (A ⊕ B) ⊕ C
--
-- ----------------------------------------------------------------------------
-- MODEL — "max over a total order, lifted pointwise"
-- ----------------------------------------------------------------------------
-- `derive(Ord)` on Rust's `VPixel` makes the cell comparison lexicographic
-- over (lamport, peer, value): a genuine TOTAL ORDER.  Any total order on a
-- finite carrier is order-isomorphic to an initial segment of ℕ, so we model
-- each cell by its ORDER KEY  k : Nat — the value `derive(Ord)`'s comparison
-- realises — and the merge as  _⊔_ = Nat-max.  "Greater key wins" is exactly
-- last-writer-wins; the carried pixel value is the value of whichever write
-- owns the surviving key, a function of the winner, so proving the keys
-- converge proves the cells converge.
--
-- Section 2 discharges the obligation behind that abstraction: it proves the
-- (lamport, peer) lexicographic order really IS a total order — reflexive,
-- antisymmetric, transitive, total.  A total order is precisely what makes
-- "keep the larger" a well-defined and symmetric (hence commutative) merge.
--
-- A tile is the list of its cell keys; the tile merge ⊕ is `zipWith _⊔_`.
-- A pointwise lift of a commutative / associative / idempotent operation is
-- again commutative / associative / idempotent, so CONC-1 and CONC-2 reduce to
-- the semilattice laws of Nat-max (Section 1) threaded through `zipWith`
-- (Section 3).  Commutativity + associativity + idempotence are exactly the
-- Strong-Eventual-Consistency conditions (Shapiro et al. 2011): see the
-- `⊕-converge` permutation corollary (Section 4), the algebraic twin of the
-- `proptest` convergence test in `src/paint_collab/tests/convergence.rs`.
--
-- INV-2 re-verification (no silent discard under concurrent commits): the
-- `⊔-upper-l` / `⊔-upper-r` lemmas show the merged cell DOMINATES both inputs,
-- so a merge never drops a committed write below either side's key.
--
-- ----------------------------------------------------------------------------
-- DISCIPLINE
-- ----------------------------------------------------------------------------
-- Builtin-only (Agda.Builtin.*); NO agda-stdlib.  NO postulate, NO
-- {-# TERMINATING #-}, NO believe_me / assert_total.  Every law is proved.
--     agda --no-libraries verification/proofs/agda/TileCRDT.agda

module TileCRDT where

open import Agda.Builtin.Nat using (Nat; zero; suc)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.List using (List; []; _∷_)

--==============================================================================
-- 0.  Equational + order toolkit (builtin only; all proved here)
--==============================================================================

data ⊥ : Set where

⊥-elim : ∀ {A : Set} → ⊥ → A
⊥-elim ()

data _⊎_ (A B : Set) : Set where
  inl : A → A ⊎ B
  inr : B → A ⊎ B

cong-suc : ∀ {a b} → a ≡ b → suc a ≡ suc b
cong-suc refl = refl

cong : ∀ {A B : Set} (f : A → B) {x y} → x ≡ y → f x ≡ f y
cong f refl = refl

cong-∷ : ∀ {A : Set} {x y : A} {xs ys : List A}
       → x ≡ y → xs ≡ ys → (x ∷ xs) ≡ (y ∷ ys)
cong-∷ refl refl = refl

sym : ∀ {A : Set} {x y : A} → x ≡ y → y ≡ x
sym refl = refl

trans : ∀ {A : Set} {x y z : A} → x ≡ y → y ≡ z → x ≡ z
trans refl q = q

subst : ∀ {A : Set} (P : A → Set) {x y : A} → x ≡ y → P x → P y
subst P refl p = p

-- ≤ on Nat, and the strict order built from it.
data _≤_ : Nat → Nat → Set where
  z≤n : ∀ {n}            → zero  ≤ n
  s≤s : ∀ {m n} → m ≤ n → suc m ≤ suc n

infix 4 _≤_

_<_ : Nat → Nat → Set
m < n = suc m ≤ n

infix 4 _<_

≤-refl : ∀ {n} → n ≤ n
≤-refl {zero}  = z≤n
≤-refl {suc n} = s≤s ≤-refl

≤-suc : ∀ {n} → n ≤ suc n
≤-suc {zero}  = z≤n
≤-suc {suc n} = s≤s ≤-suc

≤-trans : ∀ {a b c} → a ≤ b → b ≤ c → a ≤ c
≤-trans z≤n      _      = z≤n
≤-trans (s≤s p) (s≤s q) = s≤s (≤-trans p q)

≤-antisym : ∀ {a b} → a ≤ b → b ≤ a → a ≡ b
≤-antisym z≤n      z≤n      = refl
≤-antisym (s≤s p) (s≤s q)  = cong-suc (≤-antisym p q)

-- Strict order is irreflexive and asymmetric.
<-irrefl : ∀ {n} → n < n → ⊥
<-irrefl {zero}  ()
<-irrefl {suc n} (s≤s p) = <-irrefl p

<-asym : ∀ {m n} → m < n → n < m → ⊥
<-asym p q = <-irrefl (≤-trans p (≤-trans ≤-suc q))

<-trans : ∀ {m n o} → m < n → n < o → m < o
<-trans p q = ≤-trans p (≤-trans ≤-suc q)

--==============================================================================
-- 1.  Nat-max ⊔ is a join-semilattice: commutative, associative, idempotent
--==============================================================================
--
-- `_⊔_` is the per-cell CRDT join: it keeps the larger order key (the
-- last-writer-wins survivor).  These laws are CONC-1/2 at the cell level;
-- Section 3 lifts them pointwise to whole tiles.

_⊔_ : Nat → Nat → Nat
zero  ⊔ n     = n
suc m ⊔ zero  = suc m
suc m ⊔ suc n = suc (m ⊔ n)

infixl 6 _⊔_

⊔-comm : ∀ (a b : Nat) → a ⊔ b ≡ b ⊔ a
⊔-comm zero    zero    = refl
⊔-comm zero    (suc b) = refl
⊔-comm (suc a) zero    = refl
⊔-comm (suc a) (suc b) = cong-suc (⊔-comm a b)

⊔-assoc : ∀ (a b c : Nat) → (a ⊔ b) ⊔ c ≡ a ⊔ (b ⊔ c)
⊔-assoc zero    b       c       = refl
⊔-assoc (suc a) zero    c       = refl
⊔-assoc (suc a) (suc b) zero    = refl
⊔-assoc (suc a) (suc b) (suc c) = cong-suc (⊔-assoc a b c)

⊔-idem : ∀ (a : Nat) → a ⊔ a ≡ a
⊔-idem zero    = refl
⊔-idem (suc a) = cong-suc (⊔-idem a)

-- ⊔ selects the maximum: it is an upper bound of BOTH inputs (no silent
-- discard — the INV-2 re-verification at cell level).
⊔-upper-l : ∀ (a b : Nat) → a ≤ (a ⊔ b)
⊔-upper-l zero    b       = z≤n
⊔-upper-l (suc a) zero    = ≤-refl
⊔-upper-l (suc a) (suc b) = s≤s (⊔-upper-l a b)

⊔-upper-r : ∀ (a b : Nat) → b ≤ (a ⊔ b)
⊔-upper-r a b = subst (λ x → b ≤ x) (⊔-comm b a) (⊔-upper-l b a)

--==============================================================================
-- 2.  The cell order key really is a TOTAL ORDER (faithfulness of §0)
--==============================================================================
--
-- The Rust cell key is `derive(Ord)` over the dot (lamport, peer).  We prove
-- lexicographic order on (lamport, peer) is reflexive, antisymmetric,
-- transitive, and total — the fact the §0 abstraction stands on.

record Dot : Set where
  constructor dot
  field
    lamport : Nat
    peer    : Nat
open Dot

-- Trichotomy for Nat.
data Tri (m n : Nat) : Set where
  tri< : m < n → Tri m n
  tri≡ : m ≡ n → Tri m n
  tri> : n < m → Tri m n

compare : ∀ (m n : Nat) → Tri m n
compare zero    zero    = tri≡ refl
compare zero    (suc n) = tri< (s≤s z≤n)
compare (suc m) zero    = tri> (s≤s z≤n)
compare (suc m) (suc n) with compare m n
... | tri< p = tri< (s≤s p)
... | tri≡ e = tri≡ (cong-suc e)
... | tri> p = tri> (s≤s p)

≤-total : ∀ (m n : Nat) → (m ≤ n) ⊎ (n ≤ m)
≤-total zero    n       = inl z≤n
≤-total (suc m) zero    = inr z≤n
≤-total (suc m) (suc n) with ≤-total m n
... | inl p = inl (s≤s p)
... | inr q = inr (s≤s q)

-- Lexicographic ≤ on dots: smaller lamport, or equal lamport and ≤ peer.
data _≤d_ : Dot → Dot → Set where
  lam< : ∀ {a b} → lamport a < lamport b                   → a ≤d b
  lam≡ : ∀ {a b} → lamport a ≡ lamport b → peer a ≤ peer b → a ≤d b

infix 4 _≤d_

dot-≡ : ∀ {a b : Dot} → lamport a ≡ lamport b → peer a ≡ peer b → a ≡ b
dot-≡ {dot la pa} {dot lb pb} refl refl = refl

≤d-refl : ∀ (a : Dot) → a ≤d a
≤d-refl a = lam≡ refl ≤-refl

≤d-antisym : ∀ {a b} → a ≤d b → b ≤d a → a ≡ b
≤d-antisym (lam< p)    (lam< q)    = ⊥-elim (<-asym p q)
≤d-antisym (lam< p)    (lam≡ e _)  = ⊥-elim (<-irrefl (subst (λ x → _ < x) e p))
≤d-antisym (lam≡ e _)  (lam< q)    = ⊥-elim (<-irrefl (subst (λ x → _ < x) e q))
≤d-antisym (lam≡ e1 q1)(lam≡ _ q2) = dot-≡ e1 (≤-antisym q1 q2)

≤d-trans : ∀ {a b c} → a ≤d b → b ≤d c → a ≤d c
≤d-trans (lam< p)     (lam< q)     = lam< (<-trans p q)
≤d-trans (lam< p)     (lam≡ e2 _)  = lam< (subst (λ x → _ < x) e2 p)
≤d-trans (lam≡ e1 _)  (lam< q)     = lam< (subst (λ x → suc x ≤ _) (sym e1) q)
≤d-trans (lam≡ e1 q1) (lam≡ e2 q2) = lam≡ (trans e1 e2) (≤-trans q1 q2)

≤d-total : ∀ (a b : Dot) → (a ≤d b) ⊎ (b ≤d a)
≤d-total a b with compare (lamport a) (lamport b)
... | tri< p = inl (lam< p)
... | tri> p = inr (lam< p)
... | tri≡ e with ≤-total (peer a) (peer b)
...   | inl q = inl (lam≡ e q)
...   | inr q = inr (lam≡ (sym e) q)

--==============================================================================
-- 3.  Tiles and the pointwise merge ⊕
--==============================================================================
--
-- A tile is the row-major list of its cell order keys.  The merge ⊕ is the
-- pointwise join `zipWith _⊔_`.  (In the executable model a tile is a fixed
-- 4096-length vector; the laws below hold for lists of any length, so they
-- specialise to that fixed shape.)

Tile : Set
Tile = List Nat

_⊕_ : Tile → Tile → Tile
[]       ⊕ ys       = []
(x ∷ xs) ⊕ []       = []
(x ∷ xs) ⊕ (y ∷ ys) = (x ⊔ y) ∷ (xs ⊕ ys)

infixl 6 _⊕_

⊕-comm : ∀ (xs ys : Tile) → xs ⊕ ys ≡ ys ⊕ xs
⊕-comm []       []       = refl
⊕-comm []       (y ∷ ys) = refl
⊕-comm (x ∷ xs) []       = refl
⊕-comm (x ∷ xs) (y ∷ ys) = cong-∷ (⊔-comm x y) (⊕-comm xs ys)

⊕-assoc : ∀ (xs ys zs : Tile) → (xs ⊕ ys) ⊕ zs ≡ xs ⊕ (ys ⊕ zs)
⊕-assoc []       ys       zs       = refl
⊕-assoc (x ∷ xs) []       zs       = refl
⊕-assoc (x ∷ xs) (y ∷ ys) []       = refl
⊕-assoc (x ∷ xs) (y ∷ ys) (z ∷ zs) = cong-∷ (⊔-assoc x y z) (⊕-assoc xs ys zs)

⊕-idem : ∀ (xs : Tile) → xs ⊕ xs ≡ xs
⊕-idem []       = refl
⊕-idem (x ∷ xs) = cong-∷ (⊔-idem x) (⊕-idem xs)

--==============================================================================
-- 4.  The obligations, and Strong-Eventual-Consistency convergence
--==============================================================================

-- CONC-1 : tile merge is commutative.
CONC-1 : ∀ (A B : Tile) → A ⊕ B ≡ B ⊕ A
CONC-1 = ⊕-comm

-- CONC-2 : tile merge is associative (as stated in the issue:
-- A ⊕ (B ⊕ C) = (A ⊕ B) ⊕ C).
CONC-2 : ∀ (A B C : Tile) → A ⊕ (B ⊕ C) ≡ (A ⊕ B) ⊕ C
CONC-2 A B C = sym (⊕-assoc A B C)

-- Convergence: with ⊕ commutative + associative + idempotent, every replica
-- that has observed the same set of writes reaches the same state regardless
-- of the order in which it merged them.  Concretely, a 3-way reassociation +
-- reversal — the algebraic statement of the `proptest` permutation test.
⊕-converge : ∀ (A B C : Tile) → (A ⊕ B) ⊕ C ≡ (C ⊕ B) ⊕ A
⊕-converge A B C =
  trans (⊕-assoc A B C)
  (trans (cong (λ t → A ⊕ t) (⊕-comm B C))
         (⊕-comm A (C ⊕ B)))

-- Duplicate delivery is absorbed: re-merging an already-merged replica is a
-- no-op (idempotence at the tile level).
⊕-absorb-dup : ∀ (A B : Tile) → (A ⊕ B) ⊕ B ≡ A ⊕ B
⊕-absorb-dup A B = trans (⊕-assoc A B B) (cong (λ t → A ⊕ t) (⊕-idem B))

--==============================================================================
-- 5.  Concrete witnesses (the laws are not vacuous — ⊕ actually computes LWW)
--==============================================================================

-- Pointwise last-writer-wins: max of (3,1) and (2,5) keys = (3,5).
_ : (3 ∷ 1 ∷ []) ⊕ (2 ∷ 5 ∷ []) ≡ (3 ∷ 5 ∷ [])
_ = refl

-- Commutativity holds on the witness too.
_ : (3 ∷ 1 ∷ []) ⊕ (2 ∷ 5 ∷ []) ≡ (2 ∷ 5 ∷ []) ⊕ (3 ∷ 1 ∷ [])
_ = CONC-1 (3 ∷ 1 ∷ []) (2 ∷ 5 ∷ [])

-- A blank cell (key 0) is dominated by any genuine write.
_ : (0 ∷ []) ⊕ (7 ∷ []) ≡ (7 ∷ [])
_ = refl

-- The dot order decides a concrete pair: (lamport 5, peer 1) ≤ (5, 2).
-- Equal lamports, so the peer decides: 1 ≤ 2  ≡  suc zero ≤ suc (suc zero).
_ : dot 5 1 ≤d dot 5 2
_ = lam≡ refl (s≤s z≤n)

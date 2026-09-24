-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
--
-- Typing Proof: Core data type well-formedness
-- Template — replace with your project's core types.
-- All proofs MUST be constructive (no believe_me, no assert_total).

module Types

import Data.Nat

%default total

||| Example: A bounded natural number (0 to max).
||| Replace with your project's core types.
public export
record Bounded (max : Nat) where
  constructor MkBounded
  value : Nat
  ||| Witness that `value` is within `max`.
  |||
  ||| Declared as a plain field, not `{auto 0 ...}`. In Idris2 0.8.0 an
  ||| erased record field has no accessible projection, so the erased form
  ||| made `boundedLeMax` below fail with
  ||| `Bounded.(.inBounds) is not accessible in this context`. Dropping
  ||| `auto` but keeping `0` fails identically, and in the brace form the
  ||| witness becomes an implicit constructor argument, so `MkBounded 0`
  ||| then has type `LTE 0 ?right -> Bounded ?max` rather than
  ||| `Bounded ?max`. A plain field is the only form that both yields a
  ||| projection and keeps `MkBounded 0 LTEZero` well-typed.
  |||
  ||| The consequence is that the witness is retained at runtime. `LTE` is
  ||| a Peano-style unary proof, so this is not free for large `max` -- any
  ||| use of `Bounded` across the FFI should re-check the emitted layout.
  inBounds : LTE value max

||| Proof that a Bounded value is always <= max.
|||
||| `max` is bound explicitly. Left implicit it is auto-bound as a fresh
||| metavariable and Idris2 warns that it shadows `Prelude.EqOrd.max`.
export
boundedLeMax : {max : Nat} -> (b : Bounded max) -> LTE b.value max
boundedLeMax b = b.inBounds

||| Proof that zero is always a valid Bounded value.
export
zeroIsBounded : {max : Nat} -> Bounded (S max)
zeroIsBounded = MkBounded 0 LTEZero

||| Example: A non-empty list with a compile-time guarantee.
public export
data NonEmpty : List a -> Type where
  IsNonEmpty : NonEmpty (x :: xs)

||| Proof that cons always produces a non-empty list.
export
consIsNonEmpty : (x : a) -> (xs : List a) -> NonEmpty (x :: xs)
consIsNonEmpty _ _ = IsNonEmpty

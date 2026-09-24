-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
--
-- ABI Proof: Memory layout correctness
-- Proves struct size, alignment, and padding properties.
-- All proofs MUST be constructive (no believe_me, no assert_total).

module ABI.Layout

-- `NonZero` and `modNatNZ` live in Data.Nat. Without this import the module
-- does not typecheck at all: "While processing type of paddingFor. Undefined
-- name NonZero." It was the only one of the nine proof modules that never
-- compiled, so every property in this file was previously unverified.
import Data.Nat

%default total

||| Witness that a type has a known size in bytes at compile time.
public export
interface HasSize (ty : Type) where
  sizeOf : Nat

||| Witness that a type has a known alignment in bytes.
public export
interface HasAlignment (ty : Type) where
  alignOf : Nat

||| Calculate padding needed to reach the next aligned offset.
||| paddingFor offset alignment = bytes to add so (offset + padding) `mod` alignment == 0
public export
paddingFor : (offset : Nat) -> (alignment : Nat) -> {auto 0 ok : NonZero alignment} -> Nat
paddingFor offset alignment = let r = modNatNZ offset alignment ok
                              in case r of
                                   Z => Z
                                   (S _) => minus alignment r

||| Proof that an offset with zero remainder needs zero padding.
export
alignedNeedsPadding : (n : Nat) -> (a : Nat) -> {auto 0 ok : NonZero a} ->
                      modNatNZ n a ok = 0 -> paddingFor n a = 0
alignedNeedsPadding n a prf = rewrite prf in Refl

||| A field within a struct, carrying its offset and size.
public export
record StructField where
  constructor MkField
  fieldName : String
  fieldOffset : Nat
  fieldSize : Nat
  fieldAlignment : Nat
  ||| Witness that `fieldAlignment` is non-zero, so `modNatNZ` is defined
  ||| for it. Carried in the record because a zero alignment makes
  ||| "offset is a multiple of alignment" meaningless, and because the
  ||| predicate below cannot synthesise one: with `fieldAlignment` an
  ||| opaque projection, Idris2 fails to unify it with the `S ?n` shape
  ||| `SIsNonZero` demands.
  |||
  ||| Erased. `NonZero n` is a one-constructor type, so nothing is retained
  ||| at runtime and auto-search resolves it for concrete alignments.
  {0 alignNonZero : NonZero fieldAlignment}

||| Proof that a field is correctly aligned within a struct.
|||
||| Previously hard-coded `SIsNonZero` as the non-zero witness. That made
||| the whole module fail to typecheck with
||| "Can't solve constraint between: S ?n and f .fieldAlignment", so none
||| of the properties in this file had ever been checked. It now uses the
||| witness the field carries.
public export
FieldAligned : StructField -> Type
FieldAligned f = modNatNZ (fieldOffset f) (fieldAlignment f) f.alignNonZero = 0

||| Proof that a field does not overflow past a given struct size.
public export
FieldInBounds : (structSize : Nat) -> StructField -> Type
FieldInBounds sz f = LTE (fieldOffset f + fieldSize f) sz

||| A struct layout is a list of fields with a total size.
public export
record StructLayout where
  constructor MkLayout
  layoutName : String
  layoutFields : List StructField
  layoutSize : Nat
  layoutAlignment : Nat

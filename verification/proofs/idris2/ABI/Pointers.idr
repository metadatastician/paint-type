-- SPDX-License-Identifier: AGPL-3.0-or-later
-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
--
-- ABI Proof: Non-null pointer safety
-- All proofs MUST be constructive (no believe_me, no assert_total).

module ABI.Pointers

import Data.So

%default total

||| A pointer value that has been proven non-null.
|||
||| The `So` field carries the compile-time witness that `ptr /= 0`.
|||
||| NOTE on why this field is not erased. It was originally declared
||| `{auto 0 nonNull : So (ptr /= 0)}`. That form does not typecheck in
||| Idris2 0.8.0: an erased record field has no accessible projection, so
||| `safePtrNeverNull` below failed with
|||
|||     ABI.Pointers.SafePtr.(.nonNull) is not accessible in this context
|||
||| Dropping `auto` while keeping `0` does not help either, and neither does
||| deconstructing the record on the left-hand side -- in the brace form the
||| witness is an implicit constructor argument, so the constructor is
||| `So (ptr /= 0) -> SafePtr` and a two-pattern match mismatches. Measured,
||| not inferred: `0 nonNull : ...`, `{0 nonNull : ...}` and
||| `{auto 0 nonNull : ...}` all fail; a plain field is the only form that
||| yields a projection.
|||
||| The cost is that the witness is retained at runtime. `So b` is an empty
||| data type for `False` and a one-constructor type for `True`, so the
||| representation is a tag, not a burden -- but it is no longer zero-cost,
||| and anyone optimising this for FFI should re-check the emitted layout.
public export
record SafePtr where
  constructor MkSafePtr
  ptr : Bits64
  nonNull : So (ptr /= 0)

||| Proof that SafePtr can never hold a null (zero) value.
||| This is enforced by the `So` field in the record.
export
safePtrNeverNull : (sp : SafePtr) -> So (sp.ptr /= 0)
safePtrNeverNull sp = sp.nonNull

||| Wrap a raw pointer with a runtime null check.
||| Returns Nothing if the pointer is null.
export
checkPtr : (raw : Bits64) -> Maybe SafePtr
checkPtr 0 = Nothing
checkPtr raw = case choose (raw /= 0) of
  Left prf => Just (MkSafePtr raw prf)
  Right _ => Nothing

||| Proof that checkPtr 0 always returns Nothing.
export
checkPtrZeroIsNothing : checkPtr 0 = Nothing
checkPtrZeroIsNothing = Refl

||| An opaque handle backed by a non-null pointer.
||| Use this for FFI resource handles (file descriptors, sockets, etc.).
public export
record Handle (tag : String) where
  constructor MkHandle
  safePtr : SafePtr

||| `So b` has at most one inhabitant: nothing for `False`, `Oh` for `True`.
||| This is the proof-irrelevance step `handlePtrEq` needs -- without it the
||| two non-null witnesses stay distinct metavariables and `Refl` will not
||| close the goal, even though the pointers themselves have unified.
soUnique : (b : Bool) -> (x, y : So b) -> x = y
soUnique True Oh Oh = Refl

||| Injectivity of `MkSafePtr` on the pointer alone. The witnesses are
||| rewritten away by `soUnique` rather than matched on.
safePtrEq : (a, b : SafePtr) -> a.ptr = b.ptr -> a = b
safePtrEq (MkSafePtr p pa) (MkSafePtr p pb) Refl =
  rewrite soUnique (p /= 0) pa pb in Refl

||| Proof that two handles with equal pointers are equal.
|||
||| The previous form,
|||
|||     handlePtrEq (MkHandle (MkSafePtr p)) (MkHandle (MkSafePtr p)) Refl = Refl
|||
||| failed with "Pattern variable p unifies with: (?h1.safePtr).ptr". Binding
||| a pattern variable to a dot-projection of another argument is not
||| permitted, so the handles are taken apart one level and the pointer
||| equality is discharged by `safePtrEq` instead.
export
handlePtrEq : (h1, h2 : Handle tag) -> h1.safePtr.ptr = h2.safePtr.ptr -> h1 = h2
handlePtrEq (MkHandle a) (MkHandle b) prf = cong MkHandle (safePtrEq a b prf)

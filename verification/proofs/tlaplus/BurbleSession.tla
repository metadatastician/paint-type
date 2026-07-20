-------------------------- MODULE BurbleSession --------------------------
(* SPDX-License-Identifier: AGPL-3.0-or-later                               *)
(* Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)                  *)
(*                                                                          *)
(* CONC-3 : Burble collaboration session LIVENESS.                          *)
(*                                                                          *)
(*   "Every committed tile mutation is eventually visible to every peer."   *)
(*                                                                          *)
(* ------------------------------------------------------------------------ *)
(* MODEL                                                                     *)
(* ------------------------------------------------------------------------ *)
(* A session is a set of `Peers` collaborating on a shared canvas over the   *)
(* Burble WebRTC data channel (`src/paint_collab/src/transport.rs`). A peer  *)
(* COMMITS a tile mutation locally and broadcasts it; the data channel       *)
(* DELIVERS it to the other peers, who merge it into their replica.          *)
(*                                                                           *)
(* Because the tile merge is a CRDT join-semilattice (commutative,           *)
(* associative, idempotent — proved in `TileCRDT.agda`, CONC-1/CONC-2), the  *)
(* ORDER and MULTIPLICITY of delivery are irrelevant to the final state;     *)
(* convergence therefore reduces to a pure LIVENESS question about the       *)
(* transport: does every broadcast mutation eventually reach every peer?     *)
(* That is exactly what this spec model-checks. We abstract the pixel        *)
(* payload away (the CRDT proof handles merge correctness) and track only    *)
(* the SET of mutations each peer has applied.                               *)
(*                                                                           *)
(* State:                                                                     *)
(*   committed   : mutations that have been committed by some peer           *)
(*   delivered   : per-peer set of mutations that peer has applied/merged    *)
(*   inflight    : <<peer, mutation>> pairs still in transit on the channel  *)
(*                                                                           *)
(* Liveness holds under WEAK FAIRNESS of delivery: the data channel does not *)
(* drop a message forever (Burble's DataChannel is reliable/ordered, so an   *)
(* in-flight op is eventually delivered). No fairness is assumed on Commit —  *)
(* the property is conditional on a mutation having been committed.          *)
(*                                                                           *)
(* Check with TLC (deadlock checking off: the fully-converged quiescent      *)
(* state has no enabled action and is a legitimate terminal state):          *)
(*   java -cp tla2tools.jar tlc2.TLC -deadlock \                             *)
(*        -config BurbleSession.cfg BurbleSession.tla                        *)
(***************************************************************************)

EXTENDS Naturals, FiniteSets

CONSTANTS
    Peers,      \* the set of peers in the session
    Mutations   \* the (bounded) set of tile mutations, for model checking

VARIABLES
    committed,  \* SUBSET Mutations : committed by some peer
    delivered,  \* [Peers -> SUBSET Mutations] : applied per peer
    inflight    \* SUBSET (Peers \X Mutations) : in transit

vars == <<committed, delivered, inflight>>

TypeOK ==
    /\ committed \subseteq Mutations
    /\ delivered \in [Peers -> SUBSET Mutations]
    /\ inflight  \subseteq (Peers \X Mutations)

Init ==
    /\ committed = {}
    /\ delivered = [p \in Peers |-> {}]
    /\ inflight  = {}

(* A peer commits a fresh mutation: it is now committed, the originator has   *)
(* it immediately, and a copy is enqueued to every other peer.               *)
Commit(p, m) ==
    /\ m \notin committed
    /\ committed' = committed \cup {m}
    /\ delivered' = [delivered EXCEPT ![p] = @ \cup {m}]
    /\ inflight'  = inflight \cup { <<q, m>> : q \in (Peers \ {p}) }

(* The data channel delivers an in-flight mutation to its destination peer,   *)
(* which merges it (idempotently) into its replica.                          *)
Deliver(p, m) ==
    /\ <<p, m>> \in inflight
    /\ delivered' = [delivered EXCEPT ![p] = @ \cup {m}]
    /\ inflight'  = inflight \ { <<p, m>> }
    /\ UNCHANGED committed

Next ==
    \/ \E p \in Peers, m \in Mutations : Commit(p, m)
    \/ \E p \in Peers, m \in Mutations : Deliver(p, m)

(* Weak fairness on every delivery: no in-flight op is ignored forever.       *)
Fairness == \A p \in Peers, m \in Mutations : WF_vars(Deliver(p, m))

Spec == Init /\ [][Next]_vars /\ Fairness

-----------------------------------------------------------------------------
(* SAFETY                                                                     *)

(* No peer ever applies a mutation that was never committed.                  *)
Safe == \A p \in Peers : delivered[p] \subseteq committed

(* INV-2 re-verification under concurrency: a peer's applied-set only GROWS — *)
(* delivery/merge never silently discards an already-applied mutation.        *)
Monotone == [][ \A p \in Peers : delivered[p] \subseteq delivered'[p] ]_vars

-----------------------------------------------------------------------------
(* LIVENESS — CONC-3                                                          *)

AllSeen(m) == \A p \in Peers : m \in delivered[p]

(* Every committed mutation eventually becomes visible to every peer.         *)
Liveness == \A m \in Mutations : (m \in committed) ~> AllSeen(m)

=============================================================================

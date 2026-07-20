// SPDX-License-Identifier: AGPL-3.0-or-later
//
// paint_collab — the v0.5.0 collaboration core for paint.type.
//
// This crate provides the conflict-free, formally-grounded substrate for
// multi-peer collaborative painting:
//
//   * [`crdt`]       — conflict-free tile merging: a state-based join-semilattice
//                      CRDT (last-writer-wins per pixel under a total order).
//                      Commutative + associative + idempotent ⇒ Strong Eventual
//                      Consistency. Mechanised in `verification/proofs/agda/
//                      TileCRDT.agda` (CONC-1, CONC-2).
//   * [`permission`] — per-peer capability model (read / paint / layer-mutate /
//                      invite / kick); denied actions return capability errors,
//                      never silent success.
//   * [`session`]    — the per-peer session: gated local edits, CRDT-merged
//                      remote ops, transport-agnostic. In-process `sim`
//                      transport included; convergence proven order- and
//                      duplicate-insensitive.
//   * [`transport`]  — live WebRTC-over-Burble binding (SCAFFOLD — the live
//                      data channel needs a running Burble bridge).
//   * [`groove`]     — `.well-known/groove/` service discovery so peers find
//                      each other without a central broker.
//   * [`llm`]        — optional, off-by-default AI assistant channel; assistant
//                      actions pass the same permission gate as human peers.
//
// Transport-independent and FFI-free by design (see `Cargo.toml`): everything
// here is `cargo test`-able without the native `paint_core`/libpt stack.
//
// What is NOT in this crate (honestly scoped out of this pass): a live,
// latency-measured two-peer WebRTC session (needs a running Burble + browser
// WebRTC stack, unavailable in CI), the live Groove discovery runtime, and the
// live boj-server MCP gateway. Those are tracked as integration work; the
// traits here are the seams they plug into.

#![forbid(unsafe_code)]

pub mod crdt;
pub mod groove;
pub mod llm;
pub mod permission;
pub mod session;
pub mod transport;

pub use crdt::{CrdtTile, Dot, PeerId, Rgba, VPixel, TILE_PIXEL_COUNT, TILE_SIZE};
pub use permission::{Capability, CapabilityError, CapabilitySet, PermissionTable};
pub use session::{Op, Session, SessionError, Transport};

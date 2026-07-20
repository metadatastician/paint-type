// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Emit paint-type's canonical Groove manifest to stdout. This is the
// generator for the committed `.well-known/groove/manifest.json`; run
//   cargo run --example emit_manifest > ../../.well-known/groove/manifest.json
// to regenerate the contract after changing `groove::paint_type_manifest()`.

fn main() {
    print!("{}", paint_collab::groove::paint_type_manifest().to_json());
}

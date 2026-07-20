// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Groove service discovery for collaborative paint.type sessions.
//
// Groove is the estate's `.well-known/`-based capability-discovery convention
// (canonical schema: `gossamer/schema/groove-manifest.schema.json`, served at
// `GET /.well-known/groove`). A paint-type instance that is hosting a
// collaborative canvas advertises itself with a Groove manifest so peers can
// find it WITHOUT a central broker — the milestone's "peers find each other
// without a central broker" requirement.
//
// This module:
//   * builds paint-type's canonical manifest (`paint_type_manifest`),
//   * serialises it to schema-conformant JSON (`GrooveManifest::to_json`),
//   * structurally validates a manifest (`GrooveManifest::validate`), and
//   * defines a `Discovery` trait + an in-process `SimDiscovery` so the
//     announce→discover→connect handshake is testable without a network.
//
// The committed contract file `.well-known/groove/manifest.json` is the
// on-disk realisation of `paint_type_manifest()`; `tests` cross-checks the two
// so code and contract cannot drift. JSON-Schema conformance of the committed
// file is checked at the aspect-test layer (`tests/aspect_tests.sh`) with `jq`.

use std::collections::BTreeMap;

/// Groove protocol version this implementation speaks.
pub const GROOVE_VERSION: &str = "1";

/// A capability advertised in the manifest. Mirrors the `Capability` object in
/// `groove-manifest.schema.json`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrooveCapability {
    /// Capability `type` — must be a schema enum wire name. paint-type's
    /// collaborative-canvas capabilities are `custom` (not a built-in Groove
    /// type).
    pub cap_type: String,
    pub description: String,
    /// Wire protocol — `webrtc` for the edit channel (carried over Burble).
    pub protocol: String,
    pub endpoint: String,
    pub requires_auth: bool,
    pub panel_compatible: bool,
}

/// A Groove manifest. Mirrors the top-level object in the v1 schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrooveManifest {
    pub groove_version: String,
    pub service_id: String,
    pub service_version: String,
    /// Capability name → capability.
    pub capabilities: BTreeMap<String, GrooveCapability>,
    /// Capabilities this service would like from grooved siblings.
    pub consumes: Vec<String>,
    pub endpoints: BTreeMap<String, String>,
    pub health: String,
    pub applicability: Vec<String>,
}

/// Why a manifest failed structural validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ManifestError {
    WrongGrooveVersion(String),
    BadServiceId(String),
    NoCapabilities,
    BadServiceVersion(String),
}

impl core::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ManifestError::WrongGrooveVersion(v) => {
                write!(f, "groove_version must be \"1\", got {v:?}")
            }
            ManifestError::BadServiceId(s) => {
                write!(f, "service_id {s:?} violates ^[a-z][a-z0-9_-]*$")
            }
            ManifestError::NoCapabilities => write!(f, "manifest advertises no capabilities"),
            ManifestError::BadServiceVersion(v) => {
                write!(f, "service_version {v:?} is not semver-ish (X.Y.Z)")
            }
        }
    }
}

impl std::error::Error for ManifestError {}

impl GrooveManifest {
    /// Structurally validate against the v1 schema's hard constraints.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.groove_version != GROOVE_VERSION {
            return Err(ManifestError::WrongGrooveVersion(self.groove_version.clone()));
        }
        if !is_valid_service_id(&self.service_id) {
            return Err(ManifestError::BadServiceId(self.service_id.clone()));
        }
        if !is_semver_ish(&self.service_version) {
            return Err(ManifestError::BadServiceVersion(self.service_version.clone()));
        }
        if self.capabilities.is_empty() {
            return Err(ManifestError::NoCapabilities);
        }
        Ok(())
    }

    /// Serialise to canonical, schema-conformant JSON. Hand-rolled (no serde
    /// dependency) and deterministic: `BTreeMap` ordering makes output stable.
    pub fn to_json(&self) -> String {
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"groove_version\": {},\n", jstr(&self.groove_version)));
        s.push_str(&format!("  \"service_id\": {},\n", jstr(&self.service_id)));
        s.push_str(&format!(
            "  \"service_version\": {},\n",
            jstr(&self.service_version)
        ));
        s.push_str("  \"capabilities\": {\n");
        let caps: Vec<_> = self.capabilities.iter().collect();
        for (i, (name, cap)) in caps.iter().enumerate() {
            s.push_str(&format!("    {}: {{\n", jstr(name)));
            s.push_str(&format!("      \"type\": {},\n", jstr(&cap.cap_type)));
            s.push_str(&format!(
                "      \"description\": {},\n",
                jstr(&cap.description)
            ));
            s.push_str(&format!("      \"protocol\": {},\n", jstr(&cap.protocol)));
            s.push_str(&format!("      \"endpoint\": {},\n", jstr(&cap.endpoint)));
            s.push_str(&format!(
                "      \"requires_auth\": {},\n",
                cap.requires_auth
            ));
            s.push_str(&format!(
                "      \"panel_compatible\": {}\n",
                cap.panel_compatible
            ));
            s.push_str(if i + 1 == caps.len() { "    }\n" } else { "    },\n" });
        }
        s.push_str("  },\n");
        s.push_str(&format!("  \"consumes\": {},\n", jarr(&self.consumes)));
        s.push_str("  \"endpoints\": {\n");
        let eps: Vec<_> = self.endpoints.iter().collect();
        for (i, (k, v)) in eps.iter().enumerate() {
            let comma = if i + 1 == eps.len() { "" } else { "," };
            s.push_str(&format!("    {}: {}{}\n", jstr(k), jstr(v), comma));
        }
        s.push_str("  },\n");
        s.push_str(&format!("  \"health\": {},\n", jstr(&self.health)));
        s.push_str(&format!(
            "  \"applicability\": {}\n",
            jarr(&self.applicability)
        ));
        s.push_str("}\n");
        s
    }
}

fn jstr(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

fn jarr(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|s| jstr(s)).collect();
    format!("[{}]", parts.join(", "))
}

/// `^[a-z][a-z0-9_-]*$`
fn is_valid_service_id(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// Loosely matches `^[0-9]+\.[0-9]+\.[0-9]+`.
fn is_semver_ish(s: &str) -> bool {
    let core = s.split(['-', '+']).next().unwrap_or(s);
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() >= 3
        && parts[..3]
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// paint-type's canonical Groove manifest for a collaborative session host.
pub fn paint_type_manifest() -> GrooveManifest {
    let mut capabilities = BTreeMap::new();
    capabilities.insert(
        "collab-canvas".to_string(),
        GrooveCapability {
            cap_type: "custom".to_string(),
            description: "Conflict-free collaborative tile canvas (CRDT merge) \
                          over a Burble WebRTC data channel"
                .to_string(),
            protocol: "webrtc".to_string(),
            endpoint: "/collab/canvas".to_string(),
            requires_auth: true,
            panel_compatible: true,
        },
    );
    capabilities.insert(
        "edit-stream".to_string(),
        GrooveCapability {
            cap_type: "custom".to_string(),
            description: "Realtime tile-edit op stream; CRDT-merged and \
                          permission-gated per peer"
                .to_string(),
            protocol: "webrtc".to_string(),
            endpoint: "/collab/edits".to_string(),
            requires_auth: true,
            panel_compatible: false,
        },
    );

    let mut endpoints = BTreeMap::new();
    endpoints.insert("health".to_string(), "/health".to_string());
    endpoints.insert("groove".to_string(), "/.well-known/groove".to_string());
    endpoints.insert(
        "schema".to_string(),
        "/schema/groove-manifest.schema.json".to_string(),
    );

    GrooveManifest {
        groove_version: GROOVE_VERSION.to_string(),
        service_id: "paint-type".to_string(),
        service_version: "0.5.0".to_string(),
        capabilities,
        // We consume Burble's voice/text/presence (the shared session) and
        // integrity (op-stream attestation) when grooved siblings offer them.
        consumes: vec![
            "voice".to_string(),
            "text".to_string(),
            "presence".to_string(),
            "integrity".to_string(),
        ],
        endpoints,
        health: "/health".to_string(),
        applicability: vec!["individual".to_string(), "team".to_string()],
    }
}

/// A discovered peer offering a collaborative session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscoveredPeer {
    pub service_id: String,
    /// Room code / session locator to hand to the transport.
    pub room: String,
    pub manifest: GrooveManifest,
}

/// Service discovery: find paint-type peers announcing a session. The live
/// implementation reads `.well-known/groove/` over mDNS/HTTP (the `MdnsBackend`
/// / `WellKnownBackend` in `src/backends/net/`); `SimDiscovery` serves a fixed
/// in-process registry so the handshake is testable.
pub trait Discovery {
    /// Announce that `room` is hosting a collaborative session with `manifest`.
    fn announce(&mut self, room: &str, manifest: GrooveManifest);
    /// Find peers currently announcing collaborative sessions.
    fn discover(&self) -> Vec<DiscoveredPeer>;
}

/// In-process discovery registry.
#[derive(Default)]
pub struct SimDiscovery {
    peers: Vec<DiscoveredPeer>,
}

impl SimDiscovery {
    pub fn new() -> SimDiscovery {
        SimDiscovery::default()
    }
}

impl Discovery for SimDiscovery {
    fn announce(&mut self, room: &str, manifest: GrooveManifest) {
        self.peers.push(DiscoveredPeer {
            service_id: manifest.service_id.clone(),
            room: room.to_string(),
            manifest,
        });
    }

    fn discover(&self) -> Vec<DiscoveredPeer> {
        self.peers.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_validates() {
        assert_eq!(paint_type_manifest().validate(), Ok(()));
    }

    #[test]
    fn rejects_bad_service_id() {
        let mut m = paint_type_manifest();
        m.service_id = "Paint_Type".to_string(); // capital — illegal
        assert!(matches!(m.validate(), Err(ManifestError::BadServiceId(_))));
    }

    #[test]
    fn rejects_wrong_groove_version() {
        let mut m = paint_type_manifest();
        m.groove_version = "2".to_string();
        assert!(matches!(
            m.validate(),
            Err(ManifestError::WrongGrooveVersion(_))
        ));
    }

    #[test]
    fn json_contains_required_top_level_keys() {
        let j = paint_type_manifest().to_json();
        for key in [
            "\"groove_version\"",
            "\"service_id\"",
            "\"capabilities\"",
            "\"collab-canvas\"",
            "\"webrtc\"",
        ] {
            assert!(j.contains(key), "missing {key} in:\n{j}");
        }
    }

    #[test]
    fn json_matches_committed_well_known_file() {
        // Code ↔ contract: the on-disk `.well-known/groove/manifest.json` must
        // be byte-identical to what the canonical manifest serialises to.
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../.well-known/groove/manifest.json"
        );
        let on_disk = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("cannot read committed manifest {path}: {e}"));
        assert_eq!(
            on_disk,
            paint_type_manifest().to_json(),
            "committed .well-known/groove/manifest.json has drifted from \
             groove::paint_type_manifest(); regenerate it"
        );
    }

    #[test]
    fn discovery_round_trip() {
        let mut d = SimDiscovery::new();
        d.announce("room-xyz", paint_type_manifest());
        let found = d.discover();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].service_id, "paint-type");
        assert_eq!(found[0].room, "room-xyz");
        assert!(found[0].manifest.validate().is_ok());
    }
}

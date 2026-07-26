-- SPDX-License-Identifier: CC-BY-SA-4.0
-- TLA+ specification for SEC-2: Plugin API surface confinement

---- MODULE PluginApiSurface ----
EXTENDS Naturals, Sequences, TLC

CONSTANTS
    HostApiFunctions,  \* Complete set of host API functions
    SafeApiFunctions,  \* API functions available without capabilities
    ApiToCap           \* Mapping from API functions to required capabilities

VARIABLES
    api_calls,        \* Current plugin API call history (for audit trail)
    api_surface,      \* API surface registry (which functions are exposed to which plugin)
    plugin_caps,      \* Currently granted capabilities for each plugin
    plugin_active     \* Plugin execution state (active/inactive)

(* Type definitions *)
PluginId == STRING

(* Check if a plugin has the required capabilities for a function *)
has_required_caps(p, func) == 
    ApiToCap[func] \subseteq plugin_caps[p]

(* Check if a plugin can call a specific API function *)
can_call_api(p, func) == 
    /\ func \in api_surface[p]
    /\ (ApiToCap[func] = {} \/ has_required_caps(p, func))

(* Get all API functions a plugin can access *)
accessible_api(p) == {func \in HostApiFunctions : can_call_api(p, func)}

(* Manifest-based API surface for a plugin *)
ManifestApiSurface(p) == {func \in HostApiFunctions : func \in {"canvas_read_pixels", "canvas_write_pixels", "log_debug"}}

(* Initial state *)
Init == 
    /\ api_calls = [p \in PluginId |-> <<>>]
    /\ api_surface = [p \in PluginId |-> {}]
    /\ plugin_caps = [p \in PluginId |-> {}]
    /\ plugin_active = [p \in PluginId |-> FALSE]

(* Load a plugin with its API surface based on manifest *)
LoadPluginWithApi(pid, caps, api_funcs) == 
    /\ pid \notin DOMAIN plugin_active
    /\ caps \subseteq {"CanvasRead", "CanvasWrite", "LayerAccess", "FileIO", "Network"}
    /\ api_funcs \subseteq HostApiFunctions
    /\ plugin_active' = [plugin_active EXCEPT ![pid] = TRUE]
    /\ plugin_caps' = [plugin_caps EXCEPT ![pid] = caps]
    /\ api_surface' = [api_surface EXCEPT ![pid] = api_funcs]
    /\ UNCHANGED api_calls

(* Grant capability to a plugin *)
GrantCapability(pid, cap) == 
    /\ pid \in DOMAIN plugin_active
    /\ plugin_active[pid]
    /\ cap \in {"CanvasRead", "CanvasWrite", "LayerAccess", "FileIO", "Network"}
    /\ plugin_caps' = [plugin_caps EXCEPT ![pid] = plugin_caps[pid] \cup {cap}]
    /\ UNCHANGED <<api_calls, api_surface, plugin_active>>

(* Invoke a plugin API call *)
InvokeApi(pid, func, args) == 
    /\ pid \in DOMAIN plugin_active
    /\ plugin_active[pid]
    /\ func \in HostApiFunctions
    /\ can_call_api(pid, func)
    /\ api_calls' = [api_calls EXCEPT ![pid] = Append(api_calls[pid], <<pid, func, args>>)]
    /\ UNCHANGED <<plugin_caps, plugin_active, api_surface>>

(* Reject an API call due to missing capability or access *)
ApiCallRejected(pid, func, reason) == 
    /\ pid \in DOMAIN plugin_active
    /\ plugin_active[pid]
    /\ api_calls' = [api_calls EXCEPT ![pid] = Append(api_calls[pid], <<pid, func, "REJECTED: " \o reason>>)]
    /\ UNCHANGED <<plugin_caps, plugin_active, api_surface>>

(* SEC-2: API Surface Confinement *)
(* Plugins can only call API functions that are both in their API surface and have capabilities *)
ApiSurfaceConfinement == 
    \E p \in DOMAIN plugin_active, g \in HostApiFunctions, args \in STRING :
        /\ plugin_active[p]
        /\ ~can_call_api(p, g)
        => ApiCallRejected(p, g, "Capability or API access denied")

(* SEC-2a: API surface is minimal (no functions beyond declared set) *)
ApiSurfaceMinimal == 
    \E p \in DOMAIN plugin_active, h \in STRING :
        /\ plugin_active[p]
        /\ h \notin HostApiFunctions
        => ApiCallRejected(p, h, "Unknown API function")

(* SEC-2b: API surface matches manifest declaration *)
ApiSurfaceMatchesManifest == 
    \E p \in DOMAIN plugin_active :
        /\ plugin_active[p]
        => api_surface[p] = ManifestApiSurface(p)

(* SEC-2c: Capability requirements are enforced *)
CapabilityEnforcement == 
    \E p \in DOMAIN plugin_active, k \in HostApiFunctions :
        /\ plugin_active[p]
        /\ k \in api_surface[p]
        => has_required_caps(p, k)

(* Next state relation *)
Next == 
    \E p1 \in PluginId, caps \in SUBSET {"CanvasRead", "CanvasWrite", "LayerAccess", "FileIO", "Network"}, api_funcs \in SUBSET HostApiFunctions : LoadPluginWithApi(p1, caps, api_funcs)
    \/ \E p2 \in DOMAIN plugin_active, cap \in {"CanvasRead", "CanvasWrite", "LayerAccess", "FileIO", "Network"} : GrantCapability(p2, cap)
    \/ \E p3 \in DOMAIN plugin_active, q \in HostApiFunctions, args \in STRING : InvokeApi(p3, q, args)
    \/ \E p4 \in DOMAIN plugin_active, r \in STRING, reason \in STRING : ApiCallRejected(p4, r, reason)

(* Safety properties *)
Spec == Init /\ [][Next]_<<api_calls, api_surface, plugin_caps, plugin_active>>

====

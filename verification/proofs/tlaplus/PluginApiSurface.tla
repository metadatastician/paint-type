-- SPDX-License-Identifier: CC-BY-SA-4.0
-- TLA+ specification for SEC-2: Plugin API surface confinement
-- 
-- Proof obligation: The API surface exposed to plugins is minimal and
-- confined. Plugins can only access capabilities explicitly granted
-- in their manifest. No implicit or backdoor access to host functions.

---- MODULE PluginApiSurface ----
EXTENDS Naturals, Sequences, TLC, PluginSandbox

---- CONSTANTS ----
\* Complete set of host API functions
HostApiFunctions: {
    "canvas_read_pixels",
    "canvas_write_pixels", 
    "layer_create",
    "layer_delete",
    "layer_reorder",
    "file_read",
    "file_write",
    "network_fetch",
    "log_debug",
    "log_info",
    "log_warn",
    "log_error"
}

\* API functions available without capabilities (safe subset)
SafeApiFunctions: {"log_debug", "log_info", "log_warn", "log_error"}

\* Mapping from API functions to required capabilities
ApiToCap: [
    "canvas_read_pixels" |-> {"CanvasRead"},
    "canvas_write_pixels" |-> {"CanvasWrite"},
    "layer_create" |-> {"LayerAccess"},
    "layer_delete" |-> {"LayerAccess"},
    "layer_reorder" |-> {"LayerAccess"},
    "file_read" |-> {"FileIO"},
    "file_write" |-> {"FileIO"},
    "network_fetch" |-> {"Network"},
    "log_debug" |-> {},
    "log_info" |-> {},
    "log_warn" |-> {},
    "log_error" |-> {}
]

---- VARIABLES ----
\* Current plugin API call history (for audit trail)
api_calls: [PluginId -> SEQUENCE <<PluginId, STRING, STRING>>]

\* API surface registry (which functions are exposed to which plugin)
api_surface: [PluginId -> SUBSET HostApiFunctions]

---- DEFINITIONS ----
\* Check if a plugin can call a specific API function
can_call_api(p, func) == 
    /\ func \in api_surface[p]
    /\ LET required_caps = ApiToCap[func] IN
        /\ required_caps = {}  \* Safe functions need no capabilities
            \/ required_caps \subseteq plugin_caps[p]  \* Other functions need all required caps

\* Get all API functions a plugin can access
accessible_api(p) == {func \in HostApiFunctions : can_call_api(p, func)}

---- SAFETY PROPERTIES ----

\* SEC-2: API Surface Confinement
\* Plugins can only call API functions that are both:
\*   1. In their granted API surface
\*   2. For which they have all required capabilities
THEOREM ApiSurfaceConfinement == 
    []<>[](
        \E p \in DOMAIN plugin_active, func \in HostApiFunctions, args \in STRING :
            /\ plugin_active[p]
            /\ ~can_call_api(p, func)
            /\ ApiCall(p, func, args)  \* Plugin attempts to call API function
            => ApiCallRejected(p, func, "Capability or API access denied")
    )

\* SEC-2a: API surface is minimal (no functions beyond declared set)
THEOREM ApiSurfaceMinimal == 
    []<>[](
        \E p \in DOMAIN plugin_active, func \in STRING :
            /\ plugin_active[p]
            /\ func \notin HostApiFunctions
            /\ ApiCall(p, func, @)  \* Call to undeclared function
            => ApiCallRejected(p, func, "Unknown API function")
    )

\* SEC-2b: API surface matches manifest declaration
THEOREM ApiSurfaceMatchesManifest == 
    []<>[](
        \E p \in DOMAIN plugin_active :
            /\ plugin_active[p]
            => api_surface[p] = ManifestApiSurface(p)
    )

\* SEC-2c: Capability requirements are enforced
THEOREM CapabilityEnforcement == 
    []<>[](
        \E p \in DOMAIN plugin_active, func \in HostApiFunctions :
            /\ plugin_active[p]
            /\ func \in api_surface[p]
            /\ ApiCall(p, func, @)
            /\ LET required = ApiToCap[func] IN
            => required \subseteq plugin_caps[p]
    )

---- INITIAL STATE ----
Init == 
    /\ \* Initialize with empty API surfaces
    api_calls = [p \in PluginId |-> <<>>]
    /\ api_surface = [p \in PluginId |-> {}]

---- NEXT STATE ----
\* Placeholder - actual transitions would include:
\* - LoadPlugin: Sets api_surface[p] based on manifest
\* - GrantCapability: Adds capabilities to plugin_caps[p]
\* - InvokePlugin: Checks can_call_api before allowing call
Next == 
    UNCHANGED (api_calls, api_surface)

====

\* SEC-2: Plugin API surface is confined and capability-gated
\* Status: SPECIFIED (TLA+ skeleton created)
\* Next: Implement transition definitions and model-check with TLC
\* Dependencies: v0.4.0 Plugin System (issue #14) - COMPLETE
\* Related: PluginSandbox.tla (SEC-1) provides memory isolation foundation

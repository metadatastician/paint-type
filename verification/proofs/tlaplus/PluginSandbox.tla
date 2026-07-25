-- SPDX-License-Identifier: CC-BY-SA-4.0
-- TLA+ specification for SEC-1: Plugin WASM sandbox isolation
-- 
-- Proof obligation: Plugins running in the WASM sandbox cannot escape to
-- Ephapax memory or host system. The sandbox provides:
-- - Memory isolation (WASM linear memory separate from host)
-- - No direct pointer access to host memory
-- - Capability-based gating for all host calls
-- - Type-checked boundary (typed-wasm)

---- MODULE PluginSandbox ----
EXTENDS Naturals, Sequences, TLC

---- CONSTANTS ----
\* Maximum WASM memory size in bytes
MAX_WASM_MEMORY: 16777216  \* 16 MiB

\* Plugin capability set
Capabilities: {"CanvasRead", "CanvasWrite", "LayerAccess", "FileIO", "Network"}

\* Host memory addresses (opaque to WASM)
HostMemory: 0..1073741824  \* Simulated host address space

\* WASM memory addresses
WasmMemory: 0..16777215

---- VARIABLES ----
\* Current plugin memory state (WASM linear memory)
wasm_mem: [WasmMemory -> Byte]

\* Host memory state
host_mem: [HostMemory -> Byte]

\* Currently granted capabilities for each plugin
plugin_caps: [PluginId -> SUBSET Capabilities]

\* Plugin sandbox execution state (active/inactive)
plugin_active: [PluginId -> BOOLEAN]

---- DEFINITIONS ----
Byte == 0..255
PluginId == STRING

\* Check if a plugin has a specific capability
has_cap(p, cap) == cap \in plugin_caps[p]

\* Memory access is confined to WASM memory bounds
MemoryAccessConfined(addr, size) == 
    /\ addr \in WasmMemory
    /\ addr + size - 1 \in WasmMemory
    /\ size > 0

---- SAFETY PROPERTIES ----

\* SEC-1: WASM Memory Isolation
\* No WASM plugin can read or write host memory directly
THEOREM WasmMemoryIsolation == 
    <<[] (wasm_mem, host_mem, plugin_caps, plugin_active) >>_vars
    \* For all plugins and all operations, host_mem remains unchanged by WASM execution

\* SEC-1a: Memory access is always bounded
THEOREM WasmMemoryAccessBounded == 
    []<>[](
        \E p \in DOMAIN plugin_active, addr \in Nat, size \in Nat :
            /\ plugin_active[p]
            /\ ~MemoryAccessConfined(addr, size)
            /\ wasm_mem' = [wasm_mem EXCEPT ![p][addr..addr+size-1] = @]
            => UNCHANGED host_mem
    )

\* SEC-1b: No direct host memory access from WASM
THEOREM NoDirectHostAccess == 
    []<>[](
        \E p \in DOMAIN plugin_active, addr \in HostMemory :
            /\ plugin_active[p]
            /\ wasm_mem' = [wasm_mem EXCEPT ![p][@] = @]  \* Any WASM memory write
            => UNCHANGED host_mem
    )

---- CAPABILITY GATING ----

\* SEC-1c: All host API calls are gated by capabilities
THEOREM CapabilityGated == 
    []<>[](
        \E p \in DOMAIN plugin_active, cap \in Capabilities :
            /\ plugin_active[p]
            /\ ~has_cap(p, cap)
            /\ HostApiCall(p, cap)  \* Attempt to call host API requiring 'cap'
            => UNCHANGED (host_mem, wasm_mem)  \* Call is rejected
    )

---- INITIAL STATE ----
Init == 
    /\ wasm_mem = [addr \in WasmMemory |-> 0]
    /\ host_mem = [addr \in HostMemory |-> 0]
    /\ plugin_caps = [p \in PluginId |-> {}]
    /\ plugin_active = [p \in PluginId |-> FALSE]

---- NEXT STATE ----
Next == 
    \* Placeholder for actual transition definitions
    \* In full spec: define LoadPlugin, InvokePlugin, UnloadPlugin actions
    UNCHANGED (wasm_mem, host_mem, plugin_caps, plugin_active)

====

\* SEC-1: Plugin WASM sandbox provides complete isolation from host memory
\* Status: SPECIFIED (TLA+ skeleton created)
\* Next: Implement transition definitions and model-check with TLC
\* Dependencies: v0.4.0 Plugin System (issue #14) - COMPLETE

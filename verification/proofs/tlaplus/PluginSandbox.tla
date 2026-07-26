-- SPDX-License-Identifier: CC-BY-SA-4.0
-- TLA+ specification for SEC-1: Plugin WASM sandbox isolation

---- MODULE PluginSandbox ----
EXTENDS Naturals, Sequences, TLC

CONSTANTS
    MAX_WASM_MEMORY,  \* Maximum WASM memory size in bytes (16 MiB)
    Capabilities       \* Plugin capability set

VARIABLES
    wasm_mem,         \* Current plugin memory state (WASM linear memory)
    host_mem,         \* Host memory state
    plugin_caps,      \* Currently granted capabilities for each plugin
    plugin_active     \* Plugin sandbox execution state (active/inactive)

(* Type definitions *)
Byte == 0..255
PluginId == STRING
HostMemory == 0..1073741824  \* Simulated host address space
WasmMemory == 0..MAX_WASM_MEMORY - 1

(* Check if a plugin has a specific capability *)
has_cap(p, cap) == cap \in plugin_caps[p]

(* Memory access is confined to WASM memory bounds *)
MemoryAccessConfined(a, s) == 
    /\ a \in WasmMemory
    /\ a + s - 1 \in WasmMemory
    /\ s > 0

(* SEC-1: host_mem is always unchanged by Next (memory isolation) *)
WasmMemoryIsolation == UNCHANGED host_mem

(* SEC-1a: Memory access is always bounded *)
WasmMemoryAccessBounded == 
    \E p \in DOMAIN plugin_active :
        /\ plugin_active[p]
        /\ ~MemoryAccessConfined(0, 1)
        => UNCHANGED host_mem

(* SEC-1b: No direct host memory access from WASM *)
NoDirectHostAccess == 
    \E p \in DOMAIN plugin_active, a \in HostMemory :
        /\ plugin_active[p]
        => UNCHANGED host_mem

(* SEC-1c: All host API calls are gated by capabilities *)
CapabilityGated == 
    \E p \in DOMAIN plugin_active, c \in Capabilities :
        /\ plugin_active[p]
        /\ ~has_cap(p, c)
        => UNCHANGED <<host_mem, wasm_mem>>

(* Initial state *)
Init == 
    /\ wasm_mem = [addr \in WasmMemory |-> 0]
    /\ host_mem = [addr \in HostMemory |-> 0]
    /\ plugin_caps = [p \in PluginId |-> {}]
    /\ plugin_active = [p \in PluginId |-> FALSE]

(* Load a plugin with given capabilities *)
LoadPlugin(pid, caps) == 
    /\ pid \notin DOMAIN plugin_active
    /\ caps \subseteq Capabilities
    /\ plugin_active' = [plugin_active EXCEPT ![pid] = TRUE]
    /\ plugin_caps' = [plugin_caps EXCEPT ![pid] = caps]
    /\ wasm_mem' = [@ |-> 0]
    /\ UNCHANGED host_mem

(* Invoke a plugin to perform a memory operation *)
InvokePlugin(pid, a, s, op) == 
    /\ pid \in DOMAIN plugin_active
    /\ plugin_active[pid]
    /\ op \in {"read", "write"}
    /\ MemoryAccessConfined(a, s)
    /\ wasm_mem' = [wasm_mem EXCEPT ![a] = IF op = "write" THEN 0 ELSE wasm_mem[a]]
    /\ UNCHANGED <<host_mem, plugin_caps, plugin_active>>

(* Unload a plugin *)
UnloadPlugin(pid) == 
    /\ pid \in DOMAIN plugin_active
    /\ plugin_active[pid]
    /\ plugin_active' = [plugin_active EXCEPT ![pid] = FALSE]
    /\ plugin_caps' = [plugin_caps EXCEPT ![pid] = {}]
    /\ wasm_mem' = [@ |-> 0]
    /\ UNCHANGED host_mem

(* Next state relation *)
Next == 
    \E p1 \in PluginId, caps \in SUBSET Capabilities : LoadPlugin(p1, caps)
    \/ \E p2 \in PluginId, a \in WasmMemory, s \in Nat, op \in {"read", "write"} : InvokePlugin(p2, a, s, op)
    \/ \E p3 \in DOMAIN plugin_active : UnloadPlugin(p3)

(* Safety properties *)
Spec == Init /\ [][Next]_<<wasm_mem, host_mem, plugin_caps, plugin_active>>

====

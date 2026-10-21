// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Joshua Jewell (JoshuaJewell) <paint-type@pm.me>
//
// paint.type Hexadeca-Connector — sixteen-protocol unified API surface.

const std = @import("std");

pub const Connector = enum(u8) {
    grpc = 0,
    graphql = 1,
    rest = 2,
    flatbuffers = 3,
    bebop = 4,
    jsonrpc = 5,
    websocket = 6,
    mqtt = 7,
    trpc = 8,
    capnproto = 9,
    soap = 10,
    verisimdb_rest = 11,
    bsp = 12,
    scip = 13,
    ipfs = 14,
    arrow_flight = 15,

    /// Safe decoding from an untrusted wire byte without UB on `16..=255`.
    pub fn fromId(id: u8) ?Connector {
        if (id < CONNECTOR_COUNT) {
            return @enumFromInt(id);
        }
        return null;
    }

    pub fn name(self: Connector) [*:0]const u8 {
        return switch (self) {
            .grpc => "grpc",
            .graphql => "graphql",
            .rest => "rest",
            .flatbuffers => "flatbuffers",
            .bebop => "bebop",
            .jsonrpc => "jsonrpc",
            .websocket => "websocket",
            .mqtt => "mqtt",
            .trpc => "trpc",
            .capnproto => "capnproto",
            .soap => "soap",
            .verisimdb_rest => "verisimdb-rest",
            .bsp => "bsp",
            .scip => "scip",
            .ipfs => "ipfs",
            .arrow_flight => "arrow-flight",
        };
    }
};

pub const CONNECTOR_COUNT: usize = @typeInfo(Connector).@"enum".fields.len;
pub const MAX_BASE_PORT: u16 = std.math.maxInt(u16) - (CONNECTOR_COUNT - 1);

const grpc = @import("connectors/grpc.zig");
const graphql = @import("connectors/graphql.zig");
const rest = @import("connectors/rest.zig");
const flatbuffers = @import("connectors/flatbuffers.zig");
const bebop = @import("connectors/bebop.zig");
const jsonrpc = @import("connectors/jsonrpc.zig");
const websocket = @import("connectors/websocket.zig");
const mqtt = @import("connectors/mqtt.zig");
const trpc = @import("connectors/trpc.zig");
const capnproto = @import("connectors/capnproto.zig");
const soap = @import("connectors/soap.zig");
const verisimdb_rest = @import("connectors/verisimdb_rest.zig");
const bsp = @import("connectors/bsp.zig");
const scip = @import("connectors/scip.zig");
const ipfs = @import("connectors/ipfs.zig");
const arrow_flight = @import("connectors/arrow_flight.zig");

pub const PortError = error{
    PortZero,
    PortRangeOverflow,
};

/// Start all sixteen connectors on consecutive ports beginning at `base_port`.
/// Returns `PortZero` or `PortRangeOverflow` before starting any connector if
/// the port range is invalid.
pub fn startAll(base_port: u16) PortError!void {
    if (base_port == 0) return PortError.PortZero;
    if (base_port > MAX_BASE_PORT) return PortError.PortRangeOverflow;

    grpc.start(base_port + 0);
    graphql.start(base_port + 1);
    rest.start(base_port + 2);
    flatbuffers.start(base_port + 3);
    bebop.start(base_port + 4);
    jsonrpc.start(base_port + 5);
    websocket.start(base_port + 6);
    mqtt.start(base_port + 7);
    trpc.start(base_port + 8);
    capnproto.start(base_port + 9);
    soap.start(base_port + 10);
    verisimdb_rest.start(base_port + 11);
    bsp.start(base_port + 12);
    scip.start(base_port + 13);
    ipfs.start(base_port + 14);
    arrow_flight.start(base_port + 15);
}

test "hexadeca count and safe bounds" {
    try std.testing.expectEqual(@as(usize, 16), CONNECTOR_COUNT);
    try std.testing.expectEqual(@as(?Connector, .grpc), Connector.fromId(0));
    try std.testing.expectEqual(@as(?Connector, .arrow_flight), Connector.fromId(15));
    try std.testing.expectEqual(@as(?Connector, null), Connector.fromId(16));
    try std.testing.expectEqual(@as(?Connector, null), Connector.fromId(255));
    try std.testing.expectError(PortError.PortZero, startAll(0));
    try std.testing.expectError(PortError.PortRangeOverflow, startAll(MAX_BASE_PORT + 1));
    try startAll(9100);
}

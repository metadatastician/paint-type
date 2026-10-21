// SPDX-License-Identifier: AGPL-3.0-or-later
//
// paint.type — VeriSimDB storage backend.
//
// Governed by ADR-0003.

const std = @import("std");

//==============================================================================
// 1. Capability descriptor
//==============================================================================

pub const Modality = enum(u32) {
    graph = 0,
    vector = 1,
    tensor = 2,
    semantic = 3,
    document = 4,
    temporal = 5,
    provenance = 6,
    spatial = 7,
};

pub const Persistence = enum(u32) {
    ephemeral = 0,
    durable = 1,
    replicated = 2,
};

pub const Consistency = enum(u32) {
    strong = 0,
    eventual = 1,
};

pub const StorageCapability = struct {
    modalities: []const Modality,
    persistence: Persistence,
    consistency: Consistency,
};

pub const verisimdb_capability = StorageCapability{
    .modalities = &.{ .graph, .vector, .tensor, .semantic, .document, .temporal, .provenance, .spatial },
    .persistence = .durable,
    .consistency = .eventual,
};

//==============================================================================
// 2. Result type & Input Sanitisation (B-23)
//==============================================================================

pub const StorageResult = enum(u32) {
    ok = 0,
    not_found = 1,
    invalid = 2,
    conflict = 3,
    unavailable = 4,
    err = 5,
};

/// Accept only bounded URL-safe identifier tokens `[A-Za-z0-9._:-]`, rejecting
/// empty identifiers, dot-segments (`..`), slashes, query/fragment delimiters,
/// and control/whitespace bytes that could alter HTTP request targets.
pub fn isSafePathSegment(seg: []const u8) bool {
    if (seg.len == 0 or seg.len > 128) return false;
    if (std.mem.eql(u8, seg, ".") or std.mem.eql(u8, seg, "..")) return false;
    for (seg) |ch| {
        const ok = std.ascii.isAlphanumeric(ch) or ch == '-' or ch == '_' or ch == '.' or ch == ':';
        if (!ok) return false;
    }
    return true;
}

/// Escape an untrusted UTF-8 slice as a RFC 8259 JSON string body (without
/// outer quotes) into `out`, returning the slice written or `error.NoSpaceLeft`.
pub fn escapeJsonContent(out: []u8, input: []const u8) error{NoSpaceLeft}![]const u8 {
    var pos: usize = 0;
    for (input) |ch| {
        switch (ch) {
            '"' => {
                if (pos + 2 > out.len) return error.NoSpaceLeft;
                out[pos] = '\\';
                out[pos + 1] = '"';
                pos += 2;
            },
            '\\' => {
                if (pos + 2 > out.len) return error.NoSpaceLeft;
                out[pos] = '\\';
                out[pos + 1] = '\\';
                pos += 2;
            },
            '\n' => {
                if (pos + 2 > out.len) return error.NoSpaceLeft;
                out[pos] = '\\';
                out[pos + 1] = 'n';
                pos += 2;
            },
            '\r' => {
                if (pos + 2 > out.len) return error.NoSpaceLeft;
                out[pos] = '\\';
                out[pos + 1] = 'r';
                pos += 2;
            },
            '\t' => {
                if (pos + 2 > out.len) return error.NoSpaceLeft;
                out[pos] = '\\';
                out[pos + 1] = 't';
                pos += 2;
            },
            0x08 => {
                if (pos + 2 > out.len) return error.NoSpaceLeft;
                out[pos] = '\\';
                out[pos + 1] = 'b';
                pos += 2;
            },
            0x0C => {
                if (pos + 2 > out.len) return error.NoSpaceLeft;
                out[pos] = '\\';
                out[pos + 1] = 'f';
                pos += 2;
            },
            else => {
                if (ch < 0x20) {
                    if (pos + 6 > out.len) return error.NoSpaceLeft;
                    _ = std.fmt.bufPrint(out[pos .. pos + 6], "\\u00{x:0>2}", .{ch}) catch return error.NoSpaceLeft;
                    pos += 6;
                } else {
                    if (pos + 1 > out.len) return error.NoSpaceLeft;
                    out[pos] = ch;
                    pos += 1;
                }
            },
        }
    }
    return out[0..pos];
}

fn isValidJsonPayload(raw: []const u8) bool {
    const trimmed = std.mem.trim(u8, raw, " \t\r\n");
    if (trimmed.len == 0) return false;
    var parsed = std.json.parseFromSlice(std.json.Value, std.heap.page_allocator, trimmed, .{}) catch return false;
    defer parsed.deinit();
    return true;
}

//==============================================================================
// 3. Transport abstraction
//==============================================================================

pub const Transport = struct {
    ctx: *anyopaque,
    vtable: *const VTable,

    pub const VTable = struct {
        request: *const fn (
            ctx: *anyopaque,
            method: []const u8,
            path: []const u8,
            body: []const u8,
            out_buf: []u8,
        ) anyerror!usize,
    };

    pub fn request(self: Transport, method: []const u8, path: []const u8, body: []const u8, out_buf: []u8) !usize {
        return self.vtable.request(self.ctx, method, path, body, out_buf);
    }
};

pub const StubTransport = struct {
    pub fn make() Transport {
        return .{ .ctx = @ptrCast(@constCast(&dummy)), .vtable = &vt };
    }
    var dummy: u8 = 0;
    const vt = Transport.VTable{ .request = stubRequest };
    fn stubRequest(_: *anyopaque, method: []const u8, path: []const u8, body: []const u8, _: []u8) !usize {
        std.debug.print("[verisimdb stub] {s} {s} ({d} body bytes)\n", .{ method, path, body.len });
        return error.Unavailable;
    }
};

//==============================================================================
// 4. Client
//==============================================================================

pub const Client = struct {
    allocator: std.mem.Allocator,
    base_url: []const u8,
    transport: Transport,

    pub fn init(allocator: std.mem.Allocator, base_url: []const u8, transport: Transport) Client {
        return .{ .allocator = allocator, .base_url = base_url, .transport = transport };
    }

    pub fn health(self: *Client) StorageResult {
        var buf: [256]u8 = undefined;
        const n = self.transport.request("GET", "/api/v1/health", "", buf[0..]) catch return .unavailable;
        _ = n;
        return .ok;
    }

    pub fn createOctad(self: *Client, payload_json: []const u8, out_id: *[64]u8) StorageResult {
        if (!isValidJsonPayload(payload_json)) return .invalid;
        var buf: [4096]u8 = undefined;
        const n = self.transport.request("POST", "/api/v1/octads", payload_json, buf[0..]) catch return .unavailable;
        if (n == 0) return .err;
        @memset(out_id, 0);
        const max_copy: usize = if (n > out_id.len) out_id.len else n;
        @memcpy(out_id[0..max_copy], buf[0..max_copy]);
        return .ok;
    }

    pub fn getOctad(self: *Client, id: []const u8, out_buf: []u8) struct { rc: StorageResult, n: usize } {
        if (!isSafePathSegment(id)) return .{ .rc = .invalid, .n = 0 };
        var path_buf: [256]u8 = undefined;
        const path = std.fmt.bufPrint(&path_buf, "/api/v1/octads/{s}", .{id}) catch return .{ .rc = .invalid, .n = 0 };
        const n = self.transport.request("GET", path, "", out_buf) catch return .{ .rc = .unavailable, .n = 0 };
        return .{ .rc = .ok, .n = n };
    }

    pub fn temporalRecord(self: *Client, canvas_id: []const u8, op_name: []const u8, payload_json: []const u8) StorageResult {
        if (!isSafePathSegment(canvas_id)) return .invalid;
        if (!isValidJsonPayload(payload_json)) return .invalid;
        var path_buf: [256]u8 = undefined;
        const path = std.fmt.bufPrint(&path_buf, "/api/v1/octads/{s}/temporal", .{canvas_id}) catch return .invalid;

        var escaped_op_buf: [2048]u8 = undefined;
        const escaped_op = escapeJsonContent(&escaped_op_buf, op_name) catch return .invalid;

        var body_buf: [8192]u8 = undefined;
        const body = std.fmt.bufPrint(&body_buf,
            \\{{"op_name":"{s}","payload":{s}}}
        , .{ escaped_op, payload_json }) catch return .invalid;

        var resp_buf: [512]u8 = undefined;
        _ = self.transport.request("POST", path, body, resp_buf[0..]) catch return .unavailable;
        return .ok;
    }

    pub fn temporalWalk(self: *Client, canvas_id: []const u8, at_node: []const u8, out_buf: []u8) struct { rc: StorageResult, n: usize } {
        if (!isSafePathSegment(canvas_id) or !isSafePathSegment(at_node)) {
            return .{ .rc = .invalid, .n = 0 };
        }
        var path_buf: [256]u8 = undefined;
        const path = std.fmt.bufPrint(&path_buf, "/api/v1/octads/{s}/temporal?at={s}", .{ canvas_id, at_node }) catch return .{ .rc = .invalid, .n = 0 };
        const n = self.transport.request("GET", path, "", out_buf) catch return .{ .rc = .unavailable, .n = 0 };
        return .{ .rc = .ok, .n = n };
    }

    pub fn queryVQL(self: *Client, vql: []const u8, out_buf: []u8) struct { rc: StorageResult, n: usize } {
        if (vql.len == 0) return .{ .rc = .invalid, .n = 0 };
        var escaped_vql_buf: [7936]u8 = undefined;
        const escaped_vql = escapeJsonContent(&escaped_vql_buf, vql) catch return .{ .rc = .invalid, .n = 0 };
        var body_buf: [8192]u8 = undefined;
        const body = std.fmt.bufPrint(&body_buf,
            \\{{"query":"{s}"}}
        , .{escaped_vql}) catch return .{ .rc = .invalid, .n = 0 };
        const n = self.transport.request("POST", "/api/v1/query", body, out_buf) catch return .{ .rc = .unavailable, .n = 0 };
        return .{ .rc = .ok, .n = n };
    }
};

//==============================================================================
// 5. Convenience: a Client wired with the stub transport.
//==============================================================================

pub fn newStubClient(allocator: std.mem.Allocator) Client {
    return Client.init(allocator, "http://localhost:8080", StubTransport.make());
}

//==============================================================================
// 6. Tests
//==============================================================================

test "stub transport returns Unavailable for every valid endpoint" {
    var client = newStubClient(std.testing.allocator);
    try std.testing.expectEqual(StorageResult.unavailable, client.health());

    var oid: [64]u8 = undefined;
    try std.testing.expectEqual(StorageResult.unavailable, client.createOctad("{}", &oid));

    var buf: [256]u8 = undefined;
    const got = client.getOctad("test-id", buf[0..]);
    try std.testing.expectEqual(StorageResult.unavailable, got.rc);

    try std.testing.expectEqual(StorageResult.unavailable, client.temporalRecord("c1", "stroke.brush", "{}"));

    const walked = client.temporalWalk("c1", "node-5", buf[0..]);
    try std.testing.expectEqual(StorageResult.unavailable, walked.rc);

    const queried = client.queryVQL("SELECT * FROM octads LIMIT 1", buf[0..]);
    try std.testing.expectEqual(StorageResult.unavailable, queried.rc);
}

test "path injection and JSON breakout inputs are rejected or escaped safely (B-23)" {
    const Capture = struct {
        last_body: [512]u8 = undefined,
        last_len: usize = 0,

        fn req(ctx: *anyopaque, _: []const u8, _: []const u8, body: []const u8, out: []u8) !usize {
            const self: *@This() = @ptrCast(@alignCast(ctx));
            @memcpy(self.last_body[0..body.len], body);
            self.last_len = body.len;
            out[0] = 'o';
            return 1;
        }
    };

    var cap = Capture{};
    const vt = Transport.VTable{ .request = Capture.req };
    var client = Client.init(std.testing.allocator, "http://localhost:8080", .{ .ctx = &cap, .vtable = &vt });

    var buf: [256]u8 = undefined;
    try std.testing.expectEqual(StorageResult.invalid, client.getOctad("../admin", &buf).rc);
    try std.testing.expectEqual(StorageResult.invalid, client.getOctad("id?admin=1", &buf).rc);
    try std.testing.expectEqual(StorageResult.invalid, client.temporalWalk("c1", "n1&raw=true", &buf).rc);
    try std.testing.expectEqual(StorageResult.invalid, client.temporalRecord("c1", "op", "not-json"));

    try std.testing.expectEqual(StorageResult.ok, client.temporalRecord("c1", "quote\"inject\nline", "{\"x\":1}"));
    const parsed = try std.json.parseFromSlice(std.json.Value, std.testing.allocator, cap.last_body[0..cap.last_len], .{});
    defer parsed.deinit();
    try std.testing.expectEqualStrings("quote\"inject\nline", parsed.value.object.get("op_name").?.string);
}

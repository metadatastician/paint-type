// SPDX-License-Identifier: MPL-2.0
//
// paint.type desktop shell — Zig build with Gossamer integration (Zig 0.15+).
//
// This build integrates with the Gossamer webview library (libgossamer) instead
// of linking directly against GTK/WebKitGTK. Gossamer provides a unified
// cross-platform webview abstraction that handles:
//   - Linux/BSD: GTK 3 + WebKitGTK 4.1
//   - macOS: Cocoa + WebKit
//   - Windows: Win32 + WebView2
//
// The shell build links against libgossamer (shared library) which must be
// built and installed separately from hyperpolymath/gossamer.
//
// To build Gossamer:
//   cd ../gossamer/src/interface/ffi
//   zig build -Doptimize=ReleaseSafe
//   sudo cp zig-out/lib/libgossamer.so /usr/local/lib
//   sudo cp ../gossamer.h /usr/local/include
//
// Linux/BSD system deps (via libgossamer): libgtk-3-dev libwebkit2gtk-4.1-dev

const std = @import("std");

pub fn build(b: *std.Build) void {
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});

    const mod = b.createModule(.{
        .root_source_file = b.path("main.zig"),
        .target = target,
        .optimize = optimize,
    });
    mod.link_libc = true;

    // Platform-specific dependencies
    // Note: libgossamer handles webview abstraction, but we link directly
    // to GTK for the smoke timeout mechanism
    switch (target.result.os.tag) {
        .linux, .freebsd, .openbsd, .netbsd => {
            mod.linkSystemLibrary("gtk+-3.0", .{});
            mod.linkSystemLibrary("webkit2gtk-4.1", .{});
            mod.linkSystemLibrary("glib-2.0", .{});
        },
        .macos => {
            mod.linkFramework("Cocoa", .{});
            mod.linkFramework("WebKit", .{});
        },
        else => {},
    }

    // Link against libgossamer if available (optional for now)
    // This will use the system library path
    mod.linkSystemLibrary("gossamer", .{});

    // Add include path for gossamer.h
    mod.addIncludePath(b.path("../../gossamer/src/interface"));

    const exe = b.addExecutable(.{
        .name = "paint-type-shell",
        .root_module = mod,
    });
    b.installArtifact(exe);

    const run_cmd = b.addRunArtifact(exe);
    if (b.args) |args| run_cmd.addArgs(args);
    const run_step = b.step("run", "Build and launch the paint.type desktop shell");
    run_step.dependOn(&run_cmd.step);
}

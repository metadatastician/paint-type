// SPDX-License-Identifier: MPL-2.0
//
// paint.type desktop shell — Gossamer integration (v0.3.0).
//
// A minimal webview host using the Gossamer FFI library (libgossamer).
// This replaces the previous direct GTK3 + WebKitGTK calls with the
// unified Gossamer API surface, enabling cross-platform support and
// integration with the estate's webview abstraction layer.
//
// The shell creates a Gossamer webview window, loads the embedded empty
// canvas HTML (src/shell/web/index.html), and runs the event loop.
//
// Smoke mode: with PT_SHELL_SMOKE set (or `--smoke`), the shell schedules a
// timeout that quits the main loop shortly after the window is shown, so
// `tests/e2e.sh` can verify "open app -> empty canvas visible -> quit clean"
// headlessly under Xvfb. The two stderr markers (`canvas-ready`, `quit-clean`)
// are the harness's assertions.
//
// Integration notes:
//   - Links against libgossamer (shared library) built from hyperpolymath/gossamer
//   - Uses gossamer_create_ex() for full window configuration control
//   - Uses gossamer_load_html() to load the embedded canvas HTML
//   - Uses gossamer_run() to start the GTK main event loop
//   - Window is automatically cleaned up when gossamer_run() returns

const std = @import("std");

// Import the Gossamer C FFI bindings
const gossamer = @cImport({
    @cInclude("gossamer.h");
});

// Import GTK for the smoke timeout mechanism
const gtk = @cImport({
    @cInclude("gtk/gtk.h");
});

/// The empty-canvas page, embedded so the shell has no runtime file dependency.
const INDEX_HTML: [*:0]const u8 = @embedFile("web/index.html");

/// Check if smoke mode is requested via environment variable or command-line flag.
fn smokeRequested() bool {
    if (std.posix.getenv("PT_SHELL_SMOKE") != null) return true;
    var args = std.process.args();
    _ = args.next(); // argv[0]
    while (args.next()) |a| {
        if (std.mem.eql(u8, a, "--smoke")) return true;
    }
    return false;
}

/// One-shot GTK timeout callback for smoke mode.
/// Uses the GTK g_timeout_add mechanism directly since we're linking
/// against GTK through libgossamer anyway.
fn onSmokeTimeout(_: ?*anyopaque) callconv(.C) c_int {
    // Quit the GTK main loop directly
    gtk.gtk_main_quit();
    return 0; // G_SOURCE_REMOVE - don't repeat
}

pub fn main() u8 {
    // Create a Gossamer webview window with full configuration.
    // Parameters: title, width, height, min_width, min_height, max_width, max_height,
    //             resizable, decorations, fullscreen, visible
    const handle_ptr = gossamer.gossamer_create_ex(
        "paint.type",
        1024,
        768,
        0,  // min_width (unset)
        0,  // min_height (unset)
        0,  // max_width (unset)
        0,  // max_height (unset)
        1,  // resizable (true)
        1,  // decorations (true - has window chrome)
        0,  // fullscreen (false)
        1,  // visible (true - show immediately)
    );

    if (handle_ptr == 0) {
        const err = gossamer.gossamer_last_error();
        if (err != null) {
            std.debug.print("PT_SHELL: gossamer_create_ex failed: {s}\n", .{err});
        } else {
            std.debug.print("PT_SHELL: gossamer_create_ex returned null handle\n", .{});
        }
        return 1;
    }

    // Load the embedded canvas HTML into the webview
    const load_result = gossamer.gossamer_load_html(handle_ptr, INDEX_HTML);
    if (load_result != 0) {
        const err = gossamer.gossamer_last_error();
        std.debug.print("PT_SHELL: gossamer_load_html failed: {s}\n", .{err != null orelse "unknown error"});
        gossamer.gossamer_destroy(handle_ptr);
        return 1;
    }

    // Window + web view created and the canvas HTML submitted to WebKit.
    std.debug.print("PT_SHELL: canvas-ready\n", .{});

    // If smoke mode is requested, schedule a GTK timeout to quit after 800ms
    if (smokeRequested()) {
        _ = gtk.g_timeout_add(800, @ptrCast(&onSmokeTimeout), null);
    }

    // Run the event loop. This blocks until the window is closed.
    // When it returns, the handle is automatically cleaned up.
    gossamer.gossamer_run(handle_ptr);

    std.debug.print("PT_SHELL: quit-clean\n", .{});
    return 0;
}

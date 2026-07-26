// SPDX-License-Identifier: MPL-2.0
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
// Gossamer Webview Shell - C Header for FFI
//
// This header declares the C-compatible interface for the Gossamer webview library.
// It matches the export functions in src/interface/ffi/src/main.zig

#ifndef GOSSAMER_H
#define GOSSAMER_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque handle types
typedef uint64_t gossamer_handle_t;
typedef uint64_t gossamer_channel_t;

// Result codes
typedef int32_t gossamer_result_t;
#define GOSSAMER_OK 0
#define GOSSAMER_ERROR -1

// --- Core Window Functions ---

/// Create a new webview window with basic parameters
/// @param title Window title (UTF-8 null-terminated string)
/// @param width Initial width in pixels
/// @param height Initial height in pixels
/// @param resizable Whether the window can be resized (0 = false, 1 = true)
/// @param decorations Whether to show window decorations (0 = false, 1 = true)
/// @param fullscreen Whether to start in fullscreen mode (0 = false, 1 = true)
/// @return Window handle, or 0 on failure
gossamer_handle_t gossamer_create(
    const char* title,
    uint32_t width,
    uint32_t height,
    uint8_t resizable,
    uint8_t decorations,
    uint8_t fullscreen
);

/// Create a new webview window with extended parameters
/// @param title Window title (UTF-8 null-terminated string)
/// @param width Initial width in pixels
/// @param height Initial height in pixels
/// @param min_width Minimum width in pixels (0 = unset)
/// @param min_height Minimum height in pixels (0 = unset)
/// @param max_width Maximum width in pixels (0 = unset)
/// @param max_height Maximum height in pixels (0 = unset)
/// @param resizable Whether the window can be resized (0 = false, 1 = true)
/// @param decorations Whether to show window decorations (0 = false, 1 = true)
/// @param fullscreen Whether to start in fullscreen mode (0 = false, 1 = true)
/// @param visible Whether to show the window immediately (0 = false, 1 = true)
/// @return Window handle, or 0 on failure
gossamer_handle_t gossamer_create_ex(
    const char* title,
    uint32_t width,
    uint32_t height,
    uint32_t min_width,
    uint32_t min_height,
    uint32_t max_width,
    uint32_t max_height,
    uint8_t resizable,
    uint8_t decorations,
    uint8_t fullscreen,
    uint8_t visible
);

/// Load HTML content into a webview
/// @param handle Window handle
/// @param html HTML content (UTF-8 null-terminated string)
/// @return GOSSAMER_OK on success, GOSSAMER_ERROR on failure
gossamer_result_t gossamer_load_html(gossamer_handle_t handle, const char* html);

/// Run the main event loop for a webview window
/// @param handle Window handle
/// This function does not return until the window is closed
void gossamer_run(gossamer_handle_t handle);

/// Destroy a webview window
/// @param handle Window handle
void gossamer_destroy(gossamer_handle_t handle);

/// Get the last error message
/// @return Error message string, or NULL if no error
const char* gossamer_last_error(void);

/// Set window title
/// @param handle Window handle
/// @param title New title (UTF-8 null-terminated string)
/// @return GOSSAMER_OK on success, GOSSAMER_ERROR on failure
gossamer_result_t gossamer_set_title(gossamer_handle_t handle, const char* title);

/// Resize window
/// @param handle Window handle
/// @param width New width in pixels
/// @param height New height in pixels
/// @return GOSSAMER_OK on success, GOSSAMER_ERROR on failure
gossamer_result_t gossamer_resize(gossamer_handle_t handle, uint32_t width, uint32_t height);

/// Show window
/// @param handle Window handle
/// @return GOSSAMER_OK on success, GOSSAMER_ERROR on failure
gossamer_result_t gossamer_show(gossamer_handle_t handle);

/// Hide window
/// @param handle Window handle
/// @return GOSSAMER_OK on success, GOSSAMER_ERROR on failure
gossamer_result_t gossamer_hide(gossamer_handle_t handle);

/// Minimize window
/// @param handle Window handle
/// @return GOSSAMER_OK on success, GOSSAMER_ERROR on failure
gossamer_result_t gossamer_minimize(gossamer_handle_t handle);

/// Maximize window
/// @param handle Window handle
/// @return GOSSAMER_OK on success, GOSSAMER_ERROR on failure
gossamer_result_t gossamer_maximize(gossamer_handle_t handle);

/// Restore window (from minimized or maximized)
/// @param handle Window handle
/// @return GOSSAMER_OK on success, GOSSAMER_ERROR on failure
gossamer_result_t gossamer_restore(gossamer_handle_t handle);

/// Get library version
/// @return Version string
const char* gossamer_version(void);

/// Get library build info
/// @return Build info string
const char* gossamer_build_info(void);

#ifdef __cplusplus
}
#endif

#endif // GOSSAMER_H
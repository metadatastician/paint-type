#!/usr/bin/env bash
# SPDX-License-Identifier: MPL-2.0
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
#
# @a2ml-metadata begin
# (
#   id                   = "paint-type-launcher"
#   type                 = "launcher"
#   version              = "0.2.1"
#   app-name             = "paint-type"
#   app-display          = "paint.type"
#   app-url              = ""
#   runtime-kind         = "process"
#   standards-compliance = [
#     "launcher-standard.adoc"
#     "LM-LA-LIFECYCLE-STANDARD.adoc"
#     "cross-platform-system-integration-modes"
#   ]
#   standard-spec-version = "0.3.0"
#   generator             = "launch-scaffolder"
# )
# @a2ml-metadata end

set -euo pipefail

# ----------------------------------------------------------------------------
# CONFIGURATION
# ----------------------------------------------------------------------------

APP_NAME="paint-type"
APP_DISPLAY="paint.type"
APP_DESC="The open, cross-platform image editor Paint.NET users are actually asking for"
APP_CATEGORIES="Graphics;Editor;Development;"
APP_GENERIC_NAME="paint.type"
RUNTIME_KIND="process"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="${PAINT_TYPE_REPO_DIR:-$SCRIPT_DIR}"
ICON_SOURCE="$REPO_DIR/assets/icon-256.png"
DESKTOP_TOOLS_DIR="${DESKTOP_TOOLS_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/desktop-tools}"
CONFIG_FILE="${PAINT_TYPE_LAUNCHER_CONFIG:-$REPO_DIR/paint-type.launcher.a2ml}"

URL=""

PID_FILE="${XDG_RUNTIME_DIR:-${TMPDIR:-/tmp}}/paint-type.pid"
LOG_FILE="${XDG_STATE_HOME:-$HOME/.local/state}/paint-type/paint-type.log"

# Search list from [runtime].startup-command-search — first executable wins.
START_COMMAND=""
PATH_CANDIDATE="$(command -v paint-type || true)"
for candidate in \
    "$REPO_DIR/src/host/target/release/paint-type" \
    "$REPO_DIR/target/release/paint-type" \
    "$REPO_DIR/src/shell/zig-out/bin/paint-type-shell" \
    "$REPO_DIR/paint-type" \
    "$PATH_CANDIDATE"
do
    if [ -n "$candidate" ] && [ -x "$candidate" ]; then
        START_COMMAND="$candidate"
        break
    fi
done

RUNTIME_LIB_PATH="${LD_LIBRARY_PATH:-}"
for lib_dir in \
    "$REPO_DIR/third_party/gossamer/src/interface/ffi/zig-out/lib" \
    "$REPO_DIR/src/interface/ffi/zig-out/lib"
do
    if [ -d "$lib_dir" ]; then
        if [ -n "$RUNTIME_LIB_PATH" ]; then
            RUNTIME_LIB_PATH="$lib_dir:$RUNTIME_LIB_PATH"
        else
            RUNTIME_LIB_PATH="$lib_dir"
        fi
    fi
done

MODE="${1:---start}"
FORCE="false"
if [[ "${2:-}" == "--force" ]]; then
    FORCE="true"
fi

# ----------------------------------------------------------------------------
# LOGGING HELPERS
# ----------------------------------------------------------------------------

log()  { printf '\033[0;32m[%s]\033[0m %s\n' "$APP_DISPLAY" "$1"; }
warn() { printf '\033[0;33m[%s]\033[0m %s\n' "$APP_DISPLAY" "$1" >&2; }
err()  { printf '\033[0;31m[%s]\033[0m ERROR: %s\n' "$APP_DISPLAY" "$1" >&2; }

is_gui_context() {
    [ ! -t 2 ] && { [ -n "${DISPLAY:-}" ] || [ -n "${WAYLAND_DISPLAY:-}" ]; }
}

gui_error() {
    local title="$1"
    local body="$2"
    err "$title"
    printf '%s\n' "$body" | sed 's/^/  /' >&2
    if is_gui_context; then
        if   command -v kdialog     >/dev/null 2>&1; then kdialog --title "$APP_DISPLAY: $title" --error "$body" 2>/dev/null &
        elif command -v zenity      >/dev/null 2>&1; then zenity --error --title="$APP_DISPLAY: $title" --text="$body" --width=500 2>/dev/null &
        elif command -v notify-send >/dev/null 2>&1; then notify-send --urgency=critical --icon=dialog-error "$APP_DISPLAY: $title" "$body" 2>/dev/null &
        elif command -v xmessage    >/dev/null 2>&1; then xmessage -center "$APP_DISPLAY: $title\n\n$body" 2>/dev/null &
        fi
    fi
}

# ----------------------------------------------------------------------------
# PLATFORM DETECTION
# ----------------------------------------------------------------------------

detect_platform() {
    case "$(uname -s)" in
        Linux*)                          echo "linux"   ;;
        Darwin*)                         echo "macos"   ;;
        CYGWIN*|MINGW*|MSYS*|Windows_NT) echo "windows" ;;
        *)                               echo "unknown" ;;
    esac
}
PLATFORM="$(detect_platform)"

case "$PLATFORM" in
    linux)
        APPS_DIR="$HOME/.local/share/applications"
        ICON_DIR="$HOME/.local/share/icons/hicolor/256x256/apps"
        DESKTOP_SHORTCUT_DIR="$HOME/Desktop"
        BIN_DIR="$HOME/.local/bin"
        DESKTOP_FILE_TARGET="$APPS_DIR/${APP_NAME}.desktop"
        DESKTOP_SHORTCUT_TARGET="$DESKTOP_SHORTCUT_DIR/${APP_NAME}.desktop"
        ICON_TARGET="$ICON_DIR/${APP_NAME}.png"
        LAUNCHER_TARGET="$BIN_DIR/${APP_NAME}-launcher"
        ;;
    macos)
        APPS_DIR="$HOME/Applications"
        DESKTOP_SHORTCUT_DIR="$HOME/Desktop"
        BIN_DIR="$HOME/.local/bin"
        DESKTOP_FILE_TARGET="$APPS_DIR/${APP_DISPLAY}.app"
        DESKTOP_SHORTCUT_TARGET="$DESKTOP_SHORTCUT_DIR/${APP_DISPLAY}.command"
        ICON_TARGET="$APPS_DIR/${APP_DISPLAY}.app/Contents/Resources/icon.png"
        LAUNCHER_TARGET="$BIN_DIR/${APP_NAME}-launcher"
        ;;
    windows)
        APPDATA_DIR="${APPDATA:-$HOME/AppData/Roaming}"
        START_MENU_DIR="$APPDATA_DIR/Microsoft/Windows/Start Menu/Programs"
        DESKTOP_SHORTCUT_DIR="$HOME/Desktop"
        BIN_DIR="$HOME/.local/bin"
        DESKTOP_FILE_TARGET="$START_MENU_DIR/${APP_DISPLAY}.lnk"
        DESKTOP_SHORTCUT_TARGET="$DESKTOP_SHORTCUT_DIR/${APP_DISPLAY}.lnk"
        ICON_TARGET="$BIN_DIR/${APP_NAME}.ico"
        LAUNCHER_TARGET="$BIN_DIR/${APP_NAME}-launcher.sh"
        ;;
    *)
        APPS_DIR=""
        ICON_DIR=""
        DESKTOP_SHORTCUT_DIR=""
        BIN_DIR="$HOME/.local/bin"
        DESKTOP_FILE_TARGET=""
        DESKTOP_SHORTCUT_TARGET=""
        ICON_TARGET=""
        LAUNCHER_TARGET="$BIN_DIR/${APP_NAME}-launcher"
        ;;
esac

# ----------------------------------------------------------------------------
# PROCESS MANAGEMENT
# ----------------------------------------------------------------------------

read_pid() {
    cat "$PID_FILE"
}

is_running() {
    [ -f "$PID_FILE" ] && kill -0 "$(read_pid)" 2>/dev/null
}

clear_stale_pid() {
    if [ -f "$PID_FILE" ] && ! kill -0 "$(read_pid)" 2>/dev/null; then
        rm -f "$PID_FILE"
    fi
}

start_server() {
    clear_stale_pid
    if is_running; then
        log "Already running (PID $(read_pid))"
        return 0
    fi

    if [ -z "$START_COMMAND" ] || [ ! -x "$START_COMMAND" ]; then
        gui_error "No startup command found" \
"$APP_DISPLAY binary was not found. Run ./scripts/build-host.sh first or place paint-type on PATH."
        return 1
    fi

    mkdir -p "$(dirname "$LOG_FILE")" "$(dirname "$PID_FILE")"
    log "Starting $APP_DISPLAY..."
    cd "$REPO_DIR" 2>/dev/null || true
    if [ -n "$RUNTIME_LIB_PATH" ]; then
        LD_LIBRARY_PATH="$RUNTIME_LIB_PATH" nohup "$START_COMMAND" >"$LOG_FILE" 2>&1 &
    else
        nohup "$START_COMMAND" >"$LOG_FILE" 2>&1 &
    fi
    printf '%s\n' "$!" > "$PID_FILE"

    sleep 0.2
    if ! kill -0 "$(read_pid)" 2>/dev/null; then
        gui_error "Process exited immediately" "Check $LOG_FILE"
        rm -f "$PID_FILE"
        return 1
    fi

    log "Started (PID $(read_pid))"
    return 0
}

stop_server() {
    if ! is_running; then
        log "No running instance found"
        return 0
    fi
    log "Stopping $APP_DISPLAY..."
    kill "$(read_pid)" 2>/dev/null || true
    rm -f "$PID_FILE"
    log "Stopped"
}

open_browser() {
    if [ -z "$URL" ]; then
        log "No URL configured for $APP_DISPLAY"
        return 1
    fi

    log "Opening $URL..."
    case "$PLATFORM" in
        linux)
            if   command -v xdg-open >/dev/null 2>&1; then xdg-open "$URL" &
            elif command -v firefox  >/dev/null 2>&1; then firefox  "$URL" &
            elif command -v chromium >/dev/null 2>&1; then chromium "$URL" &
            else warn "No browser found — open manually: $URL"; fi ;;
        macos)   open "$URL" 2>/dev/null || warn "Open manually: $URL" ;;
        windows)
            if   command -v start    >/dev/null 2>&1; then start    "$URL"
            elif command -v cygstart >/dev/null 2>&1; then cygstart "$URL"
            else warn "Open manually: $URL"; fi ;;
        *) warn "Unknown platform — open manually: $URL" ;;
    esac
}

# ----------------------------------------------------------------------------
# SYSTEM INTEGRATION — --integ / --disinteg
# ----------------------------------------------------------------------------

already_integrated() {
    { [ -n "$DESKTOP_FILE_TARGET" ] && [ -f "$DESKTOP_FILE_TARGET" ]; } || [ -f "$LAUNCHER_TARGET" ]
}

write_linux_desktop_file() {
    local target="$1"
    local icon_name
    if [ -n "$ICON_TARGET" ] && [ -f "$ICON_TARGET" ]; then
        icon_name="$APP_NAME"
    else
        icon_name="package-x-generic"
    fi

    local keepopen="$DESKTOP_TOOLS_DIR/keepopen.sh"
    local gui_cmd tui_cmd exec_line
    gui_cmd="$LAUNCHER_TARGET --start && tail -f $LOG_FILE"
    tui_cmd="tail -n 200 -f $LOG_FILE"
    if [ -x "$keepopen" ]; then
        exec_line="$keepopen \"$APP_DISPLAY\" \"$REPO_DIR\" \"$gui_cmd\" \"$tui_cmd\" \"$LOG_FILE\""
    else
        exec_line="$LAUNCHER_TARGET --start"
    fi

    cat > "$target" <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=$APP_DISPLAY
GenericName=$APP_GENERIC_NAME
Comment=$APP_DESC
Exec=$exec_line
Icon=$icon_name
Terminal=false
Categories=$APP_CATEGORIES
StartupNotify=true
StartupWMClass=$APP_NAME
Actions=stop;status;

[Desktop Action stop]
Name=Stop
Exec=$LAUNCHER_TARGET --stop

[Desktop Action status]
Name=Status
Exec=$LAUNCHER_TARGET --status
EOF
    chmod 644 "$target"
}

do_integ_linux() {
    mkdir -p "$APPS_DIR" "$ICON_DIR" "$BIN_DIR" "$DESKTOP_SHORTCUT_DIR"
    local script_path
    script_path="$SCRIPT_DIR/$(basename "${BASH_SOURCE[0]}")"
    cp "$script_path" "$LAUNCHER_TARGET"
    chmod +x "$LAUNCHER_TARGET"
    log "  + launcher: $LAUNCHER_TARGET"

    if [ -n "$ICON_SOURCE" ] && [ -f "$ICON_SOURCE" ]; then
        cp "$ICON_SOURCE" "$ICON_TARGET"
        log "  + icon:     $ICON_TARGET"
    else
        log "  · no custom icon — using system fallback (package-x-generic)"
    fi

    write_linux_desktop_file "$DESKTOP_FILE_TARGET"
    log "  + menu:     $DESKTOP_FILE_TARGET"
    write_linux_desktop_file "$DESKTOP_SHORTCUT_TARGET"
    log "  + desktop:  $DESKTOP_SHORTCUT_TARGET"

    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPS_DIR" 2>/dev/null || true
    fi
    if command -v gio >/dev/null 2>&1; then
        gio set "$DESKTOP_FILE_TARGET"     "metadata::trusted" true 2>/dev/null || true
        gio set "$DESKTOP_SHORTCUT_TARGET" "metadata::trusted" true 2>/dev/null || true
    fi
    if [ -x "$DESKTOP_TOOLS_DIR/verify-desktop-integrity.sh" ]; then
        if "$DESKTOP_TOOLS_DIR/verify-desktop-integrity.sh" --generate 2>/dev/null; then
            log "  + integrity hashes generated"
        else
            log "  · integrity hash generation failed (non-fatal)"
        fi
    fi
}

do_integ() {
    if command -v launch-scaffolder >/dev/null 2>&1 && [ -n "$CONFIG_FILE" ] && [ -f "$CONFIG_FILE" ]; then
        local forward=(launch-scaffolder provision --integ "$CONFIG_FILE" --no-confirm)
        if [ "$FORCE" = "true" ]; then
            forward+=(--force)
        fi
        log "Delegating to launch-scaffolder provision..."
        exec "${forward[@]}"
    fi

    if already_integrated && [ "$FORCE" != "true" ]; then
        warn "$APP_DISPLAY is already integrated."
        read -rp "Reinstall? [y/N] " confirm
        if [[ ! "$confirm" =~ ^[Yy]$ ]]; then
            log "Nothing changed."
            return 0
        fi
    fi
    log "Integrating $APP_DISPLAY with the $PLATFORM desktop..."
    case "$PLATFORM" in
        linux) do_integ_linux ;;
        *)     err "Only Linux integration is implemented in the generated template so far."; return 1 ;;
    esac
    log "✓ $APP_DISPLAY is now in your menu and on your Desktop."
}

do_disinteg() {
    if is_running; then
        log "  • stopping running server"
        stop_server
    fi
    if command -v launch-scaffolder >/dev/null 2>&1 && [ -n "$CONFIG_FILE" ] && [ -f "$CONFIG_FILE" ]; then
        log "Delegating to launch-scaffolder provision..."
        exec launch-scaffolder provision --disinteg "$CONFIG_FILE" --no-confirm
    fi

    log "Removing $APP_DISPLAY system integration..."
    local removed_anything="false"
    local targets=(
        "$DESKTOP_FILE_TARGET"
        "$DESKTOP_SHORTCUT_TARGET"
        "$ICON_TARGET"
        "$LAUNCHER_TARGET"
    )
    local t
    for t in "${targets[@]}"; do
        [ -z "$t" ] && continue
        if [ -e "$t" ] || [ -L "$t" ]; then
            rm -rf "$t"
            log "  - removed $t"
            removed_anything="true"
        fi
    done
    if [ "$PLATFORM" = "linux" ] && command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPS_DIR" 2>/dev/null || true
    fi
    rm -f "$PID_FILE"
    if [ "$removed_anything" = "true" ]; then
        log "✓ $APP_DISPLAY removed from your system."
    else
        log "Nothing to remove — $APP_DISPLAY was not integrated here."
    fi
}

show_help() {
    cat <<EOF
$APP_DISPLAY launcher — $APP_DESC

Usage: $0 [MODE] [--force]

Runtime modes:

  --start      Start the process (default)
  --stop       Stop the process
  --status     Show running status
  --auto       Alias for --start

System integration:
  --integ      Install desktop entry + shortcut + icon (idempotent; --force to reinstall)
  --disinteg   Remove everything --integ installed (idempotent)

Misc:
  --help       This text

Detected platform: $PLATFORM
Runtime kind:      $RUNTIME_KIND
Repo:              $REPO_DIR
EOF
}

# ----------------------------------------------------------------------------
# MAIN SWITCH
# ----------------------------------------------------------------------------

case "$MODE" in
    --start)          start_server ;;
    --stop)           stop_server ;;
    --status)
        if is_running; then
            log "Running (PID $(read_pid))${URL:+ — $URL}"
        else
            log "Not running${URL:+ — $URL}"
        fi
        ;;
    --browser|--web)
        open_browser || true
        ;;
    --auto)
        start_server
        ;;
    --integ)          do_integ ;;
    --disinteg)       do_disinteg ;;
    --help|-h)        show_help ;;
    *)
        err "Unknown mode: $MODE"
        show_help
        exit 2
        ;;
esac

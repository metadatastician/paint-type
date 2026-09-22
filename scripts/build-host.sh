#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath)
#
# build-host.sh -- the single recipe that builds `paint-type`, the desktop
# binary this project is named after.
#
# Before this script the recipe existed in four places that could drift:
# .github/workflows/host.yml, the Justfile, .github/workflows/release.yml and
# scripts/build-host-local.sh. CI and the release build must not be able to
# diverge, so everything now calls here.
#
# Note scripts/build-host-local.sh remains, and still does something this does
# not: it provisions a local sysroot from .deb packages for machines without
# the GTK/WebKit development headers. It should call this for the build itself.
#
# Env:
#   ZIG        zig executable            (default: zig on PATH)
#   CARGO_ARGS extra args to cargo build (default: none)
#
# On success prints the binary path and the directory holding libgossamer.so,
# the latter because the binary cannot start without it on the library path.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ZIG="${ZIG:-zig}"
GOSSAMER_FFI="$ROOT/third_party/gossamer/src/interface/ffi"
GOSSAMER_LIB="$GOSSAMER_FFI/zig-out/lib"
BIN="$ROOT/src/host/target/release/paint-type"

die() { printf 'build-host: %s\n' "$*" >&2; exit 1; }

command -v "$ZIG" >/dev/null 2>&1 || die "zig not found (set ZIG=/path/to/zig)"

# Pin the Zig major/minor explicitly. 0.16 removed std.posix.getenv and the
# lowercase std.io alias, both of which sit on the gossamer FFI path, so an
# unpinned `zig` produces a compile error hundreds of lines into a vendored
# tree. Failing here names the actual problem.
zig_version="$("$ZIG" version)"
case "$zig_version" in
    0.15.*) ;;
    *) die "need Zig 0.15.x, found $zig_version" ;;
esac
printf '==> zig %s\n' "$zig_version"

# libpt is on the pixel hot path, so it is built optimised even in a debug flow.
printf '==> libpt (ReleaseFast)\n'
( cd "$ROOT/src/interface/ffi" && "$ZIG" build -Doptimize=ReleaseFast )

printf '==> libgossamer\n'
( cd "$GOSSAMER_FFI" && "$ZIG" build )

# The Rust bindings emit a bare `-lgossamer`, so a missing shared object here
# surfaces as a link error about a library nobody mentioned in the sources.
[ -f "$GOSSAMER_LIB/libgossamer.so" ] \
    || die "libgossamer.so absent from $GOSSAMER_LIB after a successful zig build"

# $ORIGIN/../lib makes a packaged bin/ + lib/ layout self-contained, so an
# installed copy needs no LD_LIBRARY_PATH.
#
# In-tree this resolves to src/host/target/lib, which does not exist, and that
# is deliberate: it keeps the LD_LIBRARY_PATH negative control honest. If the
# rpath resolved in-tree, unsetting LD_LIBRARY_PATH would still launch and the
# control would assert nothing.
#
# Single quotes are load-bearing. An unquoted $ORIGIN expands to the empty
# string and leaves a RUNPATH of "/../lib", which still matches a naive
# `readelf -d | grep RUNPATH` -- a broken rpath that passes a careless test.
printf '==> paint-type (release, locked)\n'
RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-rpath,\$ORIGIN/../lib" \
    cargo build --release --locked \
        --manifest-path "$ROOT/src/host/Cargo.toml" ${CARGO_ARGS:-}

[ -x "$BIN" ] || die "cargo reported success but $BIN is missing"

# Assert the rpath by its literal text, for the reason given above.
if command -v readelf >/dev/null 2>&1; then
    readelf -d "$BIN" | grep -qF '$ORIGIN/../lib' \
        || die "rpath \$ORIGIN/../lib did not land in $BIN"
    printf '==> rpath ok: $ORIGIN/../lib\n'
fi

printf 'binary:  %s\n' "$BIN"
printf 'libdir:  %s\n' "$GOSSAMER_LIB"

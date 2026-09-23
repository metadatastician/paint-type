# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Joshua Jewell (JoshuaJewell) <paint-type@pm.me>
#
# Build-and-verify image for paint-type.
#
#   podman build -t paint-type:latest -f Containerfile .
#
# ---------------------------------------------------------------------------
# WHY THIS CONTAINER HAS NO ENTRYPOINT
# ---------------------------------------------------------------------------
# paint-type is a LIBRARY plus demo programs, not a service. The build
# produces:
#
#   zig-out/lib/libpt.a, libpt.so          -- src/interface/ffi (`zig build lib`)
#   paint_core, crate-type = ["rlib"]      -- src/paint_core, statically links libpt
#   composite_demo, brush_demo, undo_demo  -- demo executables (root build.zig)
#   pt_bench                               -- benchmark harness
#
# There is no long-running process to exec into and no port to bind, so a
# service ENTRYPOINT here would be fiction. Verification happens during
# `podman build`: the RUN steps below execute the same suites recorded in
# AFFIRMATION.adoc, so a successful build *is* the verification, and a
# failure fails the build rather than a container that never starts.
#
# The `container/` directory is unrendered estate template scaffolding for
# a *service* deployment (compose, entrypoint, port, health check). Its
# `{{PLACEHOLDER}}` tokens were never substituted and nothing in this repo
# produces the binary it expects. It is retained because the canon forbids
# destructive removal, but `container/README.adoc` now states that plainly
# instead of implying a deployable service exists.
# ---------------------------------------------------------------------------
#
# Base images pinned by digest. Chainguard rolls :latest frequently, so
# refresh deliberately, never silently:
#
#   podman pull cgr.dev/chainguard/wolfi-base:latest
#   podman inspect --format='{{index .RepoDigests 0}}' cgr.dev/chainguard/wolfi-base:latest

FROM cgr.dev/chainguard/wolfi-base:latest@sha256:34977aa13765da89f60fee8fe5230e2bb1c55192df08e383c58221ee0d1277fb AS build

# --- Toolchain -------------------------------------------------------------
# Pinned to match .tool-versions and mise.toml.
#
# Zig is not in the Wolfi repository, so it is fetched as a pinned release
# tarball and verified against its published SHA-256 rather than trusted.
# Recorded value is from https://ziglang.org/download/index.json under
# "0.15.1" -> "x86_64-linux" -> "shasum" (53734456 bytes); it matches a
# local download, checked 2026-09-23. Update both together.
ARG ZIG_VERSION=0.15.1
ARG ZIG_SHA256=c61c5da6edeea14ca51ecd5e4520c6f4189ef5250383db33d01848293bfafe05
ARG RUST_VERSION=1.85

RUN apk add --no-cache \
        ca-certificates curl tar xz \
        build-base git \
        "rust~=${RUST_VERSION}" "cargo~=${RUST_VERSION}"

RUN set -eux; \
    curl -fsSLO "https://ziglang.org/download/${ZIG_VERSION}/zig-x86_64-linux-${ZIG_VERSION}.tar.xz"; \
    echo "${ZIG_SHA256}  zig-x86_64-linux-${ZIG_VERSION}.tar.xz" | sha256sum -c -; \
    tar -xf "zig-x86_64-linux-${ZIG_VERSION}.tar.xz" -C /opt; \
    mv "/opt/zig-x86_64-linux-${ZIG_VERSION}" /opt/zig; \
    ln -s /opt/zig/zig /usr/local/bin/zig; \
    rm "zig-x86_64-linux-${ZIG_VERSION}.tar.xz"; \
    zig version

ENV PATH="/opt/zig:${PATH}"

# --- Source ----------------------------------------------------------------
WORKDIR /build
COPY . .

# --- Build -----------------------------------------------------------------
# libpt first: paint_core's build.rs shells out to `zig build` and links
# -lpt, so the Rust step cannot run before the Zig artifacts exist.
RUN set -eux; \
    cd src/interface/ffi && zig build lib && cd ../../..; \
    cargo build --release --manifest-path src/paint_core/Cargo.toml

# --- Verify ----------------------------------------------------------------
# The same three suites AFFIRMATION.adoc records, in the same order.
# Expected: 7/7 build steps and 23/23 Zig tests; 101 + 2 + 1 doctest
# = 104 Rust tests, 0 failed. A non-zero exit fails the image build.
RUN set -eux; \
    cd src/interface/ffi && zig build test --summary all && cd ../../..; \
    cargo test --release --manifest-path src/paint_core/Cargo.toml

# ---------------------------------------------------------------------------
# Artifact stage.
# ---------------------------------------------------------------------------
# `cgr.dev/chainguard/static` is a from-scratch image with no shell, no apk
# and no dynamic loader. That is deliberate: it can hold the static library
# this project publishes and nothing that pretends to run. Copying the
# toolchain here would not work -- rustc and cargo are glibc-linked -- so
# re-running the verification requires the `build` stage:
#
#   podman build --target build -t paint-type:verify -f Containerfile .
#
# Pinned by digest; refresh as above with cgr.dev/chainguard/static:latest.
FROM cgr.dev/chainguard/static:latest@sha256:77d8b8925dc27970ec2f48243f44c7a260d52c49cd778288e4ee97566e0cb75b

# The artifacts this project actually produces. There is no service binary
# to install, so nothing goes in /usr/local/bin.
COPY --from=build /build/src/interface/ffi/zig-out/lib/libpt.a   /lib/
COPY --from=build /build/src/interface/ffi/zig-out/lib/libpt.so* /lib/

USER nonroot

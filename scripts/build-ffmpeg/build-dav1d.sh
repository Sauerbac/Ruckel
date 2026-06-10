#!/usr/bin/env bash
# dav1d static build with native MSVC (meson/ninja). Invoked by build.ps1 inside an
# MSYS2 login shell with the vcvars64 environment and pinned nasm inherited.
# Inputs: BFF_SRC_DIR (C:/-style), BFF_PREFIX (C:/-style), BFF_CONFIGURE_ARGS (newline-joined).
set -euo pipefail

SRC_U=$(cygpath -u "$BFF_SRC_DIR")
mapfile -t ARGS <<< "$BFF_CONFIGURE_ARGS"

# MSYS2's coreutils /usr/bin/link shadows MSVC's link.exe in this shell; front-run
# PATH with the MSVC bin dir (cl.exe, link.exe, lib.exe) so the toolchain wins.
export PATH="$(dirname "$(cygpath -u "$BFF_MSVC_LINK")"):$PATH"

# meson and ninja are native Windows binaries: give them Windows-style paths.
cd "$SRC_U"
rm -rf build
meson setup build . --prefix="$BFF_PREFIX" "${ARGS[@]}"
ninja -C build
ninja -C build install

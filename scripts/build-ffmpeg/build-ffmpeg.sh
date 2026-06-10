#!/usr/bin/env bash
# FFmpeg static build with --toolchain=msvc. Invoked by build.ps1 inside an MSYS2 login
# shell with the vcvars64 environment and pinned nasm inherited. x264 and dav1d are
# found via the .pc files they installed into the prefix.
# Inputs: BFF_SRC_DIR (C:/-style), BFF_PREFIX (C:/-style), BFF_CONFIGURE_ARGS
# (newline-joined), BFF_MSVC_LINK (full Windows path to MSVC link.exe).
set -euo pipefail

SRC_U=$(cygpath -u "$BFF_SRC_DIR")
PREFIX_U=$(cygpath -u "$BFF_PREFIX")
mapfile -t ARGS <<< "$BFF_CONFIGURE_ARGS"

# MSYS2's coreutils /usr/bin/link shadows MSVC's link.exe in this shell; FFmpeg's msvc
# toolchain invokes plain `link`, so front-run PATH with the MSVC bin dir
# (cl.exe, link.exe, lib.exe) so the toolchain wins.
export PATH="$(dirname "$(cygpath -u "$BFF_MSVC_LINK")"):$PATH"

export PKG_CONFIG_PATH="$PREFIX_U/lib/pkgconfig"

cd "$SRC_U"
make distclean >/dev/null 2>&1 || true
./configure --prefix="$BFF_PREFIX" "${ARGS[@]}"
make -j"$(nproc)"
make install

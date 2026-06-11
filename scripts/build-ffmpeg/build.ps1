#Requires -Version 7
<#
.SYNOPSIS
Builds Ruckel's manifest-pinned minimal static FFmpeg (+ x264, dav1d) as MSVC-native
static libraries (ADR-0029).

.DESCRIPTION
Driven entirely by the committed src-tauri/ffmpeg-build-manifest.json: downloads and
SHA-256-verifies the pinned source tarballs, builds dav1d -> x264 -> FFmpeg with cl.exe
(one CRT family end to end, no MinGW objects), and installs static libs + headers +
pkg-config files into the gitignored src-tauri/ffmpeg-libs/, stamped with the manifest's
SHA-256. src-tauri/build.rs refuses to build if the stamp is missing or stale.

Prerequisites (see scripts/build-ffmpeg/README.md):
  - Visual Studio 2022 with the C++ x64 toolset (cl.exe)
  - MSYS2 (default C:\msys64; override with -Msys2Root or RUCKEL_MSYS2_ROOT)
    with packages: make, pkgconf, diffutils
  - meson + ninja on PATH (pip install meson ninja)
NASM is pinned in the manifest and fetched by this script; it is not a prerequisite.

.PARAMETER Clean
Delete the work dir and src-tauri/ffmpeg-libs/ first, forcing a from-scratch build.
#>
[CmdletBinding()]
param(
    [string]$Msys2Root = $(if ($env:RUCKEL_MSYS2_ROOT) { $env:RUCKEL_MSYS2_ROOT } else { 'C:\msys64' }),
    [switch]$Clean
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ScriptDir = $PSScriptRoot
$RepoRoot = (Resolve-Path (Join-Path $ScriptDir '..\..')).Path
$ManifestPath = Join-Path $RepoRoot 'src-tauri\ffmpeg-build-manifest.json'
$Prefix = Join-Path $RepoRoot 'src-tauri\ffmpeg-libs'
$WorkDir = Join-Path $ScriptDir '.work'
$DownloadDir = Join-Path $WorkDir 'downloads'
$SrcRoot = Join-Path $WorkDir 'src'
$ToolsDir = Join-Path $WorkDir 'tools'
$Tar = "$env:SystemRoot\System32\tar.exe"
$Curl = "$env:SystemRoot\System32\curl.exe"

function Step([string]$msg) { Write-Host "`n==> $msg" -ForegroundColor Cyan }

$Manifest = Get-Content $ManifestPath -Raw | ConvertFrom-Json

if ($Clean) {
    Step 'Cleaning work dir and ffmpeg-libs'
    Remove-Item -Recurse -Force $WorkDir, $Prefix -ErrorAction Ignore
}

# ---------------------------------------------------------------- prerequisites
Step 'Checking prerequisites'

$Bash = Join-Path $Msys2Root 'usr\bin\bash.exe'
if (-not (Test-Path $Bash)) {
    throw "MSYS2 not found at '$Msys2Root' (override with -Msys2Root or RUCKEL_MSYS2_ROOT). Install: winget install MSYS2.MSYS2"
}
$missing = (& $Bash -lc 'for t in make pkg-config diff; do command -v $t >/dev/null || echo $t; done') -join ' '
if ($missing) {
    throw "MSYS2 tools missing: $missing. Install: & '$Bash' -lc 'pacman -S --needed --noconfirm make pkgconf diffutils'"
}
foreach ($tool in 'meson', 'ninja') {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "$tool not found on PATH. Install: pip install meson ninja"
    }
}

Step 'Importing MSVC build environment (vcvars64)'
$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
if (-not (Test-Path $vswhere)) { throw 'vswhere.exe not found — install Visual Studio 2022 with the C++ workload.' }
$vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if (-not $vsPath) { throw 'No Visual Studio installation with the C++ x64 toolset found.' }
$vcvars = Join-Path $vsPath 'VC\Auxiliary\Build\vcvars64.bat'
cmd /s /c "`"$vcvars`" >nul 2>&1 && set" | ForEach-Object {
    if ($_ -match '^([^=]+)=(.*)$') {
        [Environment]::SetEnvironmentVariable($Matches[1], $Matches[2], 'Process')
    }
}
$cl = Get-Command cl -ErrorAction SilentlyContinue
if (-not $cl) { throw 'cl.exe not available after importing vcvars64.' }
# The MSVC linker, located next to cl.exe — passed into the MSYS2 builds so that the
# `link` they invoke can never resolve to coreutils /usr/bin/link.
$MsvcLink = Join-Path (Split-Path $cl.Source) 'link.exe'
if (-not (Test-Path $MsvcLink)) { throw "link.exe not found next to cl.exe ($MsvcLink)" }
Write-Host "  cl:   $($cl.Source)"
Write-Host "  link: $MsvcLink"

# ------------------------------------------------------- download + verify + extract
Step 'Downloading + verifying pinned sources'
New-Item -ItemType Directory -Force $DownloadDir, $SrcRoot, $ToolsDir | Out-Null

function Get-Pinned($pin, [string]$fileName) {
    $file = Join-Path $DownloadDir $fileName
    if ((Test-Path $file) -and ((Get-FileHash $file -Algorithm SHA256).Hash.ToLower() -eq $pin.sha256)) {
        Write-Host "  $fileName (cached, SHA-256 OK)"
        return $file
    }
    & $Curl -fSL --retry 3 -o $file $pin.url
    if ($LASTEXITCODE) { throw "Download failed: $($pin.url)" }
    $actual = (Get-FileHash $file -Algorithm SHA256).Hash.ToLower()
    if ($actual -ne $pin.sha256) {
        throw "SHA-256 mismatch for ${fileName}: manifest pins $($pin.sha256), got $actual"
    }
    Write-Host "  $fileName (downloaded, SHA-256 OK)"
    return $file
}

$tarballs = @{}
foreach ($name in 'dav1d', 'x264', 'ffmpeg') {
    $tarballs[$name] = Get-Pinned $Manifest.sources.$name $Manifest.sources.$name.tarball
}
$nasmZip = Get-Pinned $Manifest.tools.nasm $Manifest.tools.nasm.zip

Step 'Extracting (fresh)'
foreach ($name in 'dav1d', 'x264', 'ffmpeg') {
    $dir = Join-Path $SrcRoot $Manifest.sources.$name.extractedDir
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
    & $Tar -xf $tarballs[$name] -C $SrcRoot
    if ($LASTEXITCODE) { throw "Extract failed: $($tarballs[$name])" }
    Write-Host "  $($Manifest.sources.$name.extractedDir)"
}
$nasmDir = Join-Path $ToolsDir $Manifest.tools.nasm.extractedDir
if (Test-Path $nasmDir) { Remove-Item -Recurse -Force $nasmDir }
& $Tar -xf $nasmZip -C $ToolsDir
if ($LASTEXITCODE) { throw "Extract failed: $nasmZip" }
if (-not (Test-Path (Join-Path $nasmDir 'nasm.exe'))) { throw "nasm.exe not found in $nasmDir" }
$env:PATH = "$nasmDir;$env:PATH"

New-Item -ItemType Directory -Force $Prefix | Out-Null
$PrefixFwd = $Prefix -replace '\\', '/'

# --------------------------------------------------------------------- builds
# All three builds run inside an MSYS2 bash with the Windows environment (cl.exe,
# nasm, meson/ninja) inherited. Parameters travel via BFF_* env vars; configure
# flags come straight from the manifest, newline-joined.
$env:MSYS2_PATH_TYPE = 'inherit'
$env:CHERE_INVOKING = '1'
$env:MSYSTEM = 'MSYS'
# MSYS2's /etc/profile prefers an already-set ORIGINAL_PATH over the live PATH
# (`ORIGINAL_PATH="${ORIGINAL_PATH:-${PATH}}"`). If this process inherited one from a
# Git-for-Windows bash ancestor, the vcvars64 PATH additions would silently vanish
# inside the MSYS2 shell — clear it so `inherit` inherits the real thing.
Remove-Item env:ORIGINAL_PATH -ErrorAction Ignore
$env:BFF_PREFIX = $PrefixFwd
$env:BFF_MSVC_LINK = $MsvcLink

function Invoke-Msys2Build([string]$what, [string]$script, $source, $configureArgs) {
    Step "Building $what"
    $env:BFF_SRC_DIR = (Join-Path $SrcRoot $source.extractedDir) -replace '\\', '/'
    $env:BFF_CONFIGURE_ARGS = $configureArgs -join "`n"
    & $Bash -l ((Join-Path $ScriptDir $script) -replace '\\', '/')
    if ($LASTEXITCODE) { throw "$what build failed (exit $LASTEXITCODE)" }
}

# Normalize static lib names to <name>.lib: rusty_ffmpeg emits
# `cargo:rustc-link-lib=static=<name>` and FFmpeg's msvc configure translates `-l<name>`
# to `<name>.lib` — but meson emits lib<name>.a. Must run after dav1d/x264 install (so
# FFmpeg's configure link checks resolve them) and again after FFmpeg's own install.
# Exception: configure hardcodes `-lx264` -> `libx264.lib` (matching what x264's own
# cl build emits), so that one keeps its name; build.rs links it as `static=libx264`.
function Rename-StaticLibs {
    Step 'Normalizing static library names to <name>.lib'
    Get-ChildItem (Join-Path $Prefix 'lib') -File | ForEach-Object {
        if ($_.Name -eq 'libx264.lib') { return }
        if ($_.Name -match '^lib(.+?)\.(a|lib)$') {
            $target = Join-Path $_.DirectoryName "$($Matches[1]).lib"
            # The fresh lib<name>.a always wins. (An earlier version kept a
            # pre-existing <name>.lib and deleted the new .a instead — on
            # incremental re-runs that silently stranded the previous build's
            # libs behind a fresh manifest stamp, defeating the stale guard.)
            if (Test-Path $target) { Remove-Item $target }
            Move-Item $_.FullName $target
            Write-Host "  $($_.Name) -> $(Split-Path $target -Leaf)"
        }
    }
}

Invoke-Msys2Build 'dav1d (meson/ninja, MSVC)' 'build-dav1d.sh' $Manifest.sources.dav1d $Manifest.configure.dav1d
Invoke-Msys2Build 'x264 (CC=cl)' 'build-x264.sh' $Manifest.sources.x264 $Manifest.configure.x264
Rename-StaticLibs
Invoke-Msys2Build 'FFmpeg (--toolchain=msvc)' 'build-ffmpeg.sh' $Manifest.sources.ffmpeg $Manifest.configure.ffmpeg
Rename-StaticLibs

# ---------------------------------------------------------------------- stamp
Step 'Stamping ffmpeg-libs with the manifest hash'
$hash = (Get-FileHash $ManifestPath -Algorithm SHA256).Hash.ToLower()
Set-Content -Path (Join-Path $Prefix 'manifest-hash.txt') -Value $hash -NoNewline
Write-Host "  manifest-hash.txt = $hash"

# rustc bundles `static=` libs INTO librusty_ffmpeg.rlib when that crate is
# compiled (+bundle is the default), so later links reuse the rlib's embedded
# copy of the old libav objects and never re-read the fresh .lib files. Without
# this clean, a lib rebuild silently never reaches the binaries.
Step 'Invalidating the cached rusty_ffmpeg rlib (it bundles the libav objects)'
if (Get-Command cargo -ErrorAction SilentlyContinue) {
    cargo clean -p rusty_ffmpeg --manifest-path (Join-Path $RepoRoot 'src-tauri\Cargo.toml')
    Write-Host '  cargo clean -p rusty_ffmpeg done'
} else {
    Write-Warning 'cargo not on PATH — run `cargo clean -p rusty_ffmpeg` manually before the next build, or the old libs stay linked.'
}

Step 'Done'
Get-ChildItem (Join-Path $Prefix 'lib') -Filter '*.lib' | ForEach-Object {
    Write-Host ("  {0,-20} {1,10:N0} KB" -f $_.Name, ($_.Length / 1KB))
}

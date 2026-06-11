#!/usr/bin/env node
// Generate the committed test-fixture corpus for the ADR-0028 decode matrix.
//
// ONE-SHOT TOOLING — this script depends on the v1 ffmpeg/ffprobe *sidecars*
// (populate with `npm run fetch:ffmpeg`). It exists so the corpus can be
// regenerated while the sidecars are still around; once issue 06 retires the
// sidecar machinery, the committed fixtures under src-tauri/tests/fixtures/
// are the artifact of record and this script is historical.
//
// One fixture per real-world (demuxer × video codec × audio codec) cell of the
// matrix, ~1 s, 64×64 (legal-size exceptions noted inline), silent-tone audio,
// a few KB each. Plus a rotated fixture (display-matrix side data) and an
// extensionless copy. Cells with no FFmpeg encoder (vc1, wmapro) live in the
// committed allowlisted-gaps.json instead — see that file.
//
// Naming is load-bearing: `<vcodec>-<acodec>.<container>` using ffprobe
// codec_name spellings, so the future manifest↔corpus drift test (issue 05)
// can parse coverage from filenames. Video-only fixtures omit the acodec.
//
// Usage:  node scripts/generate-fixtures.mjs   (or: npm run fixtures:generate)
// Idempotent: wipes previously generated fixtures and regenerates everything.

import { spawnSync } from 'node:child_process';
import {
  copyFileSync,
  existsSync,
  readdirSync,
  rmSync,
  statSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '..');
const binariesDir = join(repoRoot, 'src-tauri', 'binaries');
const fixturesDir = join(repoRoot, 'src-tauri', 'tests', 'fixtures');
const ffmpeg = join(binariesDir, 'ffmpeg-x86_64-pc-windows-msvc.exe');
const ffprobe = join(binariesDir, 'ffprobe-x86_64-pc-windows-msvc.exe');

const GAP_FILE = 'allowlisted-gaps.json';
// "Low hundreds of KB" total (issue 01 acceptance criterion).
const MAX_TOTAL_BYTES = 600 * 1024;

function fail(msg) {
  console.error(`\n✗ ${msg}\n`);
  process.exit(1);
}

// --- the fixture table -------------------------------------------------------
//
// Each entry: output name, muxer (-f), expected demuxer token in ffprobe's
// format_name, video encoder args, audio encoder args (null = video-only),
// source sizes/rates. Defaults: 64×64 video, rate 10, 44100 Hz mono tone.
//
// Encoder quirks, resolved here so a re-run stays mechanical:
// - h263 rejects 64×64 (legal sizes only) → 128×96; clock-locked framerate.
// - mpeg1/mpeg2 only accept standard framerates → rate 25.
// - prores/dnxhd are intra-only (every frame costs) → low framerates; dnxhr_lb
//   is the resolution-flexible DNxHR profile (classic dnxhd has a fixed table).
// - opus-in-mp4 is still flagged experimental by the mp4 muxer.
// - PCM is uncompressed → 8000 Hz mono to keep files small.

const PCM_RATE = '8000';

const fixtures = [
  // -- mov family (mp4 / m4v / m4a / 3gp) --
  { name: 'h264-aac.mp4', format: 'mp4', demux: 'mp4',
    v: ['-c:v', 'libx264', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'aac', '-b:a', '32k'] },
  { name: 'hevc-aac.mp4', format: 'mp4', demux: 'mp4',
    v: ['-c:v', 'libx265', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'aac', '-b:a', '32k'] },
  { name: 'av1-opus.mp4', format: 'mp4', demux: 'mp4', ar: '48000',
    v: ['-c:v', 'libsvtav1', '-preset', '12', '-crf', '50', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'libopus', '-b:a', '24k'],
    extra: ['-strict', 'experimental'] },
  { name: 'h264-alac.mp4', format: 'mp4', demux: 'mp4',
    v: ['-c:v', 'libx264', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'alac'] },
  { name: 'prores-pcm_s16le.mov', format: 'mov', demux: 'mov', rate: '5', ar: PCM_RATE,
    v: ['-c:v', 'prores', '-profile:v', '0', '-pix_fmt', 'yuv422p10le'],
    a: ['-c:a', 'pcm_s16le'] },
  { name: 'h264-pcm_s16be.mov', format: 'mov', demux: 'mov', ar: PCM_RATE,
    v: ['-c:v', 'libx264', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'pcm_s16be'] },
  { name: 'h264-pcm_s24le.mov', format: 'mov', demux: 'mov', ar: PCM_RATE,
    v: ['-c:v', 'libx264', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'pcm_s24le'] },
  { name: 'h264-pcm_s32le.mov', format: 'mov', demux: 'mov', ar: PCM_RATE,
    v: ['-c:v', 'libx264', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'pcm_s32le'] },
  { name: 'h264-pcm_f32le.mov', format: 'mov', demux: 'mov', ar: PCM_RATE,
    v: ['-c:v', 'libx264', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'pcm_f32le'] },
  { name: 'h263-aac.3gp', format: '3gp', demux: '3gp', size: '128x96', rate: '30000/1001',
    v: ['-c:v', 'h263', '-q:v', '31'],
    a: ['-c:a', 'aac', '-b:a', '32k'] },

  // -- matroska (mkv / webm) --
  { name: 'hevc-aac.mkv', format: 'matroska', demux: 'matroska',
    v: ['-c:v', 'libx265', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'aac', '-b:a', '32k'] },
  { name: 'theora-vorbis.mkv', format: 'matroska', demux: 'matroska',
    v: ['-c:v', 'libtheora', '-q:v', '3'],
    a: ['-c:a', 'libvorbis', '-q:a', '0'] },
  { name: 'h264-flac.mkv', format: 'matroska', demux: 'matroska',
    v: ['-c:v', 'libx264', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p'],
    a: ['-c:a', 'flac'] },
  { name: 'vp9-opus.webm', format: 'webm', demux: 'matroska', ar: '48000',
    v: ['-c:v', 'libvpx-vp9', '-crf', '50', '-b:v', '0'],
    a: ['-c:a', 'libopus', '-b:a', '24k'] },
  { name: 'vp8-vorbis.webm', format: 'webm', demux: 'matroska',
    v: ['-c:v', 'libvpx', '-crf', '50', '-b:v', '100k'],
    a: ['-c:a', 'libvorbis', '-q:a', '0'] },

  // -- avi --
  { name: 'mpeg4-mp3.avi', format: 'avi', demux: 'avi',
    v: ['-c:v', 'mpeg4', '-q:v', '31'],
    a: ['-c:a', 'libmp3lame', '-b:a', '32k'] },
  { name: 'msmpeg4v2-mp3.avi', format: 'avi', demux: 'avi',
    v: ['-c:v', 'msmpeg4v2', '-q:v', '31'],
    a: ['-c:a', 'libmp3lame', '-b:a', '32k'] },
  { name: 'mjpeg-pcm_s16le.avi', format: 'avi', demux: 'avi', ar: PCM_RATE,
    v: ['-c:v', 'mjpeg', '-q:v', '31', '-pix_fmt', 'yuvj420p'],
    a: ['-c:a', 'pcm_s16le'] },
  { name: 'mpeg4-pcm_u8.avi', format: 'avi', demux: 'avi', ar: PCM_RATE,
    v: ['-c:v', 'mpeg4', '-q:v', '31'],
    a: ['-c:a', 'pcm_u8'] },
  { name: 'mpeg4-pcm_alaw.avi', format: 'avi', demux: 'avi', ar: PCM_RATE,
    v: ['-c:v', 'mpeg4', '-q:v', '31'],
    a: ['-c:a', 'pcm_alaw'] },
  { name: 'mpeg4-pcm_mulaw.avi', format: 'avi', demux: 'avi', ar: PCM_RATE,
    v: ['-c:v', 'mpeg4', '-q:v', '31'],
    a: ['-c:a', 'pcm_mulaw'] },

  // -- asf (wmv) --
  { name: 'msmpeg4v3-wmav2.wmv', format: 'asf', demux: 'asf',
    v: ['-c:v', 'msmpeg4', '-q:v', '31'],
    a: ['-c:a', 'wmav2', '-b:a', '32k'] },
  { name: 'wmv1-wmav2.wmv', format: 'asf', demux: 'asf',
    v: ['-c:v', 'wmv1', '-q:v', '31'],
    a: ['-c:a', 'wmav2', '-b:a', '32k'] },
  { name: 'wmv2-wmav2.wmv', format: 'asf', demux: 'asf',
    v: ['-c:v', 'wmv2', '-q:v', '31'],
    a: ['-c:a', 'wmav2', '-b:a', '32k'] },

  // -- flv --
  { name: 'flv1-mp3.flv', format: 'flv', demux: 'flv',
    v: ['-c:v', 'flv', '-q:v', '31'],
    a: ['-c:a', 'libmp3lame', '-b:a', '32k'] },

  // -- mpegts / mpegps (vob) / mpegvideo (elementary stream) --
  { name: 'mpeg2video-mp2.ts', format: 'mpegts', demux: 'mpegts', rate: '25', ar: '48000',
    v: ['-c:v', 'mpeg2video', '-q:v', '31'],
    a: ['-c:a', 'mp2', '-b:a', '32k'] },
  { name: 'mpeg2video-eac3.ts', format: 'mpegts', demux: 'mpegts', rate: '25', ar: '48000',
    v: ['-c:v', 'mpeg2video', '-q:v', '31'],
    a: ['-c:a', 'eac3', '-b:a', '96k'] },
  { name: 'mpeg2video-ac3.vob', format: 'vob', demux: 'mpeg', rate: '25', ar: '48000',
    v: ['-c:v', 'mpeg2video', '-q:v', '31'],
    a: ['-c:a', 'ac3', '-b:a', '64k'] },
  { name: 'mpeg1video.m1v', format: 'mpeg1video', demux: 'mpegvideo', rate: '25',
    v: ['-c:v', 'mpeg1video', '-q:v', '31'],
    a: null, durationless: true },

  // -- mxf --
  // dnxhr_lb refuses anything under 256×120; its fixed per-frame cost is the
  // corpus's biggest, so keep the frame count minimal — the mxf muxer needs
  // -strict unofficial to accept a 5 fps rate (the demuxer reads it fine).
  { name: 'dnxhd.mxf', format: 'mxf', demux: 'mxf', rate: '5', size: '256x120',
    v: ['-c:v', 'dnxhd', '-profile:v', 'dnxhr_lb', '-pix_fmt', 'yuv422p'],
    a: null, extra: ['-strict', 'unofficial'] },
];

// Derived fixtures (not generated from lavfi sources).
const ROTATED_NAME = 'h264-aac-rotated.mp4';
// Non-square (96×64) on purpose: it lets the issue-05 transcode test prove
// autorotation by the output's *swapped* dimensions (a square clip couldn't).
const ROTATED_SIZE = '96x64';
const NOEXT_SOURCE = 'h264-aac.mp4';
const NOEXT_NAME = 'h264-aac-noext';

// --- helpers ------------------------------------------------------------------

function run(exe, args) {
  const res = spawnSync(exe, args, { encoding: 'utf8' });
  if (res.error) fail(`failed to spawn ${exe}: ${res.error.message}`);
  return res;
}

function generate(fix) {
  const out = join(fixturesDir, fix.name);
  const size = fix.size ?? '64x64';
  const rate = fix.rate ?? '10';
  const ar = fix.ar ?? '44100';

  const args = ['-hide_banner', '-loglevel', 'error', '-y'];
  args.push('-f', 'lavfi', '-i', `testsrc2=size=${size}:rate=${rate}:duration=1`);
  if (fix.a) {
    args.push('-f', 'lavfi', '-i', `sine=frequency=440:sample_rate=${ar}:duration=1`);
    args.push('-shortest');
  }
  args.push(...fix.v);
  args.push(...(fix.a ?? ['-an']));
  args.push(...(fix.extra ?? []));
  args.push('-f', fix.format, out);

  const res = run(ffmpeg, args);
  if (res.status !== 0) {
    fail(`generating ${fix.name} failed:\n${res.stderr}`);
  }
}

function probeJson(path) {
  const res = run(ffprobe, [
    '-v', 'quiet', '-print_format', 'json', '-show_format', '-show_streams', path,
  ]);
  if (res.status !== 0) fail(`ffprobe failed for ${path} (exit ${res.status})`);
  try {
    return JSON.parse(res.stdout);
  } catch (e) {
    fail(`ffprobe produced malformed JSON for ${path}: ${e.message}`);
  }
}

/// Mirror of probe.rs's acceptance plus filename↔content checks.
function verify(fix) {
  const path = join(fixturesDir, fix.name);
  const probed = probeJson(path);
  const streams = probed.streams ?? [];

  const video = streams.find((s) => s.codec_type === 'video');
  if (!video) fail(`${fix.name}: no video stream — v1 pre-flight would reject it`);

  // The filename's codec tokens must match what's actually inside.
  const expectedV = fix.name.split(/[-.]/)[0];
  if (video.codec_name !== expectedV) {
    fail(`${fix.name}: video codec is ${video.codec_name}, filename says ${expectedV}`);
  }
  const audio = streams.find((s) => s.codec_type === 'audio');
  if (fix.a) {
    const expectedA = fix.name.split('-')[1]?.split('.')[0];
    if (!audio) fail(`${fix.name}: expected an audio stream`);
    if (audio.codec_name !== expectedA) {
      fail(`${fix.name}: audio codec is ${audio.codec_name}, filename says ${expectedA}`);
    }
  } else if (audio) {
    fail(`${fix.name}: unexpected audio stream`);
  }

  // The intended demuxer must be the one that picks the file up.
  const formatName = probed.format?.format_name ?? '';
  if (!formatName.split(',').includes(fix.demux)) {
    fail(`${fix.name}: demuxed as "${formatName}", expected "${fix.demux}"`);
  }

  // probe.rs derives progress from format.duration; raw elementary streams
  // (mpegvideo) may legitimately lack it.
  const duration = parseFloat(probed.format?.duration ?? '');
  if (!fix.durationless && !(duration > 0)) {
    fail(`${fix.name}: missing/zero format.duration (${probed.format?.duration})`);
  }

  return statSync(path).size;
}

function verifyRotation(name) {
  const probed = probeJson(join(fixturesDir, name));
  const video = (probed.streams ?? []).find((s) => s.codec_type === 'video');
  const matrix = (video?.side_data_list ?? []).find(
    (sd) => sd.side_data_type === 'Display Matrix',
  );
  if (!matrix) {
    fail(`${name}: no Display Matrix side data — rotation must be metadata, not baked in`);
  }
  const rotation = Math.abs(Number(matrix.rotation));
  if (rotation !== 90) fail(`${name}: display-matrix rotation is ${matrix.rotation}, expected ±90`);
}

// --- main ----------------------------------------------------------------------

if (!existsSync(ffmpeg) || !existsSync(ffprobe)) {
  fail('v1 sidecars missing from src-tauri/binaries/ — run `npm run fetch:ffmpeg` first');
}
if (!existsSync(join(fixturesDir, GAP_FILE))) {
  fail(`${GAP_FILE} missing from ${fixturesDir} — it is committed, not generated; restore it`);
}

// Idempotent: wipe everything previously generated (the gap file is the only
// committed-by-hand resident), then regenerate.
for (const entry of readdirSync(fixturesDir)) {
  if (entry !== GAP_FILE) rmSync(join(fixturesDir, entry), { force: true });
}

console.log(`Generating ${fixtures.length} fixtures with the v1 sidecar ffmpeg…`);
for (const fix of fixtures) generate(fix);

// Rotated: encode a non-square h264+aac clip, then remux it with display-matrix
// side data only — the stream copy guarantees the rotation is metadata, never
// baked-in pixels, and the non-square frame makes the swap observable.
{
  const tmp = join(tmpdir(), 'ruckel-rotated-src.mp4');
  let res = run(ffmpeg, [
    '-hide_banner', '-loglevel', 'error', '-y',
    '-f', 'lavfi', '-i', `testsrc2=size=${ROTATED_SIZE}:rate=10:duration=1`,
    '-f', 'lavfi', '-i', 'sine=frequency=440:sample_rate=44100:duration=1',
    '-shortest',
    '-c:v', 'libx264', '-preset', 'veryfast', '-crf', '35', '-pix_fmt', 'yuv420p',
    '-c:a', 'aac', '-b:a', '32k',
    '-f', 'mp4', tmp,
  ]);
  if (res.status !== 0) fail(`generating rotated source failed:\n${res.stderr}`);
  res = run(ffmpeg, [
    '-hide_banner', '-loglevel', 'error', '-y',
    '-display_rotation', '90',
    '-i', tmp,
    '-c', 'copy', '-f', 'mp4', join(fixturesDir, ROTATED_NAME),
  ]);
  if (res.status !== 0) fail(`generating ${ROTATED_NAME} failed:\n${res.stderr}`);
  rmSync(tmp, { force: true });
}

// Extensionless: byte-identical copy, drives the ADR-0011 fallback path.
copyFileSync(join(fixturesDir, NOEXT_SOURCE), join(fixturesDir, NOEXT_NAME));

console.log('Verifying every fixture against the sidecar ffprobe…');
let total = 0;
const rows = [];
for (const fix of fixtures) {
  const bytes = verify(fix);
  total += bytes;
  rows.push([fix.name, bytes]);
}
for (const name of [ROTATED_NAME, NOEXT_NAME]) {
  const probed = probeJson(join(fixturesDir, name));
  if (!(probed.streams ?? []).some((s) => s.codec_type === 'video')) {
    fail(`${name}: no video stream`);
  }
  const bytes = statSync(join(fixturesDir, name)).size;
  total += bytes;
  rows.push([name, bytes]);
}
verifyRotation(ROTATED_NAME);

const width = Math.max(...rows.map(([n]) => n.length));
for (const [name, bytes] of rows) {
  console.log(`  ${name.padEnd(width)}  ${(bytes / 1024).toFixed(1).padStart(7)} KiB`);
}
console.log(`  ${'TOTAL'.padEnd(width)}  ${(total / 1024).toFixed(1).padStart(7)} KiB`);

if (total > MAX_TOTAL_BYTES) {
  fail(`corpus is ${(total / 1024).toFixed(0)} KiB — over the ${MAX_TOTAL_BYTES / 1024} KiB budget`);
}

console.log(`\n✓ ${rows.length} fixtures generated and verified in src-tauri/tests/fixtures/`);

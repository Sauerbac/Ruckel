#!/usr/bin/env node
// Fetch the pinned FFmpeg/ffprobe sidecars into src-tauri/binaries/.
//
// Reproducible path for the binaries that ADR-0005 / ADR-0022 keep out of git: read the
// committed manifest, download the pinned archive, verify every SHA-256, extract the two
// executables, and write them under Tauri's `<name>-<target-triple>.exe` sidecar convention.
// Any checksum mismatch is fatal — we never write an unverified binary.
//
// Usage:  node scripts/fetch-ffmpeg.mjs   (or: npm run fetch:ffmpeg)
//         --force   re-download/re-extract even if outputs already match the manifest

import { createHash } from 'node:crypto';
import { inflateRawSync } from 'node:zlib';
import {
  closeSync,
  existsSync,
  mkdirSync,
  openSync,
  readFileSync,
  readSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(scriptDir, '..');
const binariesDir = join(repoRoot, 'src-tauri', 'binaries');
const cacheDir = join(binariesDir, '.cache');
const manifestPath = join(binariesDir, 'ffmpeg-manifest.json');

const force = process.argv.includes('--force');

function fail(msg) {
  console.error(`\n✗ ${msg}\n`);
  process.exit(1);
}

function sha256File(path) {
  return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function fmtBytes(n) {
  return `${(n / 1024 / 1024).toFixed(1)} MiB`;
}

// --- minimal, dependency-free zip extraction --------------------------------
// Reads the central directory from the end of the file, then extracts only the
// entries we need via fd seeks — so the ~150 MB archive is never fully buffered.

function readChunk(fd, position, length) {
  const buf = Buffer.alloc(length);
  let read = 0;
  while (read < length) {
    const n = readSync(fd, buf, read, length - read, position + read);
    if (n === 0) break;
    read += n;
  }
  if (read !== length) throw new Error('unexpected end of file while reading zip');
  return buf;
}

function readCentralDirectory(fd, fileSize) {
  // Locate the End Of Central Directory record (sig 0x06054b50). It lives in the
  // last 22 bytes + an optional comment (<= 64 KiB), so scan a bounded tail window.
  const tailLen = Math.min(fileSize, 22 + 0xffff);
  const tail = readChunk(fd, fileSize - tailLen, tailLen);
  let eocd = -1;
  for (let i = tail.length - 22; i >= 0; i--) {
    if (tail.readUInt32LE(i) === 0x06054b50) {
      eocd = i;
      break;
    }
  }
  if (eocd === -1) throw new Error('zip End Of Central Directory record not found');

  const entryCount = tail.readUInt16LE(eocd + 10);
  const cdSize = tail.readUInt32LE(eocd + 12);
  const cdOffset = tail.readUInt32LE(eocd + 16);
  if (cdOffset === 0xffffffff || cdSize === 0xffffffff || entryCount === 0xffff) {
    throw new Error('zip64 archives are not supported by this fetcher');
  }

  const cd = readChunk(fd, cdOffset, cdSize);
  const entries = new Map();
  let p = 0;
  for (let i = 0; i < entryCount; i++) {
    if (cd.readUInt32LE(p) !== 0x02014b50) throw new Error('corrupt central directory header');
    const method = cd.readUInt16LE(p + 10);
    const compSize = cd.readUInt32LE(p + 20);
    const nameLen = cd.readUInt16LE(p + 28);
    const extraLen = cd.readUInt16LE(p + 30);
    const commentLen = cd.readUInt16LE(p + 32);
    const localOffset = cd.readUInt32LE(p + 42);
    const name = cd.toString('utf8', p + 46, p + 46 + nameLen);
    if (compSize === 0xffffffff || localOffset === 0xffffffff) {
      throw new Error('zip64 entry is not supported by this fetcher');
    }
    entries.set(name, { method, compSize, localOffset });
    p += 46 + nameLen + extraLen + commentLen;
  }
  return entries;
}

function extractEntry(fd, entry) {
  // The local header repeats the name/extra lengths (they can differ from the
  // central directory's), so read them fresh to find where the data starts.
  const local = readChunk(fd, entry.localOffset, 30);
  if (local.readUInt32LE(0) !== 0x04034b50) throw new Error('corrupt local file header');
  const nameLen = local.readUInt16LE(26);
  const extraLen = local.readUInt16LE(28);
  const dataOffset = entry.localOffset + 30 + nameLen + extraLen;
  const compressed = readChunk(fd, dataOffset, entry.compSize);
  if (entry.method === 0) return compressed; // stored
  if (entry.method === 8) return inflateRawSync(compressed); // deflate
  throw new Error(`unsupported zip compression method ${entry.method}`);
}

// --- main -------------------------------------------------------------------

async function main() {
  if (!existsSync(manifestPath)) fail(`manifest not found at ${manifestPath}`);
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));

  const outputs = manifest.binaries.map((b) => ({
    ...b,
    outputPath: join(binariesDir, b.outputName),
  }));

  // Idempotent: if every output already matches the manifest, do nothing.
  if (!force && outputs.every((o) => existsSync(o.outputPath) && sha256File(o.outputPath) === o.sha256)) {
    console.log('✓ Sidecars already present and verified — nothing to do.');
    for (const o of outputs) console.log(`  ${o.outputName}`);
    console.log('  (use --force to re-fetch)');
    return;
  }

  mkdirSync(cacheDir, { recursive: true });
  const archivePath = join(cacheDir, manifest.archive.asset);

  // Reuse a cached archive only if it already matches the pinned checksum.
  let haveArchive = false;
  if (existsSync(archivePath) && sha256File(archivePath) === manifest.archive.sha256) {
    console.log(`✓ Using cached archive ${manifest.archive.asset}`);
    haveArchive = true;
  }

  if (!haveArchive) {
    console.log(`↓ Downloading ${manifest.archive.asset} (${fmtBytes(manifest.archive.sizeBytes)})`);
    console.log(`  ${manifest.archive.url}`);
    const res = await fetch(manifest.archive.url, { redirect: 'follow' });
    if (!res.ok) fail(`download failed: HTTP ${res.status} ${res.statusText}`);
    const bytes = Buffer.from(await res.arrayBuffer());
    writeFileSync(archivePath, bytes);

    const got = createHash('sha256').update(bytes).digest('hex');
    if (got !== manifest.archive.sha256) {
      rmSync(archivePath, { force: true });
      fail(
        `archive SHA-256 mismatch — refusing to use it.\n` +
          `  expected ${manifest.archive.sha256}\n` +
          `  got      ${got}`,
      );
    }
    console.log('✓ Archive checksum verified');
  }

  // Extract + verify each binary, writing atomically (tmp then rename).
  const fd = openSync(archivePath, 'r');
  try {
    const fileSize = statSync(archivePath).size;
    const entries = readCentralDirectory(fd, fileSize);

    for (const o of outputs) {
      const entry = entries.get(o.archivePath);
      if (!entry) fail(`archive does not contain expected path: ${o.archivePath}`);

      const data = extractEntry(fd, entry);
      const got = createHash('sha256').update(data).digest('hex');
      if (got !== o.sha256) {
        fail(
          `${o.outputName} SHA-256 mismatch — refusing to write it.\n` +
            `  expected ${o.sha256}\n` +
            `  got      ${got}`,
        );
      }

      const tmp = `${o.outputPath}.tmp`;
      writeFileSync(tmp, data);
      renameSync(tmp, o.outputPath);
      console.log(`✓ ${o.outputName} (${fmtBytes(data.length)}) verified & written`);
    }
  } finally {
    closeSync(fd);
  }

  console.log(`\n✓ FFmpeg ${manifest.ffmpegVersion} sidecars ready in src-tauri/binaries/`);
}

main().catch((err) => fail(err.stack || String(err)));

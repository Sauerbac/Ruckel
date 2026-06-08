// Display formatters for the mono data values surfaced in the file list and
// status bar (ADR-0018: data is IBM Plex Mono). Pure, table-testable helpers —
// no contract types of their own; they format primitives pulled off the frozen
// S1 payloads (FileProbe duration/size, ProgressEvent percent, …).

/** Seconds → `M:SS`, or `H:MM:SS` once it crosses an hour. */
export function formatDuration(secs: number): string {
  const total = Math.max(0, Math.floor(secs))
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const s = total % 60
  const ss = String(s).padStart(2, '0')
  if (h > 0) return `${h}:${String(m).padStart(2, '0')}:${ss}`
  return `${m}:${ss}`
}

/** Byte count → a compact `248 MB` / `1.4 GB` label (binary units, decimal-ish). */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${Math.max(0, Math.round(bytes))} B`
  const units = ['KB', 'MB', 'GB', 'TB']
  let value = bytes / 1024
  let i = 0
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024
    i++
  }
  const rounded = value < 10 ? value.toFixed(1) : String(Math.round(value))
  return `${rounded} ${units[i]}`
}

/** Probe width×height → `1920×1080` (true multiplication sign, not an `x`). */
export function formatResolution(width: number, height: number): string {
  return `${width}×${height}`
}

/** Last path segment of a Windows or POSIX path. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path
}

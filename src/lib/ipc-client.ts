// Typed wrappers over the Tauri IPC surface (ADR-0016). Payload shapes and
// event names come from the generated S1 contract in `./ipc` — the Rust types
// are the single source of truth, so nothing here is hand-duplicated.

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { getCurrentWindow } from '@tauri-apps/api/window'

import {
  CONVERSION_EVENTS,
  type ConversionJob,
  type DoneEvent,
  type FileDoneEvent,
  type FileErrorEvent,
  type PreflightResult,
  type ProgressEvent,
} from './ipc'

/** True only inside the Tauri runtime (false in the browser-only gallery). */
export const inTauri = '__TAURI_INTERNALS__' in window

/** `preflight` — resolve dropped paths into a Conversion Plan + collisions. */
export function preflight(paths: string[]): Promise<PreflightResult> {
  return invoke<PreflightResult>('preflight', { paths })
}

/** `start_conversion` — run the plan; progress arrives via events. */
export function startConversion(plan: ConversionJob[]): Promise<void> {
  return invoke('start_conversion', { plan })
}

/** `cancel_conversion` — request cancellation of the in-flight batch. */
export function cancelConversion(): Promise<void> {
  return invoke('cancel_conversion')
}

/** Subscribers for the conversion event stream (ADR-0016). */
export interface ConversionHandlers {
  onProgress?: (e: ProgressEvent) => void
  onFileDone?: (e: FileDoneEvent) => void
  onFileError?: (e: FileErrorEvent) => void
  onDone?: (e: DoneEvent) => void
  onCancelled?: () => void
}

/** Wire every conversion event to its handler; resolves to one unlisten fn. */
export async function listenConversion(
  handlers: ConversionHandlers,
): Promise<UnlistenFn> {
  const unlisteners = await Promise.all([
    listen<ProgressEvent>(CONVERSION_EVENTS.PROGRESS, (e) =>
      handlers.onProgress?.(e.payload),
    ),
    listen<FileDoneEvent>(CONVERSION_EVENTS.FILE_DONE, (e) =>
      handlers.onFileDone?.(e.payload),
    ),
    listen<FileErrorEvent>(CONVERSION_EVENTS.FILE_ERROR, (e) =>
      handlers.onFileError?.(e.payload),
    ),
    listen<DoneEvent>(CONVERSION_EVENTS.DONE, (e) =>
      handlers.onDone?.(e.payload),
    ),
    listen(CONVERSION_EVENTS.CANCELLED, () => handlers.onCancelled?.()),
  ])
  return () => unlisteners.forEach((un) => un())
}

/** OS file-drop onto the window (ADR-0020). Real paths arrive only via Tauri's
 * drag-drop events, never HTML5 drop. `onEnterOrLeave` toggles the drop-target
 * highlight; `onDrop` carries the dropped paths. No-op outside Tauri. */
export async function listenFileDrop(handlers: {
  onActive?: (active: boolean) => void
  onDrop?: (paths: string[]) => void
}): Promise<UnlistenFn> {
  if (!inTauri) return () => {}
  return getCurrentWindow().onDragDropEvent((event) => {
    const p = event.payload
    if (p.type === 'enter' || p.type === 'over') handlers.onActive?.(true)
    else if (p.type === 'leave') handlers.onActive?.(false)
    else if (p.type === 'drop') {
      handlers.onActive?.(false)
      handlers.onDrop?.(p.paths)
    }
  })
}

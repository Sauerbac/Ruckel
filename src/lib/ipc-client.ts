// Typed wrappers over the Tauri IPC surface (ADR-0016). Payload shapes and
// event names come from the generated S1 contract in `./ipc` — the Rust types
// are the single source of truth, so nothing here is hand-duplicated.

import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
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

/** Video container extensions offered by the browse dialog — mirrors
 * `scanner.rs` `VIDEO_EXTENSIONS` (ADR-0011) so click-to-browse and drag-drop
 * filter the same set. */
const VIDEO_EXTENSIONS = [
  'mp4',
  'm4v',
  'mov',
  'mkv',
  'webm',
  'avi',
  'wmv',
  'flv',
  'f4v',
  'mpg',
  'mpeg',
  'm2v',
  'mts',
  'm2ts',
  'ts',
  '3gp',
  '3g2',
  'ogv',
  'vob',
  'asf',
  'divx',
  'dv',
  'mxf',
]

/** Click-to-browse intake (ADR-0023). Opens a multi-select Open dialog filtered
 * to the video extensions (plus All files for extensionless videos) and returns
 * the chosen paths, which flow through the same pre-flight path as a drop. `[]`
 * when cancelled or outside Tauri. Folders stay drag-only (native multi-select
 * file dialogs can't also pick directories). */
export async function openVideoDialog(): Promise<string[]> {
  if (!inTauri) return []
  const selected = await open({
    multiple: true,
    directory: false,
    filters: [
      { name: 'Video', extensions: VIDEO_EXTENSIONS },
      { name: 'All files', extensions: ['*'] },
    ],
  })
  if (selected == null) return []
  return Array.isArray(selected) ? selected : [selected]
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

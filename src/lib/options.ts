// The option vocabulary (ADR-0007) expressed against the frozen S1 contract
// enums — the single place that maps each `Resolution`/`Framerate`/`Audio`
// variant to its raw display label, and the three presets to their concrete
// `ConversionOptions`. Components import these so segment lists and preset snaps
// never re-spell the contract values by hand.

import type { Audio, ConversionOptions, Framerate, Resolution } from './ipc'

/** A selectable segment: the raw label the user sees + its contract value. */
export type Segment<T> = { label: string; value: T }

export const RESOLUTION_OPTIONS: Segment<Resolution>[] = [
  { label: '480', value: 'P480' },
  { label: '720', value: 'P720' },
  { label: '1080', value: 'P1080' },
  { label: 'ORIG', value: 'Original' },
]

/** CRF is the raw 18–28 value (ADR-0007 surfaces three of the range). Ordered
 *  low→high quality left→right (28 = lowest, 18 = best) to match the other
 *  groups, where the left segment is always the smallest/lowest (ADR-0023). */
export const CRF_OPTIONS: Segment<number>[] = [
  { label: '28', value: 28 },
  { label: '23', value: 23 },
  { label: '18', value: 18 },
]

export const FRAMERATE_OPTIONS: Segment<Framerate>[] = [
  { label: '24', value: 'Fps24' },
  { label: '30', value: 'Fps30' },
  { label: '60', value: 'Fps60' },
  { label: 'ORIG', value: 'Original' },
]

export const AUDIO_OPTIONS: Segment<Audio>[] = [
  { label: '96K', value: 'Kbps96' },
  { label: '128K', value: 'Kbps128' },
  { label: '192K', value: 'Kbps192' },
  { label: 'NONE', value: 'None' },
]

/** The three one-tap presets (ADR-0007). Each snaps all four knobs at once. */
export type PresetName = 'PRESENTATION' | 'HIGH' | 'COMPACT'

export const PRESET_ORDER: PresetName[] = ['PRESENTATION', 'HIGH', 'COMPACT']

export const PRESET_LABELS: Record<PresetName, string> = {
  PRESENTATION: 'PRESENTATION',
  HIGH: 'HIGH',
  COMPACT: 'COMPACT',
}

export const PRESETS: Record<PresetName, ConversionOptions> = {
  PRESENTATION: {
    resolution: 'Original',
    crf: 23,
    framerate: 'Original',
    audio: 'Kbps128',
  },
  HIGH: {
    resolution: 'Original',
    crf: 18,
    framerate: 'Original',
    audio: 'Kbps192',
  },
  COMPACT: { resolution: 'P720', crf: 28, framerate: 'Fps30', audio: 'Kbps96' },
}

/** The preset whose four values exactly match `o`, or `null` for "Custom". */
export function presetFor(o: ConversionOptions): PresetName | null {
  for (const name of PRESET_ORDER) {
    const p = PRESETS[name]
    if (
      p.resolution === o.resolution &&
      p.crf === o.crf &&
      p.framerate === o.framerate &&
      p.audio === o.audio
    ) {
      return name
    }
  }
  return null
}

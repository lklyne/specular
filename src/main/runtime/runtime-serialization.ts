import { VIEWPORT_PRESETS } from '../../shared/constants'
import type { Annotation, WorkspacePageSnapshot, WorkspaceSnapshot } from '../../shared/types'

const FALLBACK_PRESET_INDEX = 3
const LEGACY_PRESET_INDEX_MAP: Record<number, number> = {
  0: 0,
  1: 1,
  2: 1,
  3: 2,
  4: 2,
  5: 2,
  6: 3,
  7: 3,
  8: 4,
  9: 3,
  10: 4,
  11: 6,
}

type PageLabelTarget = Pick<WorkspacePageSnapshot, 'presetIndex' | 'name' | 'title' | 'url'>

function hostnameLabel(url: string | undefined): string | null {
  if (!url) return null
  try {
    const parsed = new URL(url)
    return parsed.hostname.replace(/^www\./, '') || null
  } catch {
    return null
  }
}

export function normalizePresetIndex(presetIndex: number): number {
  if (Number.isInteger(presetIndex) && presetIndex >= 0 && presetIndex < VIEWPORT_PRESETS.length) {
    return presetIndex
  }
  if (Number.isInteger(presetIndex) && presetIndex in LEGACY_PRESET_INDEX_MAP) {
    return LEGACY_PRESET_INDEX_MAP[presetIndex] ?? FALLBACK_PRESET_INDEX
  }
  return Math.min(FALLBACK_PRESET_INDEX, VIEWPORT_PRESETS.length - 1)
}

export function viewportPresetForIndex(presetIndex: number) {
  return VIEWPORT_PRESETS[normalizePresetIndex(presetIndex)]
}

export function pageDisplayLabel(page: PageLabelTarget): string {
  const trimmedTitle = page.title?.trim()
  if (trimmedTitle) return trimmedTitle
  const trimmedName = page.name?.trim()
  if (trimmedName) return trimmedName
  const hostLabel = hostnameLabel(page.url)
  if (hostLabel) return hostLabel
  return viewportPresetForIndex(page.presetIndex)?.label ?? 'Page'
}

export function cloneAnnotationsForPersistence(
  annotations: Annotation[],
): Annotation[] {
  return structuredClone(annotations)
}

export function cloneWorkspaceSnapshot(snapshot: WorkspaceSnapshot): WorkspaceSnapshot {
  return structuredClone(snapshot)
}

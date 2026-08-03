import type { DevtoolsPanelData } from '../../shared/types'

export { isUnresolved } from '../../shared/annotation-utils'

export const INITIAL_PANEL_DATA: DevtoolsPanelData = {
  activeTab: 'comments',
  panelMode: { kind: 'document' },
  activeTool: { kind: 'select' },
  annotateEnabled: false,
  annotateAvailable: false,
}

export function valuePreview(value: unknown): string {
  if (typeof value === 'string') return value
  if (
    typeof value === 'number' ||
    typeof value === 'boolean' ||
    value === null ||
    value === undefined
  ) {
    return String(value)
  }
  try {
    return JSON.stringify(value)
  } catch {
    return String(value)
  }
}

export function formatSourceLocation(
  sourceLocation: { file: string; line?: number; column?: number } | undefined,
): string | null {
  if (!sourceLocation?.file) return null
  const parts = [sourceLocation.file]
  if (typeof sourceLocation.line === 'number') {
    parts.push(String(sourceLocation.line))
    if (typeof sourceLocation.column === 'number') {
      parts.push(String(sourceLocation.column))
    }
  }
  return parts.join(':')
}

export function parseEditedValue(raw: string, original: unknown): unknown {
  if (typeof original === 'boolean') return raw.trim().toLowerCase() === 'true'
  if (typeof original === 'number') {
    const parsed = Number(raw)
    return Number.isFinite(parsed) ? parsed : original
  }
  if (original === null) return raw.trim().toLowerCase() === 'null' ? null : raw
  if (typeof original === 'object') {
    try {
      return JSON.parse(raw) as unknown
    } catch {
      return raw
    }
  }
  return raw
}

export function formatCommentTime(value: string): string {
  const parsed = Date.parse(value)
  if (!Number.isFinite(parsed)) return value
  return new Date(parsed).toLocaleString([], {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  })
}

export function authorLabel(author: 'user' | 'agent'): string {
  return author === 'agent' ? 'Agent' : 'You'
}

export const mutedClass = 'text-[var(--surface-foreground-muted)]'

export function dividerClass(isDark: boolean): string {
  return isDark ? 'border-zinc-700/50' : 'border-zinc-200'
}

export function paneActionBtnClass(isDark: boolean): string {
  return isDark
    ? 'rounded p-0.5 text-[var(--surface-foreground-muted)] hover:bg-zinc-700 hover:text-[var(--surface-foreground)]'
    : 'rounded p-0.5 text-[var(--surface-foreground-muted)] hover:bg-zinc-200 hover:text-[var(--surface-foreground)]'
}

export function paneDeleteBtnClass(isDark: boolean): string {
  return isDark
    ? 'rounded p-0.5 text-[var(--surface-foreground-muted)] hover:bg-red-500/12 hover:text-red-400'
    : 'rounded p-0.5 text-[var(--surface-foreground-muted)] hover:bg-red-50 hover:text-red-600'
}

export function fileEntityLabel(path: string): string {
  return path.split('/').pop() ?? path
}

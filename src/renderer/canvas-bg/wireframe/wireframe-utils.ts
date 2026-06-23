import type { WireframeSizing } from '../../../shared/wireframe/wireframe-types'

// Presentation helpers for the wireframe renderer. The pure tree ops live in
// src/shared/wireframe/wireframe-ops.ts so the main process can reuse them, and
// node classification (e.g. nodeHasEditableText) lives in wireframe-selection.ts.

// --- Sizing helpers ---

export function sizingToFlex(size: WireframeSizing | undefined): React.CSSProperties {
  if (size === 'fill') return { flex: 1, minWidth: 0, minHeight: 0 }
  if (size === 'hug' || size === undefined) return {}
  return {}
}

export function sizingToWidth(size: WireframeSizing | undefined): string | number | undefined {
  if (size === 'fill') return undefined // handled by flex
  if (size === 'hug' || size === undefined) return undefined
  return size
}

export function sizingToHeight(size: WireframeSizing | undefined): string | number | undefined {
  if (size === 'fill') return undefined
  if (size === 'hug' || size === undefined) return undefined
  return size
}

export function parsePadding(
  padding: number | [number, number] | [number, number, number, number] | undefined,
): string {
  if (padding === undefined) return '0'
  if (typeof padding === 'number') return `${padding}px`
  if (padding.length === 2) return `${padding[0]}px ${padding[1]}px`
  return `${padding[0]}px ${padding[1]}px ${padding[2]}px ${padding[3]}px`
}

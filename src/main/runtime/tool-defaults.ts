/**
 * Tool defaults runtime — mediator between the persisted defaults in
 * `preferences.ts` and the rest of the app (ADR 0008 §9).
 *
 * Creation paths (e.g. `register-canvas-entity-ipc.ts` for `add-text`,
 * `add-shape`; `useAnnotationDrawingGestures` for draw) read these via
 * `getToolDefault*` helpers when stamping new entities. The tool-mode popup
 * writes patches through `applyToolDefaultPatch`, which persists and triggers
 * a layout broadcast so the renderer's swatch state and `layoutRef` (read by
 * the draw gesture at stroke-start) both pick up the new value.
 *
 * Per ADR: not in Y.Doc, not in `.canvas`, not in undo/redo — user
 * preferences only.
 */

import {
  getToolDefaults as readToolDefaults,
  saveToolDefaults,
} from './preferences'
import { markDirty } from './layout-dirty'
import { broadcastToolChange } from './runtime-slice-broadcast'
import { requestLayout } from './viewport-control'
import type { ToolDefaults, ToolDefaultPatch } from '../../shared/tool-defaults'
import type { TextFont } from '../../shared/text-fonts'

export function getToolDefaults(): ToolDefaults {
  return readToolDefaults()
}

export function getStickyDefaultColor(): string {
  return readToolDefaults()['add-sticky'].color
}

export function getPlainTextDefaultColor(): string | null {
  return readToolDefaults()['add-text'].color
}

export function getTextDefaultSize(): number {
  return readToolDefaults()['add-text'].textSize
}

export function getStickyDefaultSize(): number {
  return readToolDefaults()['add-sticky'].textSize
}

export function getTextDefaultFont(): TextFont {
  return readToolDefaults()['add-text'].textFont
}

export function getStickyDefaultFont(): TextFont {
  return readToolDefaults()['add-sticky'].textFont
}

export function getShapeDefaults(): ToolDefaults['add-shape'] {
  return readToolDefaults()['add-shape']
}

export function getDrawDefaults(): ToolDefaults['draw'] {
  return readToolDefaults().draw
}

/**
 * Apply a single typed patch. Persists to disk and marks the canvas surface
 * dirty so the renderer (which carries tool-defaults in its layout broadcast)
 * sees the new value on the next layout pass. `'floating-ui'` would be the
 * natural channel, but it's been retired in layout-engine — `'canvas'` is the
 * only flag that actually broadcasts `layout-update` to bg + above views.
 *
 * `'toolbar'` is marked too so the Draw button's glyph tracks `draw.brushType`
 * (the toolbar only receives `toolbar-selection-changed`, not `layout-update`).
 */
export function applyToolDefaultPatch(patch: ToolDefaultPatch): void {
  const current = readToolDefaults()
  if (scopeSlice(current, patch.scope)[patch.key] === patch.value) return
  const next: ToolDefaults = {
    'add-text': { ...current['add-text'] },
    'add-sticky': { ...current['add-sticky'] },
    'add-shape': { ...current['add-shape'] },
    draw: { ...current.draw },
  }
  scopeSlice(next, patch.scope)[patch.key] = patch.value
  saveToolDefaults(next)
  // The defaults live in the `tool` slice, and the placement preview is built
  // from them; the toolbar reads them off the pass.
  broadcastToolChange()
  markDirty('toolbar')
  requestLayout()
}

/**
 * A scope's slice, addressable by patch key. `ToolDefaultPatch` pairs each
 * scope with only the keys that scope actually holds, so `[scope][key]` names
 * a real slot by construction — TypeScript checks the two indices
 * independently and can't see that correlation, which is what the cast stands
 * in for. A new key on an existing scope needs no change here.
 */
function scopeSlice(
  defaults: ToolDefaults,
  scope: ToolDefaultPatch['scope'],
): Record<string, ToolDefaultPatch['value']> {
  return defaults[scope] as Record<string, ToolDefaultPatch['value']>
}

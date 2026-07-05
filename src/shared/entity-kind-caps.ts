/**
 * Renderer-side per-kind interaction capabilities (ADR 0024 §7).
 *
 * The exhaustive `Record<CanvasEntityKind, ...>` makes adding a kind a compile
 * error here — a forced decision, not a silent default. Consumed by the shared
 * hit-tester (`hit-test.ts`) and the pointer router's resize config; layer
 * rules forbid the renderer from reaching into the main-process registry, so
 * these live in `src/shared/`.
 */

import type { AspectRatioResizeMode } from './resize-accumulator'
import type { CanvasEntityKind } from './types'

export interface EntityKindCaps {
  /**
   * A chrome strip renders above the entity (page URL bar, file header).
   * Text/shape have inline editors when selected, not chrome.
   */
  hasChrome: boolean
  /**
   * Edge anchors render on selection/hover for connecting edges. Drawings opt
   * out — the dots crowd the selection chrome and make a selected stroke
   * awkward to grab and drag.
   */
  hasAnchors: boolean
  /** Smallest resize footprint in canvas units. */
  minSize: { width: number; height: number }
  /** Aspect-lock behavior during resize. File overrides this per-file at runtime. */
  aspectMode: AspectRatioResizeMode
}

export const ENTITY_KIND_CAPS: Record<CanvasEntityKind, EntityKindCaps> = {
  page: {
    hasChrome: true,
    hasAnchors: true,
    minSize: { width: 320, height: 200 },
    aspectMode: 'off',
  },
  text: {
    hasChrome: false,
    hasAnchors: true,
    minSize: { width: 100, height: 60 },
    aspectMode: 'off',
  },
  file: {
    hasChrome: true,
    hasAnchors: true,
    minSize: { width: 80, height: 80 },
    aspectMode: 'off',
  },
  group: {
    hasChrome: false,
    hasAnchors: true,
    minSize: { width: 120, height: 80 },
    aspectMode: 'off',
  },
  drawing: {
    hasChrome: false,
    hasAnchors: false,
    minSize: { width: 16, height: 16 },
    aspectMode: 'off',
  },
  shape: {
    hasChrome: false,
    hasAnchors: true,
    minSize: { width: 24, height: 24 },
    aspectMode: 'shift-locks',
  },
  // Edges are connective tissue, not canvas items (ADR 0024 §3): no chrome,
  // anchors, or resize footprint of their own. They never appear in the
  // CanvasSceneEntity lists these caps are read against, so this row exists
  // only to keep the Record exhaustive.
  edge: {
    hasChrome: false,
    hasAnchors: false,
    minSize: { width: 0, height: 0 },
    aspectMode: 'off',
  },
}

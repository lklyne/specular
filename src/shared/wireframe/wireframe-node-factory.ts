// Default-node factory for the insert palette (plan 3.2).
//
// Pure and deterministic-by-input: the node id comes from the injected `genId`,
// so the same call order reproduces the same tree (no Date.now()/Math.random()).
// Lives in src/shared so both the panel palette and the agent CLI (3.4) build
// new nodes the same way — the "one op set for humans and agents" thesis.

import type { WireframeNode } from './wireframe-types'

/**
 * Node kinds the insert palette exposes. `page` is the palette's user-facing
 * name for a top-level frame (the panel labels frames "Page"); both map to an
 * empty vertical frame.
 */
export type WireframePaletteType =
  | 'page'
  | 'frame'
  | 'text'
  | 'button'
  | 'input'
  | 'dropdown'
  | 'checkbox'
  | 'toggle'
  | 'image'
  | 'divider'
  | 'spacer'

/**
 * Build a fresh node of the given palette type with sensible placeholder content,
 * taking its id from `genId`. Frames start empty; leaf nodes carry a default label
 * so the inserted node is visible and immediately editable on the canvas.
 */
export function createWireframeNode(
  type: WireframePaletteType,
  genId: () => string,
): WireframeNode {
  const id = genId()
  switch (type) {
    case 'page':
    case 'frame':
      return { id, type: 'frame', direction: 'vertical', children: [] }
    case 'text':
      return { id, type: 'text', text: 'Text' }
    case 'button':
      return { id, type: 'button', text: 'Button' }
    case 'input':
      return { id, type: 'input', placeholder: 'Input' }
    case 'dropdown':
      return { id, type: 'dropdown', placeholder: 'Select', options: [] }
    case 'checkbox':
      return { id, type: 'checkbox', label: 'Checkbox', checked: false }
    case 'toggle':
      return { id, type: 'toggle', label: 'Toggle', on: false }
    case 'image':
      return { id, type: 'image' }
    case 'divider':
      return { id, type: 'divider' }
    case 'spacer':
      return { id, type: 'spacer' }
  }
}

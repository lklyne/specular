// Per-node property editors (plan 3.3).
//
// The right-details panel renders editors for the *selected* wireframe node, and
// every editor change maps to a `setProps` patch applied through the shared
// `updateNodeProps` op (one undoable Y.Doc op). This module is the pure controller
// in between: it describes which editors a node type exposes, reads the current
// display value off a node, and coerces a raw editor input into a typed patch.
//
// It lives in src/shared/ (no Electron, no React, deterministic-by-input) so the
// mapping "editor change → op patch" is unit-testable without spinning the app,
// and the panel stays a thin view over it. Every editor key is a member of the
// node type's LEGAL_PROPS set in wireframe-ops.ts — the patch the panel sends back
// is therefore always accepted by `updateNodeProps`.

export type WireframePropControl = 'select' | 'number' | 'sizing' | 'text' | 'options'

export interface WireframePropEditor {
  /** Patch key — must be legal for the node type (see LEGAL_PROPS). */
  key: string
  label: string
  control: WireframePropControl
  /** Choices for a `select` control. */
  options?: ReadonlyArray<{ value: string; label: string }>
}

/** A node-shaped record: at least `id`/`type`, plus the node's own props inline. */
export type WireframeNodeShape = { id: string; type: string } & Record<string, unknown>

const SIZING_MODES = ['fill', 'hug'] as const

const EDITORS_BY_TYPE: Record<string, ReadonlyArray<WireframePropEditor>> = {
  frame: [
    {
      key: 'direction',
      label: 'Direction',
      control: 'select',
      options: [
        { value: 'vertical', label: 'Vertical' },
        { value: 'horizontal', label: 'Horizontal' },
      ],
    },
    { key: 'gap', label: 'Gap', control: 'number' },
    { key: 'padding', label: 'Padding', control: 'number' },
    { key: 'width', label: 'Width', control: 'sizing' },
    { key: 'height', label: 'Height', control: 'sizing' },
  ],
  text: [
    {
      key: 'level',
      label: 'Level',
      control: 'select',
      options: [
        { value: 'h1', label: 'H1' },
        { value: 'h2', label: 'H2' },
        { value: 'h3', label: 'H3' },
        { value: 'body', label: 'Body' },
        { value: 'caption', label: 'Caption' },
      ],
    },
  ],
  button: [
    {
      key: 'variant',
      label: 'Variant',
      control: 'select',
      options: [
        { value: 'primary', label: 'Primary' },
        { value: 'secondary', label: 'Secondary' },
        { value: 'ghost', label: 'Ghost' },
      ],
    },
  ],
  input: [{ key: 'label', label: 'Label', control: 'text' }],
  dropdown: [
    { key: 'label', label: 'Label', control: 'text' },
    { key: 'options', label: 'Options', control: 'options' },
  ],
  checkbox: [{ key: 'label', label: 'Label', control: 'text' }],
  toggle: [{ key: 'label', label: 'Label', control: 'text' }],
  image: [
    { key: 'width', label: 'Width', control: 'number' },
    { key: 'height', label: 'Height', control: 'number' },
    { key: 'alt', label: 'Alt', control: 'text' },
  ],
  divider: [],
  spacer: [],
}

/** The editors a node of `type` exposes (empty for divider/spacer/unknown). */
export function editorsForNodeType(type: string): ReadonlyArray<WireframePropEditor> {
  return EDITORS_BY_TYPE[type] ?? []
}

/**
 * The current display string for `editor` read off `node` — what the control
 * shows. Empty string when the prop is unset.
 */
export function editorDisplayValue(node: WireframeNodeShape, editor: WireframePropEditor): string {
  const raw = node[editor.key]
  if (raw == null) return ''
  if (editor.control === 'options') {
    return Array.isArray(raw) ? raw.join(', ') : ''
  }
  return String(raw)
}

/**
 * Coerce a raw editor input into the typed patch value for `editor.key`, or
 * `null` when the input is empty/invalid (a no-op the panel should skip). This is
 * the "editor change → op patch" mapping the panel sends as a `setProps` op.
 *
 * - `select` — the chosen string (assumed one of `editor.options`).
 * - `text` — the string verbatim.
 * - `number` — parsed finite number; invalid ⇒ null.
 * - `sizing` — `'fill'`/`'hug'` verbatim, else a parsed finite number; else null.
 * - `options` — comma-separated list, trimmed, empties dropped.
 */
export function patchForEditorChange(
  editor: WireframePropEditor,
  raw: string,
): Record<string, unknown> | null {
  switch (editor.control) {
    case 'select':
    case 'text':
      return { [editor.key]: raw }
    case 'number': {
      const n = Number(raw)
      if (raw.trim() === '' || !Number.isFinite(n)) return null
      return { [editor.key]: n }
    }
    case 'sizing': {
      if ((SIZING_MODES as readonly string[]).includes(raw)) return { [editor.key]: raw }
      const n = Number(raw)
      if (raw.trim() === '' || !Number.isFinite(n)) return null
      return { [editor.key]: n }
    }
    case 'options': {
      const options = raw
        .split(',')
        .map((s) => s.trim())
        .filter((s) => s.length > 0)
      return { [editor.key]: options }
    }
  }
}

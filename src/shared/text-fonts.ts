/**
 * Text fonts — the three typefaces a text entity can render in.
 *
 * Entities store the semantic token (`'mono'`), never the family string. Two
 * reasons: `.canvas` files stay readable and portable for other JSON Canvas
 * tools, and the face a token resolves to can change without migrating every
 * canvas on disk. Same rule color follows — storage holds a preset, the
 * renderer resolves it.
 *
 * The token is absent on every entity that never left the default, so legacy
 * canvases need no migration and new files stay clean.
 */

export type TextFont = 'sans' | 'mono' | 'hand'

/** Entities without a `textFont` render in this one. */
export const TEXT_FONT_DEFAULT: TextFont = 'sans'

/**
 * Resolved CSS stacks. The fallbacks matter: `font-display: swap` means the
 * first paint after a swap uses them, and a bundled face can fail to load.
 */
export const TEXT_FONT_STACKS: Record<TextFont, string> = {
  sans: 'system-ui, sans-serif',
  mono: '"Geist Mono", ui-monospace, SFMono-Regular, Menlo, monospace',
  hand: 'Kalam, "Bradley Hand", cursive',
}

/** Dropdown order and labels. */
export const TEXT_FONT_OPTIONS: readonly { value: TextFont; label: string }[] = [
  { value: 'sans', label: 'Sans' },
  { value: 'mono', label: 'Mono' },
  { value: 'hand', label: 'Hand' },
]

export function isTextFont(value: unknown): value is TextFont {
  return value === 'sans' || value === 'mono' || value === 'hand'
}

/** Normalize any stored/serialized value to a renderable token. */
function textFontOrDefault(value: unknown): TextFont {
  return isTextFont(value) ? value : TEXT_FONT_DEFAULT
}

export function fontStackFor(value: unknown): string {
  return TEXT_FONT_STACKS[textFontOrDefault(value)]
}

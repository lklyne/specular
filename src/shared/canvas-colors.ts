/**
 * JSON Canvas color presets and resolution.
 *
 * Three-axis model (ADR 0013 §1):
 *
 *  - **Slot**: one of eight named choices the popups expose
 *    (`neutral` · `purple` · `blue` · `cyan` · `green` · `yellow` · `orange` · `red`).
 *  - **Palette**: `'soft'` (muted pastels — stickies, shapes, highlighter brush)
 *    or `'vivid'` (saturated — plain text, edges, pen brush, groups). The same
 *    slot resolves to a different hue depending on the surface it paints.
 *
 *    The two resolve to different *kinds* of value, and the reason is what
 *    each one's consumers do with it. Vivid resolves to a CSS custom property
 *    (`var(--canvas-ink-*)`, defined per theme in `surfaceTheme.css`) — every
 *    vivid consumer hands the value straight to CSS or to an SVG paint
 *    attribute, both of which resolve `var()`, so the light/dark swap belongs
 *    in the stylesheet. Soft resolves to a literal hex, because its consumers
 *    (`lightenHex`, `darkenHex`, `withAlpha` for shape fills, sticky cards and
 *    the highlighter) derive new colors from the channels and need real
 *    numbers. Adding channel math to a vivid consumer means giving that call
 *    site a hex some other way — the `var()` will not survive it.
 *  - **Role**: how the color is used at render time — `'fill'` (sticky/shape
 *    backgrounds) or `'ink'` (pen strokes, plain text glyphs). Role only
 *    affects the theme-aware `neutral` slot.
 *
 * **Storage.** A hue is stored as its JSON Canvas preset number — `"1"`–`"6"`
 * for the six spec hues (red/orange/yellow/green/cyan/purple) and `"7"` for
 * blue (a Specular extension — the spec only numbers six). Neutral is stored
 * as the `'neutral'` sentinel (`specular.colorRole` on disk). The stored value
 * carries the *slot*, not a hue — the palette is chosen by the surface at
 * render time, so the same `"2"` reads muted on a sticky and punchy on a pen.
 *
 * A literal `#RRGGBB` hex is also accepted and passes through unchanged — used
 * for custom colors and for canvases saved before the two-palette split.
 */

export type CanvasColorRole = 'fill' | 'ink'

/** Muted pastels vs. saturated hues — see the module doc. */
export type CanvasPalette = 'soft' | 'vivid'

export type CanvasColorSlot =
  | 'neutral'
  | 'purple'
  | 'blue'
  | 'cyan'
  | 'green'
  | 'yellow'
  | 'orange'
  | 'red'

export interface CanvasColorSlotInfo {
  id: CanvasColorSlot
  label: string
  /**
   * JSON Canvas preset number stored in `entity.color`. `"1"`–`"6"` are the
   * spec hues; `"7"` is the Specular-added blue; `null` for the theme-aware
   * neutral (stored as the `'neutral'` sentinel instead).
   */
  preset: string | null
  /** Muted pastel hex (stickies, shapes, highlighter). `null` for neutral. */
  soft: string | null
  /** Saturated hex (plain text, edges, pen, groups). `null` for neutral. */
  vivid: string | null
}

/** One slot resolved for a single palette — the shape the popups iterate over. */
export interface ResolvedColorSlot {
  id: CanvasColorSlot
  label: string
  /** Display hex for hue slots; `null` for the theme-aware neutral. */
  hex: string | null
  /** Value written to `entity.color` when the user picks this slot. */
  storage: string
}

/** Storage sentinel for the theme/role-aware neutral. */
export const NEUTRAL_STORAGE = 'neutral'

const NEUTRAL_FILL_LIGHT = '#fdf8f5'
// Slightly muted cream for dark mode — still light enough for dark ink to read,
// but a touch less glaring on a dark canvas.
const NEUTRAL_FILL_DARK = '#dcd2c4'

/**
 * Eight-slot palette in canonical popup order (ADR 0013 §1).
 *
 * `preset` is the on-disk number; `soft`/`vivid` are the two render hues.
 * `soft` matches the original muted pastels; `vivid` is the punchier set used
 * wherever contrast matters. `neutral` is theme/role-resolved at render time.
 */
export const CANVAS_COLOR_SLOTS: ReadonlyArray<CanvasColorSlotInfo> = [
  { id: 'neutral', label: 'Neutral', preset: null, soft: null, vivid: null },
  { id: 'purple', label: 'Purple', preset: '6', soft: '#c8b8d8', vivid: '#BD4BE5' },
  { id: 'blue', label: 'Blue', preset: '7', soft: '#b0c4d8', vivid: '#1084FF' },
  { id: 'cyan', label: 'Cyan', preset: '5', soft: '#b0d0d8', vivid: '#00CBFF' },
  { id: 'green', label: 'Green', preset: '4', soft: '#b8d8c8', vivid: '#00CA48' },
  { id: 'yellow', label: 'Yellow', preset: '3', soft: '#FFE18E', vivid: '#FFD500' },
  { id: 'orange', label: 'Orange', preset: '2', soft: '#e8ccb0', vivid: '#FF8E00' },
  { id: 'red', label: 'Red', preset: '1', soft: '#e8b4b8', vivid: '#FF1016' },
] as const

/** Preset number (`"1"`–`"7"`) → slot. */
const SLOT_BY_PRESET: Record<string, CanvasColorSlotInfo> = Object.fromEntries(
  CANVAS_COLOR_SLOTS.filter((s) => s.preset !== null).map((s) => [s.preset!, s]),
)

/** Every known hue hex (both `soft` and `vivid`) → slot. */
const SLOT_BY_HEX: Record<string, CanvasColorSlotInfo> = Object.fromEntries(
  CANVAS_COLOR_SLOTS.flatMap((s) =>
    [s.soft, s.vivid]
      .filter((hex): hex is string => hex !== null)
      .map((hex) => [hex.toLowerCase(), s]),
  ),
)

/**
 * The CSS custom property carrying a slot's vivid hue. Themed in
 * `surfaceTheme.css`; the `vivid` hex on the slot stays the dark-mode value so
 * `slotForStorage` can still match canvases that stored a literal hex.
 */
function inkVarFor(slot: CanvasColorSlot): string {
  return `var(--canvas-ink-${slot})`
}

function paletteValueFor(slot: CanvasColorSlotInfo, palette: CanvasPalette): string {
  if (palette === 'vivid') return inkVarFor(slot.id)
  return slot.soft ?? NEUTRAL_FILL_LIGHT
}

/**
 * The eight slots resolved for one palette, in canonical popup order. Popups
 * iterate this: `hex` is the swatch color, `storage` is the preset a pick
 * writes into `entity.color`.
 */
export function paletteSlots(
  palette: CanvasPalette,
): ReadonlyArray<ResolvedColorSlot> {
  return CANVAS_COLOR_SLOTS.map((slot) => ({
    id: slot.id,
    label: slot.label,
    // A vivid swatch shows the themed variable, not the raw hue, so the
    // picker previews the color the canvas will actually paint.
    hex:
      slot.preset === null
        ? null
        : palette === 'vivid'
          ? inkVarFor(slot.id)
          : slot.soft,
    storage: slot.preset ?? NEUTRAL_STORAGE,
  }))
}

/**
 * Resolve a stored canvas color value to a CSS color string.
 *
 * Accepts the `'neutral'` sentinel, a preset number (`"1"`–`"7"`), or a
 * literal `#RRGGBB` hex. A preset resolves to its slot's hue in `opts.palette`
 * — the caller passes the palette of the *surface* being painted, so the same
 * preset reads muted on a sticky and punchy on a pen. For neutral, pass
 * `opts.role` and `opts.isDark`. Hex values pass through unchanged.
 *
 * The result is a CSS color *value*, not necessarily a hex: the vivid palette
 * and neutral ink return `var(--canvas-ink-*)` (see the module doc). Assign it
 * to a CSS property or an SVG paint attribute; don't parse it.
 */
export function resolveCanvasColor(
  color: string,
  opts?: { role?: CanvasColorRole; isDark?: boolean; palette?: CanvasPalette },
): string {
  if (color === NEUTRAL_STORAGE) {
    return resolveNeutral(opts?.role ?? 'fill', opts?.isDark ?? false)
  }
  const slot = SLOT_BY_PRESET[color]
  if (slot) return paletteValueFor(slot, opts?.palette ?? 'soft')
  return color
}

function resolveNeutral(role: CanvasColorRole, isDark: boolean): string {
  // Ink is themed in CSS like the hues. Fill stays a hex: it is the sticky
  // card and shape body, whose renderers derive from its channels.
  if (role === 'ink') return inkVarFor('neutral')
  return isDark ? NEUTRAL_FILL_DARK : NEUTRAL_FILL_LIGHT
}

/**
 * The slot that a stored color matches, or `null` when it doesn't line up
 * with any palette slot. Recognizes the `'neutral'` sentinel, preset numbers,
 * and both the `soft` and `vivid` hex of every hue — so a picker highlights
 * the active swatch whether the value is a preset or a pre-split literal hex.
 */
export function slotForStorage(color: string | null | undefined): CanvasColorSlot | null {
  if (!color) return null
  if (color === NEUTRAL_STORAGE) return 'neutral'
  if (SLOT_BY_PRESET[color]) return SLOT_BY_PRESET[color].id
  if (color.startsWith('#')) {
    return SLOT_BY_HEX[color.toLowerCase()]?.id ?? null
  }
  return null
}

/**
 * Palette a drawing brush paints in. Highlighter uses muted pastels; pen uses
 * saturated hues. ADR 0013 §1.
 */
export function paletteForBrushType(brushType: 'pen' | 'highlight'): CanvasPalette {
  return brushType === 'highlight' ? 'soft' : 'vivid'
}

/**
 * Palette a text style paints in. Sticky bodies are fills in the muted palette;
 * plain text glyphs are ink in the saturated palette. ADR 0013 §1.
 */
export function paletteForTextStyle(style: 'plain' | 'sticky'): CanvasPalette {
  return style === 'sticky' ? 'soft' : 'vivid'
}

/** Apply an alpha to a #RRGGBB hex color; passes other forms through. */
export function withAlpha(color: string, alpha: number): string {
  if (color.startsWith('#') && color.length === 7) {
    const r = parseInt(color.slice(1, 3), 16)
    const g = parseInt(color.slice(3, 5), 16)
    const b = parseInt(color.slice(5, 7), 16)
    return `rgba(${r}, ${g}, ${b}, ${alpha})`
  }
  return color
}

/** Lighten a #RRGGBB hex by interpolating toward white. amount=0 leaves it; amount=1 returns white. */
export function lightenHex(color: string, amount: number): string {
  if (!color.startsWith('#') || color.length !== 7) return color
  const r = parseInt(color.slice(1, 3), 16)
  const g = parseInt(color.slice(3, 5), 16)
  const b = parseInt(color.slice(5, 7), 16)
  const lr = Math.round(r + (255 - r) * amount)
  const lg = Math.round(g + (255 - g) * amount)
  const lb = Math.round(b + (255 - b) * amount)
  const hex = (n: number) => n.toString(16).padStart(2, '0')
  return `#${hex(lr)}${hex(lg)}${hex(lb)}`
}

/** Darken a #RRGGBB hex by interpolating toward black. amount=0 leaves it; amount=1 returns black. */
export function darkenHex(color: string, amount: number): string {
  if (!color.startsWith('#') || color.length !== 7) return color
  const r = parseInt(color.slice(1, 3), 16)
  const g = parseInt(color.slice(3, 5), 16)
  const b = parseInt(color.slice(5, 7), 16)
  const dr = Math.round(r * (1 - amount))
  const dg = Math.round(g * (1 - amount))
  const db = Math.round(b * (1 - amount))
  const hex = (n: number) => n.toString(16).padStart(2, '0')
  return `#${hex(dr)}${hex(dg)}${hex(db)}`
}

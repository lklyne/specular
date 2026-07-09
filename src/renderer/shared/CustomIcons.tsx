import { useId, type ComponentProps } from 'react'
import { darkenHex, lightenHex, NEUTRAL_STORAGE, resolveCanvasColor } from '../../shared/canvas-colors'

// Custom-drawn icons exported from the agent-canvas Figma file
// (file key hgwwoe0EzUrErdviULmRtb).
//
// Two flavors:
//   - Inline JSX (PenSlimIcon / PenMarkerIcon / StrokeThinIcon /
//     StrokeThickIcon / DrawPenToolIcon / DrawHighlightToolIcon): the pen
//     glyphs take an `ink` prop so the cap/tip can preview the active draw
//     color. The two toolbar Draw glyphs additionally take `isDark` and
//     recolor their body gradient + stroke inline — see CenterActions.
//   - Toolbar icons (Select / Hand / AddSticky / AddShape / AddPage /
//     AddText / Comment / Inspect / Sun / Moon / SunMoon / ZoomChevron):
//     rendered from raw
//     SVG assets in ./icons/toolbar/*.svg (light) and ./icons/toolbar/dark/
//     (dark variants generated via color-substitution: light gradient stops
//     remap to #65625D → #484744, stroke flips to #E2DEDB). Caller passes
//     `isDark` to pick the right asset. Shadows are still reapplied via CSS
//     `filter: drop-shadow(...)` on the `<img>` so each glyph gets its own
//     alpha-correct shadow instead of a bounding-box shadow.
//
// Re-extracting from Figma: see docs/adr/0013-popup-menus-v2.md §Icons for
// the per-node id table and an mcp__plugin_figma_figma__use_figma recipe.

import addDocumentUrl from './icons/toolbar/add-document.svg'
import addPageUrl from './icons/toolbar/add-page.svg'
import addTextUrl from './icons/toolbar/add-text.svg'
import commentUrl from './icons/toolbar/comment.svg'
import handUrl from './icons/toolbar/hand.svg'
import inspectUrl from './icons/toolbar/inspect.svg'
import moonUrl from './icons/toolbar/moon.svg'
import selectUrl from './icons/toolbar/select.svg'
import sunUrl from './icons/toolbar/sun.svg'
import sunMoonUrl from './icons/toolbar/sun-moon.svg'
import zoomChevronUrl from './icons/toolbar/zoom-chevron.svg'
import addDocumentDarkUrl from './icons/toolbar/dark/add-document.svg'
import addPageDarkUrl from './icons/toolbar/dark/add-page.svg'
import addTextDarkUrl from './icons/toolbar/dark/add-text.svg'
import commentDarkUrl from './icons/toolbar/dark/comment.svg'
import handDarkUrl from './icons/toolbar/dark/hand.svg'
import inspectDarkUrl from './icons/toolbar/dark/inspect.svg'
import moonDarkUrl from './icons/toolbar/dark/moon.svg'
import selectDarkUrl from './icons/toolbar/dark/select.svg'
import sunDarkUrl from './icons/toolbar/dark/sun.svg'
import sunMoonDarkUrl from './icons/toolbar/dark/sun-moon.svg'
import zoomChevronDarkUrl from './icons/toolbar/dark/zoom-chevron.svg'

// ── Toolbar icons ──────────────────────────────────────────────────────────

type ToolbarIconProps = {
  size?: number
  isDark?: boolean
  className?: string
  style?: React.CSSProperties
}

function makeToolbarIcon(lightUrl: string, darkUrl: string, name: string) {
  // Render each glyph inside a fixed `size` × `size` box with object-fit:
  // contain so 18×18 (comment, inspect) and 12×12 (zoom chevron) SVGs sit
  // on the same nominal grid as the 20×20 ones at their natural Figma scale.
  const Icon = ({ size = 20, isDark = false, className, style }: ToolbarIconProps) => (
    <img
      src={isDark ? darkUrl : lightUrl}
      alt=""
      aria-hidden
      draggable={false}
      width={size}
      height={size}
      className={className}
      style={{ objectFit: 'contain', ...style }}
    />
  )
  Icon.displayName = name
  return Icon
}

// Default size matches the 20×20 Figma toolbar slot.
export const SelectToolIcon = makeToolbarIcon(selectUrl, selectDarkUrl, 'SelectToolIcon')
export const HandToolIcon = makeToolbarIcon(handUrl, handDarkUrl, 'HandToolIcon')

// Placeholder laser-pointer glyph — a bright dot with four radiating beams,
// drawn in `currentColor` so it inherits the toolbar button's hover/active
// text color. Swap for a proper Figma asset when the tool graduates.
export const LaserToolIcon = ({
  size = 20,
  style,
  className,
}: ToolbarIconProps) => (
  <svg
    width={size}
    height={size}
    viewBox="0 0 20 20"
    fill="none"
    xmlns="http://www.w3.org/2000/svg"
    className={className}
    style={style}
    aria-hidden
  >
    <circle cx="10" cy="10" r="2.5" fill="currentColor" />
    <g stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
      <path d="M10 1.5v3" />
      <path d="M10 15.5v3" />
      <path d="M1.5 10h3" />
      <path d="M15.5 10h3" />
    </g>
  </svg>
)
LaserToolIcon.displayName = 'LaserToolIcon'
// ── Draw toolbar glyphs (inline JSX — `ink` tracks the active draw color) ───
//
// The toolbar Draw button swaps glyph by `draw.brushType`: the broad marker
// for `highlight`, the slim pen for `pen` (see CenterActions). Unlike the
// file-based toolbar icons these are inline so `ink` (the resolved draw
// color) tints the cap/tip; `isDark` recolors the body gradient + stroke.
// Extracted from Figma nodes 178:150 (pen) and 410:16 (highlight).

type DrawToolIconProps = {
  size?: number
  isDark?: boolean
  ink?: string
  style?: React.CSSProperties
}

// Red preset — the first-launch draw default. Callers always pass `ink`.
const DEFAULT_DRAW_INK = '#e8b4b8'

// ── Shared pen-glyph gradient defs ──────────────────────────────────────────
//
// All four pen-family glyphs (toolbar Draw pen/highlight, popup slim/marker)
// share the same mask/body/shine/seam gradient stops and isDark palette; only
// the gradient geometry differs per glyph. Ids are namespaced by `prefix`
// (which embeds a useId value) so several icon instances on one page never
// collide.

/** The gray ramp behind a pen glyph's gradients, keyed by theme. */
function penGlyphPalette(isDark: boolean) {
  return {
    maskColor: isDark ? '#484744' : '#D0CDCB',
    bodyTop: isDark ? '#65625D' : '#F0F0F0',
    bodyBottom: isDark ? '#65625D' : '#F8F8F8',
    shineTop: isDark ? '#484744' : '#D8D8D8',
    shineRest: isDark ? '#484744' : '#DBDBDB',
    seamTop: isDark ? '#484744' : '#D9D9D9',
    seamBottom: isDark ? '#E2DEDB' : '#B5B5B5',
  }
}

type PenGradientCoords = { x1: string; y1: string; x2: string; y2: string }

/**
 * The four gradients every pen glyph references as `${prefix}-mask-grad`,
 * `${prefix}-body-grad`, `${prefix}-shine-grad`, and `${prefix}-seam-grad`.
 * Rendered inside the glyph's `<defs>`.
 */
function PenIconDefs({
  prefix,
  isDark = false,
  mask,
  body,
  shine,
  seam,
}: {
  prefix: string
  isDark?: boolean
  mask: PenGradientCoords
  body: PenGradientCoords
  shine: PenGradientCoords
  seam: PenGradientCoords
}) {
  const palette = penGlyphPalette(isDark)
  return (
    <>
      <linearGradient id={`${prefix}-mask-grad`} {...mask} gradientUnits="userSpaceOnUse">
        <stop stopColor={palette.maskColor} stopOpacity="0" />
        <stop offset="0.278846" stopColor={palette.maskColor} />
      </linearGradient>
      <linearGradient id={`${prefix}-body-grad`} {...body} gradientUnits="userSpaceOnUse">
        <stop stopColor={palette.bodyTop} />
        <stop offset="1" stopColor={palette.bodyBottom} />
      </linearGradient>
      <linearGradient id={`${prefix}-shine-grad`} {...shine} gradientUnits="userSpaceOnUse">
        <stop stopColor={palette.shineTop} />
        <stop offset="0.540027" stopColor={palette.shineRest} stopOpacity="0.2" />
        <stop offset="0.985392" stopColor={palette.shineRest} stopOpacity="0.1" />
      </linearGradient>
      <linearGradient id={`${prefix}-seam-grad`} {...seam} gradientUnits="userSpaceOnUse">
        <stop stopColor={palette.seamTop} stopOpacity="0.33" />
        <stop offset="1" stopColor={palette.seamBottom} />
      </linearGradient>
    </>
  )
}

export function DrawPenToolIcon({
  size = 20,
  isDark = false,
  ink = DEFAULT_DRAW_INK,
  style,
}: DrawToolIconProps) {
  const prefix = `draw-pen-${useId()}`
  const clipId = `${prefix}-clip`
  const maskId = `${prefix}-mask`
  const capFilterId = `${prefix}-cap`
  const bodyFilterId = `${prefix}-body`
  const maskGradId = `${prefix}-mask-grad`
  const bodyGradId = `${prefix}-body-grad`
  const shineGradId = `${prefix}-shine-grad`
  const seamGradId = `${prefix}-seam-grad`
  const stroke = isDark ? '#E2DEDB' : '#18181B'
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={style}
    >
      <g clipPath={`url(#${clipId})`}>
        <mask
          id={maskId}
          style={{ maskType: 'alpha' }}
          maskUnits="userSpaceOnUse"
          x="3"
          y="0"
          width="15"
          height="21"
        >
          <rect
            width="15"
            height="21"
            transform="matrix(1 0 0 -1 3 21)"
            fill={`url(#${maskGradId})`}
          />
        </mask>
        <g mask={`url(#${maskId})`}>
          <g filter={`url(#${capFilterId})`}>
            <path
              d="M12.4365 5.50696C12.4785 5.62646 12.5 5.75221 12.5 5.87888C12.5 6.49806 11.9981 7 11.3789 7L9.62112 7C9.00194 7 8.5 6.49806 8.5 5.87888C8.5 5.75221 8.52147 5.62646 8.56349 5.50696L9.98471 1.46537C10.1559 0.97867 10.8441 0.97867 11.0153 1.46537L12.4365 5.50696Z"
              fill={ink}
            />
          </g>
          <path
            d="M13 5.87891C13 6.7742 12.2742 7.49998 11.3789 7.5L9.62109 7.5C8.72579 7.49998 8.00002 6.7742 8 5.87891C8 5.69575 8.03104 5.51361 8.0918 5.34082L9.5127 1.2998C9.8405 0.367597 11.1595 0.367598 11.4873 1.2998L12.9082 5.34082C12.969 5.51361 13 5.69575 13 5.87891Z"
            stroke={stroke}
          />
          <g filter={`url(#${bodyFilterId})`}>
            <path
              d="M9.42307 6.01468C10.1431 5.99459 10.8056 5.99524 11.5261 6.01592C12.6811 6.04908 13.6929 6.78409 14.1292 7.85397L16.7779 14.3492C16.9246 14.7089 17 15.0936 17 15.482L17 53C17 54.6569 15.6568 56 14 56L7 56C5.34314 56 4 54.6569 4 53L4.00001 15.473C4.00001 15.0904 4.07319 14.7114 4.21561 14.3563L6.81875 7.86586C7.25047 6.78945 8.26376 6.04703 9.42307 6.01468Z"
              fill={`url(#${bodyGradId})`}
            />
          </g>
          <path
            d="M16.5 15.4824L16.5 53C16.5 54.3807 15.3807 55.5 14 55.5L6.99999 55.5C5.61928 55.5 4.49999 54.3807 4.49999 53L4.5 15.4727C4.50004 15.154 4.56105 14.8378 4.67969 14.542L7.2832 8.05176C7.64485 7.15042 8.48799 6.54114 9.4375 6.51465C10.1477 6.49484 10.8006 6.49521 11.5117 6.51562C12.4589 6.54282 13.3007 7.14717 13.666 8.04297L16.3145 14.5381C16.4367 14.8378 16.5 15.1587 16.5 15.4824Z"
            stroke={stroke}
          />
          <path
            d="M13.1012 8.37423L16 15C14.5723 13.9005 12.4905 14.4509 11.7927 16.1123L11 18L11 7C11.9112 7 12.736 7.53942 13.1012 8.37423Z"
            fill={`url(#${shineGradId})`}
          />
          <rect x="10" y="7" width="1" height="12" fill={`url(#${seamGradId})`} />
        </g>
      </g>
      <defs>
        <filter
          id={capFilterId}
          x="6.5"
          y="0.100342"
          width="7"
          height="7.89966"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset dx="-4" />
          <feGaussianBlur stdDeviation="0.5" />
          <feComposite in2="hardAlpha" operator="arithmetic" k2="-1" k3="1" />
          <feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.1 0" />
          <feBlend mode="normal" in2="shape" result="effect1_innerShadow" />
        </filter>
        <filter
          id={bodyFilterId}
          x="4"
          y="6"
          width="13"
          height="50"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset />
          <feGaussianBlur stdDeviation="1" />
          <feComposite in2="hardAlpha" operator="arithmetic" k2="-1" k3="1" />
          <feColorMatrix type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 1 0" />
          <feBlend mode="normal" in2="shape" result="effect1_innerShadow" />
        </filter>
        <PenIconDefs
          prefix={prefix}
          isDark={isDark}
          mask={{ x1: '7.5', y1: '0', x2: '7.5', y2: '21' }}
          body={{ x1: '10.8545', y1: '35.0134', x2: '9.57424', y2: '35.0134' }}
          shine={{ x1: '12.8501', y1: '7.10886', x2: '14.7895', y2: '15.577' }}
          seam={{ x1: '10.5', y1: '7', x2: '10.5', y2: '19' }}
        />
        <clipPath id={clipId}>
          <rect width="20" height="20" fill="white" />
        </clipPath>
      </defs>
    </svg>
  )
}

export function DrawHighlightToolIcon({
  size = 20,
  isDark = false,
  ink = DEFAULT_DRAW_INK,
  style,
}: DrawToolIconProps) {
  const prefix = `draw-hl-${useId()}`
  const clipId = `${prefix}-clip`
  const maskId = `${prefix}-mask`
  const maskGradId = `${prefix}-mask-grad`
  const bodyGradId = `${prefix}-body-grad`
  const shineGradId = `${prefix}-shine-grad`
  const seamGradId = `${prefix}-seam-grad`
  const stroke = isDark ? '#E2DEDB' : '#352C24'
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      style={style}
    >
      <g clipPath={`url(#${clipId})`}>
        <mask
          id={maskId}
          style={{ maskType: 'alpha' }}
          maskUnits="userSpaceOnUse"
          x="3"
          y="0"
          width="15"
          height="21"
        >
          <rect
            width="15"
            height="21"
            transform="matrix(1 0 0 -1 3 21)"
            fill={`url(#${maskGradId})`}
          />
        </mask>
        <g mask={`url(#${maskId})`}>
          <path
            d="M13 5.6875C13 6.68852 12.1885 7.5 11.1875 7.5L9.8125 7.5C8.87414 7.5 8.10271 6.78688 8.00977 5.87305L8 5.6875L8 4.1875C8.00021 3.85369 8.22128 3.56048 8.54199 3.46777L12.042 2.45605C12.5214 2.31759 12.9997 2.67683 13 3.17578L13 5.6875Z"
            fill={ink}
            stroke={stroke}
          />
          <path
            d="M16.5 15.4824L16.5 53C16.5 54.3807 15.3807 55.5 14 55.5L6.99999 55.5C5.61928 55.5 4.49999 54.3807 4.49999 53L4.5 15.4727C4.50004 15.154 4.56105 14.8378 4.67969 14.542L5.56152 12.3437C6.01697 11.2081 6.35229 10.0278 6.56152 8.82227L6.6123 8.52637C6.81111 7.38097 7.76663 6.53617 8.90625 6.51465C9.93481 6.49523 11.0175 6.49561 12.0459 6.51562C13.1821 6.53774 14.1363 7.37604 14.3418 8.51562L14.3994 8.83398C14.6157 10.0339 14.9566 11.208 15.417 12.3369L16.3145 14.5381C16.4367 14.8378 16.5 15.1587 16.5 15.4824Z"
            fill={`url(#${bodyGradId})`}
            stroke={stroke}
          />
          <path
            d="M11 18L11 7C11.8697 7 12.6161 7.61946 12.7764 8.47431L12.8714 8.98061C13.121 10.3121 13.6465 11.5767 14.414 12.6931L16 15C14.5723 13.9005 12.4905 14.4509 11.7927 16.1123L11 18Z"
            fill={`url(#${shineGradId})`}
          />
          <rect
            x="16"
            y="16"
            width="1"
            height="11"
            transform="rotate(90 16 16)"
            fill={`url(#${seamGradId})`}
          />
          <rect x="5" y="17" width="11" height="4" fill={ink} />
        </g>
      </g>
      <defs>
        <PenIconDefs
          prefix={prefix}
          isDark={isDark}
          mask={{ x1: '7.5', y1: '0', x2: '7.5', y2: '21' }}
          body={{ x1: '10.8545', y1: '35.0134', x2: '9.57424', y2: '35.0134' }}
          shine={{ x1: '12.8501', y1: '7.10886', x2: '14.7895', y2: '15.577' }}
          seam={{ x1: '16.5', y1: '16', x2: '16.5', y2: '27' }}
        />
        <clipPath id={clipId}>
          <rect width="20" height="20" fill="white" />
        </clipPath>
      </defs>
    </svg>
  )
}
// ── AddShapeToolIcon (inline JSX — `color` tints the square + circle fills to
// match the currently selected shape color; gradient stops derived via
// lighten/darken like AddStickyToolIcon so the highlight/shadow reads across
// hues. Geometry ported from icons/toolbar/add-shape.svg; the morph arrow and
// strokes follow the theme like the other glyphs.) ─────────────────────────

type AddShapeIconProps = {
  size?: number
  isDark?: boolean
  /** Raw stored shape-color value (slot sentinel, preset, or hex). */
  color?: string
  style?: React.CSSProperties
  className?: string
}

export function AddShapeToolIcon({
  size = 20,
  isDark = false,
  color = NEUTRAL_STORAGE,
  style,
  className,
}: AddShapeIconProps) {
  const tint = resolveCanvasColor(color, { role: 'fill', isDark, palette: 'soft' })
  const isNeutral = color === NEUTRAL_STORAGE
  const uid = useId()
  const paint0 = `add-shape-paint0-${uid}`
  const paint1 = `add-shape-paint1-${uid}`
  const fillTop = isDark
    ? darkenHex(tint, isNeutral ? 0.55 : 0.4)
    : lightenHex(tint, 0.45)
  const fillBottom = isDark
    ? darkenHex(tint, isNeutral ? 0.7 : 0.55)
    : lightenHex(tint, 0.15)
  const stroke = isDark ? '#E2DEDB' : '#45403C'

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      style={style}
    >
      <rect x="0.5" y="1.5" width="10" height="10" rx="1.5" fill={`url(#${paint0})`} stroke={stroke} />
      <rect x="6.5" y="8.5" width="9" height="9" rx="4.5" fill={`url(#${paint1})`} stroke={stroke} />
      <path
        d="M13 1.5C12.7239 1.5 12.5 1.72386 12.5 2C12.5 2.27614 12.7239 2.5 13 2.5L13 2L13 1.5ZM17 8L16.6464 8.35355L17 8.70711L17.3536 8.35355L17 8ZM19.3536 6.35355L19.7071 6L19 5.29289L18.6464 5.64645L19 6L19.3536 6.35355ZM15.3536 5.64645L15 5.29289L14.2929 6L14.6464 6.35355L15 6L15.3536 5.64645ZM13 2L13 2.5L15 2.5L15 2L15 1.5L13 1.5L13 2ZM17 4L16.5 4L16.5 8L17 8L17.5 8L17.5 4L17 4ZM17 8L17.3536 8.35355L19.3536 6.35355L19 6L18.6464 5.64645L16.6464 7.64645L17 8ZM17 8L17.3536 7.64645L15.3536 5.64645L15 6L14.6464 6.35355L16.6464 8.35355L17 8ZM15 2L15 2.5C15.8284 2.5 16.5 3.17157 16.5 4L17 4L17.5 4C17.5 2.61929 16.3807 1.5 15 1.5L15 2Z"
        fill={stroke}
      />
      <defs>
        <linearGradient id={paint0} x1="5.5" y1="1" x2="5.5" y2="12" gradientUnits="userSpaceOnUse">
          <stop stopColor={fillTop} />
          <stop offset="1" stopColor={fillBottom} />
        </linearGradient>
        <linearGradient id={paint1} x1="11" y1="8" x2="11" y2="18" gradientUnits="userSpaceOnUse">
          <stop stopColor={fillTop} />
          <stop offset="1" stopColor={fillBottom} />
        </linearGradient>
      </defs>
    </svg>
  )
}

// ── AddStickyToolIcon (inline JSX — `tint` colors the paper to match the
// currently selected sticky color; gradient stops derived via lighten/darken
// so the paper-highlight + paper-shadow visual is preserved across hues.) ───
//
// Geometry ported from icons/toolbar/add-sticky.svg. Light mode keeps the
// paper pale via lighten(tint, 0.45) → lighten(tint, 0.15). Dark mode is
// split: neutral darkens hard (0.55 → 0.70, matching the original raster);
// hues darken to 0.40 → 0.55 so the fill sits below the #E2DEDB stroke in
// value and the outline reads clearly. Less darkening washed the stroke out.

type AddStickyIconProps = {
  size?: number
  isDark?: boolean
  /** Raw stored sticky-color value (slot sentinel, preset, or hex). */
  color?: string
  style?: React.CSSProperties
  className?: string
}

export function AddStickyToolIcon({
  size = 20,
  isDark = false,
  color = NEUTRAL_STORAGE,
  style,
  className,
}: AddStickyIconProps) {
  const tint = resolveCanvasColor(color, { role: 'fill', isDark, palette: 'soft' })
  const isNeutral = color === NEUTRAL_STORAGE
  const uid = useId()
  const filter0 = `add-sticky-filter0-${uid}`
  const filter1 = `add-sticky-filter1-${uid}`
  const paint0 = `add-sticky-paint0-${uid}`
  const paint1 = `add-sticky-paint1-${uid}`
  const paint2 = `add-sticky-paint2-${uid}`
  const clip = `add-sticky-clip-${uid}`

  const paperTop = isDark
    ? darkenHex(tint, isNeutral ? 0.55 : 0.4)
    : lightenHex(tint, 0.45)
  const paperBottom = isDark
    ? darkenHex(tint, isNeutral ? 0.7 : 0.55)
    : lightenHex(tint, 0.15)
  const stroke = isDark ? '#E2DEDB' : '#45403C'
  // Drop-shadow color matrix differs by theme: light uses 32% gray, dark uses
  // 40% black — preserved from the original SVG assets so the shadow stays
  // alpha-correct against either toolbar surface.
  const shadowMatrix = isDark
    ? '0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0.4 0'
    : '0 0 0 0 0.785325 0 0 0 0 0.785325 0 0 0 0 0.785325 0 0 0 0.32 0'
  const innerShadowAlpha = isDark ? 0.3 : 0.2

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 20 20"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
      style={style}
    >
      <g clipPath={`url(#${clip})`}>
        <g filter={`url(#${filter0})`}>
          <rect
            width="15"
            height="15"
            rx="3"
            transform="matrix(0.998628 0.0523679 -0.0416989 0.99913 4.40137 3.61328)"
            fill={`url(#${paint0})`}
          />
          <rect
            x="0.478464"
            y="0.525749"
            width="14"
            height="14"
            rx="2.5"
            transform="matrix(0.998628 0.0523679 -0.0416989 0.99913 4.42395 3.58868)"
            stroke={stroke}
          />
        </g>
        <g filter={`url(#${filter1})`}>
          <path
            d="M1.00098 3V14C1.00098 15.1046 1.89641 16 3.00098 16H14.001C15.1055 16 16.001 15.1046 16.001 14V10.875C16.001 5 12.001 1 8.40098 1H3.00098C1.89641 1 1.00098 1.89543 1.00098 3Z"
            fill={`url(#${paint1})`}
          />
          <path
            d="M3.00098 1.5H8.40137C11.645 1.5003 15.501 5.18877 15.501 10.875V14C15.501 14.8284 14.8294 15.5 14.001 15.5H3.00098C2.17255 15.5 1.50098 14.8284 1.50098 14V3C1.50098 2.17157 2.17255 1.5 3.00098 1.5Z"
            stroke={stroke}
          />
        </g>
        <path
          d="M8.40137 1.5C8.51643 1.5 8.6648 1.53695 8.83652 1.60729C10.9037 2.45409 10.8554 5.23078 9.81889 7.2097C9.72914 7.38106 9.87327 7.58141 10.0643 7.55079L10.6922 7.45012C12.4547 7.16758 14.379 7.70514 15.0699 9.35098C15.3914 10.1167 15.5 10.743 15.5 11"
          stroke={stroke}
        />
        <path
          d="M2 8H12C13.6569 8 15 9.34315 15 11V14C15 14.5523 14.5523 15 14 15H3C2.44772 15 2 14.5523 2 14V8Z"
          fill={`url(#${paint2})`}
        />
      </g>
      <defs>
        <filter
          id={filter0}
          x="-0.101562"
          y="1.76611"
          width="23.3604"
          height="23.4668"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset dy="2" />
          <feGaussianBlur stdDeviation="2" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix type="matrix" values={shadowMatrix} />
          <feBlend mode="normal" in2="BackgroundImageFix" result="effect1" />
          <feBlend mode="normal" in="SourceGraphic" in2="effect1" result="shape" />
        </filter>
        <filter
          id={filter1}
          x="-2.99902"
          y="-1"
          width="23"
          height="23"
          filterUnits="userSpaceOnUse"
          colorInterpolationFilters="sRGB"
        >
          <feFlood floodOpacity="0" result="BackgroundImageFix" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset dy="2" />
          <feGaussianBlur stdDeviation="2" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix type="matrix" values={shadowMatrix} />
          <feBlend mode="normal" in2="BackgroundImageFix" result="effect1" />
          <feColorMatrix
            in="SourceAlpha"
            type="matrix"
            values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0"
            result="hardAlpha"
          />
          <feOffset dx="0.5" dy="0.5" />
          <feGaussianBlur stdDeviation="0.25" />
          <feComposite in2="hardAlpha" operator="out" />
          <feColorMatrix type="matrix" values={`0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 ${innerShadowAlpha} 0`} />
          <feBlend mode="normal" in2="effect1" result="effect2" />
          <feBlend mode="normal" in="SourceGraphic" in2="effect2" result="shape" />
        </filter>
        <linearGradient id={paint0} x1="7.5" y1="0" x2="7.5" y2="15" gradientUnits="userSpaceOnUse">
          <stop stopColor={paperTop} />
          <stop offset="1" stopColor={paperBottom} />
        </linearGradient>
        <radialGradient
          id={paint1}
          cx="0"
          cy="0"
          r="1"
          gradientTransform="matrix(-4.59752 3.97059 -3.97059 -5.12255 13.3307 3.20588)"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor={paperTop} />
          <stop offset="0.0001" stopColor={paperBottom} />
          <stop offset="1" stopColor={paperTop} />
        </radialGradient>
        <linearGradient id={paint2} x1="8.5" y1="8" x2="8.5" y2="15" gradientUnits="userSpaceOnUse">
          <stop stopColor={paperTop} stopOpacity="0" />
          <stop offset="1" stopColor={paperBottom} />
        </linearGradient>
        <clipPath id={clip}>
          <rect width="20" height="20" fill="white" />
        </clipPath>
      </defs>
    </svg>
  )
}

export const AddPageToolIcon = makeToolbarIcon(addPageUrl, addPageDarkUrl, 'AddPageToolIcon')
export const AddTextToolIcon = makeToolbarIcon(addTextUrl, addTextDarkUrl, 'AddTextToolIcon')
export const AddDocumentToolIcon = makeToolbarIcon(
  addDocumentUrl,
  addDocumentDarkUrl,
  'AddDocumentToolIcon',
)
export const CommentToolIcon = makeToolbarIcon(commentUrl, commentDarkUrl, 'CommentToolIcon')
export const InspectToolIcon = makeToolbarIcon(inspectUrl, inspectDarkUrl, 'InspectToolIcon')
export const SunToolIcon = makeToolbarIcon(sunUrl, sunDarkUrl, 'SunToolIcon')
export const MoonToolIcon = makeToolbarIcon(moonUrl, moonDarkUrl, 'MoonToolIcon')
export const SunMoonToolIcon = makeToolbarIcon(sunMoonUrl, sunMoonDarkUrl, 'SunMoonToolIcon')
export const ZoomChevronIcon = makeToolbarIcon(zoomChevronUrl, zoomChevronDarkUrl, 'ZoomChevronIcon')

// ── Pen-popup icons (inline JSX so `ink` can theme dynamically) ────────────

type PenIconProps = {
  ink?: string
  size?: number
  selected?: boolean
} & Omit<ComponentProps<'svg'>, 'width' | 'height' | 'viewBox'>

const DEFAULT_PEN_INK = '#BD4BE5'
const PEN_STROKE_IDLE = '#797875'
const PEN_STROKE_SELECTED = '#18181B'

export function PenSlimIcon({
  ink = DEFAULT_PEN_INK,
  size = 16,
  selected = false,
  ...props
}: PenIconProps) {
  // Figma node 360:12 — 16×16 frame with the pen body extending past the
  // bottom and clipped. The `ink` prop drives the cap fill.
  const prefix = `pen-slim-${useId()}`
  const maskId = `${prefix}-mask`
  const maskGradId = `${prefix}-mask-grad`
  const bodyGradId = `${prefix}-body-grad`
  const shineGradId = `${prefix}-shine-grad`
  const seamGradId = `${prefix}-seam-grad`
  const strokeColor = selected ? PEN_STROKE_SELECTED : PEN_STROKE_IDLE
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <mask
        id={maskId}
        style={{ maskType: 'alpha' }}
        maskUnits="userSpaceOnUse"
        x="2"
        y="1"
        width="12"
        height="16"
      >
        <rect
          width="11.25"
          height="15.75"
          transform="matrix(1 0 0 -1 2 16.75)"
          fill={`url(#${maskGradId})`}
        />
      </mask>
      <g mask={`url(#${maskId})`}>
        <path
          d="M9.5 5.40918C9.49999 6.08065 8.95565 6.62499 8.28418 6.625L6.96582 6.625C6.29435 6.62499 5.75001 6.08065 5.75 5.40918C5.75 5.27181 5.77376 5.13545 5.81934 5.00586L6.88477 1.97461C7.13063 1.27547 8.11937 1.27547 8.36523 1.97461L9.43066 5.00586C9.47623 5.13545 9.5 5.27181 9.5 5.40918Z"
          fill={ink}
          stroke={strokeColor}
          strokeWidth="0.75"
        />
        <path
          d="M12.125 12.6113L12.125 40.75C12.125 41.7855 11.2855 42.625 10.25 42.625L4.99999 42.625C3.96446 42.625 3.12499 41.7855 3.12499 40.75L3.125 12.6045C3.12503 12.3656 3.17088 12.129 3.25976 11.9072L5.21191 7.03906C5.48309 6.36293 6.11591 5.90562 6.82812 5.88574C7.3608 5.87089 7.85049 5.87141 8.38379 5.88672C9.09419 5.90712 9.72602 6.36038 10 7.03223L11.9863 11.9033C12.078 12.1281 12.125 12.3686 12.125 12.6113Z"
          fill={`url(#${bodyGradId})`}
          stroke={strokeColor}
          strokeWidth="0.75"
        />
        <path
          d="M9.57592 7.28068L11.75 12.25C10.6792 11.4254 9.11786 11.8382 8.59456 13.0842L8 14.5L8 6.25C8.68341 6.25 9.302 6.65457 9.57592 7.28068Z"
          fill={`url(#${shineGradId})`}
        />
        <rect x="7.25" y="6.25" width="0.75" height="9" fill={`url(#${seamGradId})`} />
      </g>
      <defs>
        <PenIconDefs
          prefix={prefix}
          mask={{ x1: '5.625', y1: '0', x2: '5.625', y2: '15.75' }}
          body={{ x1: '7.89091', y1: '27.2601', x2: '6.93068', y2: '27.2601' }}
          shine={{ x1: '9.38755', y1: '6.33165', x2: '10.8421', y2: '12.6828' }}
          seam={{ x1: '7.625', y1: '6.25', x2: '7.625', y2: '15.25' }}
        />
      </defs>
    </svg>
  )
}

export function PenMarkerIcon({
  ink = DEFAULT_PEN_INK,
  size = 16,
  selected = false,
  ...props
}: PenIconProps) {
  // Figma node 360:22 — 16×16 frame; cap, body, and tip rect take the ink
  // color. Body extends past the bottom and is clipped by the mask.
  const prefix = `pen-marker-${useId()}`
  const maskId = `${prefix}-mask`
  const maskGradId = `${prefix}-mask-grad`
  const bodyGradId = `${prefix}-body-grad`
  const shineGradId = `${prefix}-shine-grad`
  const seamGradId = `${prefix}-seam-grad`
  const strokeColor = selected ? PEN_STROKE_SELECTED : PEN_STROKE_IDLE
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <mask
        id={maskId}
        style={{ maskType: 'alpha' }}
        maskUnits="userSpaceOnUse"
        x="2"
        y="0"
        width="13"
        height="17"
      >
        <rect
          width="12"
          height="16.8"
          transform="matrix(1 0 0 -1 2.40039 16.7998)"
          fill={`url(#${maskGradId})`}
        />
      </mask>
      <g mask={`url(#${maskId})`}>
        <path
          d="M10.5 4.55078C10.4998 5.40658 9.80602 6.10048 8.95019 6.10059L7.84961 6.10059C6.99388 6.10037 6.30002 5.40651 6.2998 4.55078L6.2998 3.35059C6.29988 3.03895 6.50628 2.76528 6.80566 2.67871L9.60547 1.86914C10.053 1.73978 10.4998 2.07524 10.5 2.54102L10.5 4.55078Z"
          fill={ink}
          stroke={ink}
        />
        <path
          d="M13.0996 12.3867L13.0996 42.4004C13.0996 43.4497 12.2486 44.3008 11.1992 44.3008L5.5996 44.3008C4.55026 44.3008 3.69921 43.4497 3.69921 42.4004L3.69922 12.3789C3.69925 12.1367 3.74578 11.8967 3.83594 11.6719L4.54199 9.91309C4.90888 8.99827 5.17809 8.04729 5.34668 7.07617L5.38867 6.83887C5.53997 5.96777 6.26471 5.32898 7.12598 5.3125C7.94758 5.29698 8.81329 5.29749 9.63477 5.31348C10.4938 5.33034 11.2187 5.96424 11.375 6.83105L11.4209 7.08594C11.5952 8.05266 11.8703 8.99862 12.2412 9.9082L12.959 11.6689C13.0519 11.8967 13.0996 12.1407 13.0996 12.3867Z"
          fill={`url(#${bodyGradId})`}
          stroke={strokeColor}
        />
        <path
          d="M8.7998 5.80078L9.84127 5.80078C10.3274 5.80078 10.7426 6.15084 10.829 6.62924C11.515 10.4243 11.6502 9.94325 12.5 12.0007C11.5381 11.1467 10.0216 11.4914 9.52348 12.6775L8.7998 14.4007L8.7998 5.80078Z"
          fill={`url(#${shineGradId})`}
        />
        <rect
          x="12.4004"
          y="13"
          width="1"
          height="8"
          transform="rotate(90 12.4004 13)"
          fill={`url(#${seamGradId})`}
        />
        <rect x="4" y="13.5996" width="8.8" height="3.2" fill={ink} />
      </g>
      <defs>
        <PenIconDefs
          prefix={prefix}
          mask={{ x1: '6', y1: '0', x2: '6', y2: '16.8' }}
          body={{ x1: '8.68324', y1: '28.0115', x2: '7.659', y2: '28.0115' }}
          shine={{ x1: '10.2799', y1: '5.68792', x2: '11.8314', y2: '12.4625' }}
          seam={{ x1: '12.9004', y1: '13', x2: '12.9004', y2: '21' }}
        />
      </defs>
    </svg>
  )
}

export function RotateIcon({
  size = 14,
  ...props
}: { size?: number } & Omit<ComponentProps<'svg'>, 'width' | 'height' | 'viewBox'>) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 14 14"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        d="M1 6.88233V4.72047C1.00005 4.17449 1.21712 3.65103 1.60319 3.26496C1.98926 2.87888 2.5127 2.66181 3.05868 2.66176H5.89125L4.7838 1.55429C4.54272 1.3131 4.54272 0.922053 4.7838 0.680862C5.02498 0.439679 5.416 0.439746 5.65722 0.680862L7.81904 2.84272C7.93488 2.95856 8 3.11562 8 3.27943C8 3.44325 7.93488 3.60031 7.81904 3.71615L5.65722 5.878C5.416 6.11912 5.02498 6.11919 4.7838 5.878C4.54272 5.63681 4.54272 5.24577 4.7838 5.00458L5.89125 3.89711H3.05868C2.84033 3.89716 2.631 3.98398 2.4766 4.13839C2.32221 4.29279 2.23538 4.50212 2.23533 4.72047V6.88233C2.23533 7.22346 1.95879 7.5 1.61766 7.5C1.27654 7.5 1 7.22346 1 6.88233Z"
        fill="currentColor"
      />
      <path
        d="M13 6.11767L13 8.27953C12.9999 8.82552 12.7829 9.34897 12.3968 9.73504C12.0107 10.1211 11.4873 10.3382 10.9413 10.3382L8.10875 10.3382L9.2162 11.4457C9.45728 11.6869 9.45728 12.0779 9.2162 12.3191C8.97502 12.5603 8.584 12.5603 8.34278 12.3191L6.18096 10.1573C6.06512 10.0414 6 9.88438 6 9.72057C6 9.55675 6.06512 9.39969 6.18096 9.28385L8.34278 7.122C8.584 6.88088 8.97502 6.88081 9.2162 7.122C9.45728 7.36319 9.45728 7.75423 9.2162 7.99542L8.10875 9.10289L10.9413 9.10289C11.1597 9.10284 11.369 9.01602 11.5234 8.86161C11.6778 8.70721 11.7646 8.49788 11.7647 8.27953L11.7647 6.11767C11.7647 5.77654 12.0412 5.5 12.3823 5.5C12.7235 5.5 13 5.77654 13 6.11767Z"
        fill="currentColor"
      />
    </svg>
  )
}

export function StrokeThinIcon(props: ComponentProps<'svg'>) {
  return (
    <svg
      width="17"
      height="9"
      viewBox="0 0 17 9"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        d="M0.5 4.42123C2.49609 2.87695 3.99447 1.21776 5.99609 1.37695C9.99566 1.69504 3.4279 8.75642 8.49772 8.49268C12.4961 8.28467 10.5 0.5 16.5 0.5"
        stroke="currentColor"
        strokeLinecap="round"
      />
    </svg>
  )
}

export function StrokeThickIcon(props: ComponentProps<'svg'>) {
  return (
    <svg
      width="19"
      height="11"
      viewBox="0 0 19 11"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      {...props}
    >
      <path
        d="M1.5 5.42123C3.49609 3.87695 4.99447 2.21776 6.99609 2.37695C10.9957 2.69504 4.4279 9.75642 9.49772 9.49268C13.4961 9.28467 11.5 1.5 17.5 1.5"
        stroke="currentColor"
        strokeWidth="3"
        strokeLinecap="round"
      />
    </svg>
  )
}

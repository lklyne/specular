import { ipcRenderer } from 'electron'
import {
  deepElementFromPoint,
  elementSelectorParts,
  inspectionPayload,
} from './dom-element-utils'
import { isPageOverlayTarget } from './gesture-forwarding'

let domInspectionEnabled = false
let domInspectionHoverKey = ''
let domInspectionHighlightEl: HTMLDivElement | null = null
let domInspectionPinnedHighlightEl: HTMLDivElement | null = null
let domInspectionLabelEl: HTMLDivElement | null = null
let domInspectionLastTarget: Element | null = null
let domInspectionPinnedTarget: Element | null = null

// Chrome-style box model overlay: 4 strips for margin (orange, hashed),
// 4 strips for padding (green, solid). Rendered as fixed-position divs.
type Side = 'top' | 'right' | 'bottom' | 'left'
type StripKind = 'margin' | 'padding'
const SIDES: readonly Side[] = ['top', 'right', 'bottom', 'left'] as const
const STRIP_KINDS: readonly StripKind[] = ['margin', 'padding'] as const
let domInspectionStripEls: Record<StripKind, Record<Side, HTMLDivElement>> | null = null
let domInspectionGapStrips: HTMLDivElement[] = []

const MARGIN_COLOR = 'rgba(246, 178, 107, 0.55)'
const PADDING_COLOR = 'rgba(147, 196, 125, 0.55)'
const MARGIN_HASH = `repeating-linear-gradient(45deg, ${MARGIN_COLOR} 0 4px, rgba(246, 178, 107, 0.25) 4px 8px)`
const GAP_COLOR = 'rgba(147, 112, 219, 0.55)'
const GAP_HASH = `repeating-linear-gradient(45deg, ${GAP_COLOR} 0 4px, rgba(147, 112, 219, 0.25) 4px 8px)`

export function isDomInspectionEnabled(): boolean {
  return domInspectionEnabled
}

export function getDomInspectionLastTarget(): Element | null {
  return domInspectionLastTarget
}

export function ensureDomInspectionOverlay(): void {
  if (domInspectionHighlightEl && domInspectionPinnedHighlightEl && domInspectionLabelEl) return

  const createHighlight = (id: string, backgroundAlpha: number, borderAlpha: number) => {
    const highlight = document.createElement('div')
    highlight.id = id
    Object.assign(highlight.style, {
      position: 'fixed',
      zIndex: '2147483645',
      pointerEvents: 'none',
      border: `1px solid rgba(59, 130, 246, ${borderAlpha})`,
      borderStyle: 'dashed',
      background: `rgba(59, 130, 246, ${backgroundAlpha})`,
      boxShadow: '0 0 0 1px rgba(255,255,255,0.22) inset',
      display: 'none',
    })
    return highlight
  }

  const pinnedHighlight = createHighlight(
    '__canvas-dom-inspection-pinned-highlight',
    0.14,
    0.95,
  )
  const highlight = createHighlight('__canvas-dom-inspection-highlight', 0.14, 0.95)
  Object.assign(highlight.style, {
    position: 'fixed',
    zIndex: '2147483646',
  })

  const label = document.createElement('div')
  label.id = '__canvas-dom-inspection-label'
  Object.assign(label.style, {
    position: 'fixed',
    zIndex: '2147483647',
    pointerEvents: 'none',
    maxWidth: '360px',
    padding: '5px 6px',
    borderRadius: '6px',
    background: 'rgba(30, 41, 59, 0.96)',
    color: '#ffffff',
    font: '11px/1.25 ui-monospace, SFMono-Regular, Menlo, monospace',
    boxShadow: '0 2px 8px rgba(0,0,0,0.25)',
    display: 'none',
  })

  const makeStrip = (kind: StripKind, side: Side) => {
    const strip = document.createElement('div')
    strip.id = `__canvas-dom-inspection-${kind}-${side}`
    Object.assign(strip.style, {
      position: 'fixed',
      zIndex: '2147483644',
      pointerEvents: 'none',
      display: 'none',
      background: kind === 'margin' ? MARGIN_HASH : PADDING_COLOR,
    })
    return strip
  }

  const strips = {} as Record<StripKind, Record<Side, HTMLDivElement>>
  for (const kind of STRIP_KINDS) {
    const sideMap = {} as Record<Side, HTMLDivElement>
    for (const side of SIDES) {
      sideMap[side] = makeStrip(kind, side)
      document.documentElement.appendChild(sideMap[side])
    }
    strips[kind] = sideMap
  }

  document.documentElement.appendChild(pinnedHighlight)
  document.documentElement.appendChild(highlight)
  document.documentElement.appendChild(label)
  domInspectionPinnedHighlightEl = pinnedHighlight
  domInspectionHighlightEl = highlight
  domInspectionLabelEl = label
  domInspectionStripEls = strips
}

function hideBoxModelOverlay(): void {
  if (!domInspectionStripEls) return
  for (const kind of STRIP_KINDS) {
    for (const side of SIDES) domInspectionStripEls[kind][side].style.display = 'none'
  }
  hideGapStrips()
}

function ensureGapStrip(index: number): HTMLDivElement {
  if (index < domInspectionGapStrips.length) return domInspectionGapStrips[index]
  const strip = document.createElement('div')
  strip.id = `__canvas-dom-inspection-gap-${index}`
  Object.assign(strip.style, {
    position: 'fixed',
    zIndex: '2147483644',
    pointerEvents: 'none',
    display: 'none',
    background: GAP_HASH,
  })
  document.documentElement.appendChild(strip)
  domInspectionGapStrips.push(strip)
  return strip
}

function hideGapStrips(): void {
  for (const strip of domInspectionGapStrips) {
    strip.style.display = 'none'
  }
}

interface GapRect {
  x: number
  y: number
  w: number
  h: number
}

function updateFlexGapStrips(el: Element, rect: DOMRect, styles: CSSStyleDeclaration): void {
  const isRow = styles.flexDirection === 'row' || styles.flexDirection === 'row-reverse'
  const rowGap = parsePx(styles.rowGap)
  const columnGap = parsePx(styles.columnGap)

  if (rowGap === 0 && columnGap === 0) {
    hideGapStrips()
    return
  }

  const childRects = Array.from(el.children)
    .map(child => child.getBoundingClientRect())
    .filter(r => r.width > 0 && r.height > 0)

  if (childRects.length < 2) {
    hideGapStrips()
    return
  }

  const bl = parsePx(styles.borderLeftWidth)
  const bt = parsePx(styles.borderTopWidth)
  const br = parsePx(styles.borderRightWidth)
  const bb = parsePx(styles.borderBottomWidth)
  const contentLeft = rect.left + bl + parsePx(styles.paddingLeft)
  const contentTop = rect.top + bt + parsePx(styles.paddingTop)
  const contentRight = rect.right - br - parsePx(styles.paddingRight)
  const contentBottom = rect.bottom - bb - parsePx(styles.paddingBottom)

  const gaps: GapRect[] = []

  if (isRow) {
    // Group children into flex lines: a new line starts when a child's top >= current line's max bottom.
    const sorted = childRects.slice().sort((a, b) => a.top - b.top || a.left - b.left)
    const lines: DOMRect[][] = []
    let currentLine: DOMRect[] = []
    let lineMaxBottom = -Infinity

    for (const r of sorted) {
      if (currentLine.length === 0 || r.top < lineMaxBottom) {
        currentLine.push(r)
        lineMaxBottom = Math.max(lineMaxBottom, r.bottom)
      } else {
        lines.push(currentLine.slice().sort((a, b) => a.left - b.left))
        currentLine = [r]
        lineMaxBottom = r.bottom
      }
    }
    if (currentLine.length > 0) lines.push(currentLine.slice().sort((a, b) => a.left - b.left))

    if (columnGap > 0) {
      for (const line of lines) {
        const lineTop = Math.min(...line.map(r => r.top))
        const lineBottom = Math.max(...line.map(r => r.bottom))
        for (let i = 0; i < line.length - 1; i++) {
          gaps.push({ x: line[i].right, y: lineTop, w: columnGap, h: lineBottom - lineTop })
        }
      }
    }

    if (rowGap > 0 && lines.length > 1) {
      for (let i = 0; i < lines.length - 1; i++) {
        const lineABottom = Math.max(...lines[i].map(r => r.bottom))
        gaps.push({ x: contentLeft, y: lineABottom, w: contentRight - contentLeft, h: rowGap })
      }
    }
  } else {
    // Column direction: group children into flex columns.
    const sorted = childRects.slice().sort((a, b) => a.left - b.left || a.top - b.top)
    const columns: DOMRect[][] = []
    let currentColumn: DOMRect[] = []
    let colMaxRight = -Infinity

    for (const r of sorted) {
      if (currentColumn.length === 0 || r.left < colMaxRight) {
        currentColumn.push(r)
        colMaxRight = Math.max(colMaxRight, r.right)
      } else {
        columns.push(currentColumn.slice().sort((a, b) => a.top - b.top))
        currentColumn = [r]
        colMaxRight = r.right
      }
    }
    if (currentColumn.length > 0) columns.push(currentColumn.slice().sort((a, b) => a.top - b.top))

    if (rowGap > 0) {
      for (const col of columns) {
        const colLeft = Math.min(...col.map(r => r.left))
        const colRight = Math.max(...col.map(r => r.right))
        for (let i = 0; i < col.length - 1; i++) {
          gaps.push({ x: colLeft, y: col[i].bottom, w: colRight - colLeft, h: rowGap })
        }
      }
    }

    if (columnGap > 0 && columns.length > 1) {
      for (let i = 0; i < columns.length - 1; i++) {
        const colARight = Math.max(...columns[i].map(r => r.right))
        gaps.push({ x: colARight, y: contentTop, w: columnGap, h: contentBottom - contentTop })
      }
    }
  }

  for (let i = 0; i < gaps.length; i++) {
    const { x, y, w, h } = gaps[i]
    setStripRect(ensureGapStrip(i), x, y, w, h)
  }
  for (let i = gaps.length; i < domInspectionGapStrips.length; i++) {
    domInspectionGapStrips[i].style.display = 'none'
  }
}

function setStripRect(
  strip: HTMLDivElement,
  x: number,
  y: number,
  width: number,
  height: number,
): void {
  if (width <= 0 || height <= 0) {
    strip.style.display = 'none'
    return
  }
  strip.style.display = 'block'
  strip.style.left = `${Math.round(x)}px`
  strip.style.top = `${Math.round(y)}px`
  strip.style.width = `${Math.round(width)}px`
  strip.style.height = `${Math.round(height)}px`
}

function parsePx(value: string): number {
  const n = parseFloat(value)
  return Number.isFinite(n) ? n : 0
}

function updateBoxModelOverlay(rect: DOMRect, styles: CSSStyleDeclaration, el: Element): void {
  if (!domInspectionStripEls) return
  const mt = parsePx(styles.marginTop)
  const mr = parsePx(styles.marginRight)
  const mb = parsePx(styles.marginBottom)
  const ml = parsePx(styles.marginLeft)
  const pt = parsePx(styles.paddingTop)
  const pr = parsePx(styles.paddingRight)
  const pb = parsePx(styles.paddingBottom)
  const pl = parsePx(styles.paddingLeft)
  const bt = parsePx(styles.borderTopWidth)
  const br = parsePx(styles.borderRightWidth)
  const bb = parsePx(styles.borderBottomWidth)
  const bl = parsePx(styles.borderLeftWidth)

  const margins = domInspectionStripEls.margin
  const paddings = domInspectionStripEls.padding

  // Margin strips lie OUTSIDE the border-box. Top/bottom span full width
  // (covering margin corners); left/right fit between them.
  setStripRect(margins.top, rect.left - ml, rect.top - mt, rect.width + ml + mr, mt)
  setStripRect(margins.bottom, rect.left - ml, rect.bottom, rect.width + ml + mr, mb)
  setStripRect(margins.left, rect.left - ml, rect.top, ml, rect.height)
  setStripRect(margins.right, rect.right, rect.top, mr, rect.height)

  // Padding strips lie INSIDE the border-box, between border and content.
  const padBoxX = rect.left + bl
  const padBoxY = rect.top + bt
  const padBoxW = Math.max(0, rect.width - bl - br)
  const padBoxH = Math.max(0, rect.height - bt - bb)
  setStripRect(paddings.top, padBoxX, padBoxY, padBoxW, pt)
  setStripRect(paddings.bottom, padBoxX, padBoxY + padBoxH - pb, padBoxW, pb)
  setStripRect(paddings.left, padBoxX, padBoxY + pt, pl, Math.max(0, padBoxH - pt - pb))
  setStripRect(paddings.right, padBoxX + padBoxW - pr, padBoxY + pt, pr, Math.max(0, padBoxH - pt - pb))

  const display = styles.display
  if (display === 'flex' || display === 'inline-flex') {
    updateFlexGapStrips(el, rect, styles)
  } else {
    hideGapStrips()
  }
}

function shortFontFamily(fontFamily: string): string {
  // Computed font-family is a comma-separated stack; the first family is the
  // resolved primary. Strip surrounding quotes for display.
  const first = fontFamily.split(',')[0]?.trim() ?? ''
  return first.replace(/^['"]|['"]$/g, '')
}

function buildLabelContent(label: HTMLDivElement, element: Element, styles: CSSStyleDeclaration): void {
  label.replaceChildren()

  const headerRow = document.createElement('div')
  Object.assign(headerRow.style, {
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    minWidth: '0',
  })

  const { tag, remainder } = elementSelectorParts(element)

  const chip = document.createElement('span')
  Object.assign(chip.style, {
    padding: '1px 5px',
    borderRadius: '4px',
    background: 'rgba(59, 130, 246, 0.95)',
    color: '#fff',
    flexShrink: '0',
  })
  chip.textContent = tag
  headerRow.appendChild(chip)

  if (remainder) {
    const name = document.createElement('span')
    Object.assign(name.style, {
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      minWidth: '0',
      opacity: '0.9',
    })
    name.textContent = remainder
    headerRow.appendChild(name)
  }

  label.appendChild(headerRow)

  const fontFamily = shortFontFamily(styles.fontFamily)
  if (fontFamily) {
    const fontRow = document.createElement('div')
    Object.assign(fontRow.style, {
      display: 'flex',
      alignItems: 'baseline',
      gap: '6px',
      marginTop: '6px',
      whiteSpace: 'nowrap',
      minWidth: '0',
    })

    const familyName = document.createElement('span')
    Object.assign(familyName.style, {
      fontFamily: styles.fontFamily,
      fontWeight: styles.fontWeight,
      fontStyle: styles.fontStyle,
      color: '#fff',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      minWidth: '0',
    })
    familyName.textContent = fontFamily
    fontRow.appendChild(familyName)

    const metaParts = [styles.fontSize, styles.fontWeight]
    if (styles.letterSpacing && styles.letterSpacing !== 'normal') {
      metaParts.push(styles.letterSpacing)
    }
    const metaRest = document.createElement('span')
    Object.assign(metaRest.style, {
      opacity: '0.7',
      flexShrink: '0',
    })
    metaRest.textContent = `· ${metaParts.join(' · ')}`
    fontRow.appendChild(metaRest)

    label.appendChild(fontRow)
  }

  const colorRow = document.createElement('div')
  Object.assign(colorRow.style, {
    display: 'flex',
    alignItems: 'center',
    gap: '10px',
    marginTop: '6px',
  })

  const addSwatch = (name: string, color: string) => {
    const item = document.createElement('span')
    Object.assign(item.style, {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '4px',
      opacity: '0.85',
    })
    const sw = document.createElement('span')
    Object.assign(sw.style, {
      display: 'inline-block',
      width: '10px',
      height: '10px',
      borderRadius: '2px',
      background: color,
      boxShadow: 'inset 0 0 0 1px rgba(255,255,255,0.35)',
      flexShrink: '0',
    })
    const text = document.createElement('span')
    text.textContent = `${name} ${color}`
    item.appendChild(sw)
    item.appendChild(text)
    colorRow.appendChild(item)
  }
  addSwatch('text', styles.color)
  if (styles.backgroundColor && styles.backgroundColor !== 'rgba(0, 0, 0, 0)') {
    addSwatch('bg', styles.backgroundColor)
  }
  label.appendChild(colorRow)
}

export function hideDomInspectionOverlay(): void {
  if (domInspectionHighlightEl) domInspectionHighlightEl.style.display = 'none'
  if (domInspectionPinnedHighlightEl) domInspectionPinnedHighlightEl.style.display = 'none'
  if (domInspectionLabelEl) domInspectionLabelEl.style.display = 'none'
  hideBoxModelOverlay()
}

export function updateDomInspectionOverlay(
  element: Element,
  payload: ReturnType<typeof inspectionPayload>,
  options?: {
    pinnedElement?: Element | null
    pinnedPayload?: ReturnType<typeof inspectionPayload> | null
    showLabel?: boolean
  },
): void {
  ensureDomInspectionOverlay()
  if (!domInspectionHighlightEl || !domInspectionPinnedHighlightEl || !domInspectionLabelEl) return

  const setHighlightRect = (highlight: HTMLDivElement, target: Element | null) => {
    if (!target) {
      highlight.style.display = 'none'
      return null
    }
    const rect = target.getBoundingClientRect()
    highlight.style.display = 'block'
    highlight.style.left = `${Math.round(rect.left)}px`
    highlight.style.top = `${Math.round(rect.top)}px`
    highlight.style.width = `${Math.max(0, Math.round(rect.width))}px`
    highlight.style.height = `${Math.max(0, Math.round(rect.height))}px`
    return rect
  }

  const pinnedElement = options?.pinnedElement ?? null
  const pinnedPayload = options?.pinnedPayload ?? null
  const sameAsPinned = pinnedPayload?.nodeId === payload.nodeId

  if (pinnedElement && pinnedPayload && !sameAsPinned) {
    setHighlightRect(domInspectionPinnedHighlightEl, pinnedElement)
    domInspectionHighlightEl.style.border = '1px solid rgba(59, 130, 246, 0.48)'
    domInspectionHighlightEl.style.background = 'rgba(59, 130, 246, 0.07)'
  } else {
    domInspectionPinnedHighlightEl.style.display = 'none'
    domInspectionHighlightEl.style.border = '1px solid rgba(59, 130, 246, 0.95)'
    domInspectionHighlightEl.style.background = 'rgba(59, 130, 246, 0.14)'
  }

  const rect = setHighlightRect(domInspectionHighlightEl, element)
  if (!rect) return

  const styles = window.getComputedStyle(element)
  updateBoxModelOverlay(rect, styles, element)

  if (options?.showLabel === false) {
    domInspectionLabelEl.style.display = 'none'
    domInspectionLabelEl.replaceChildren()
    return
  }

  domInspectionLabelEl.style.display = 'block'
  buildLabelContent(domInspectionLabelEl, element, styles)
  const outerPadding = 2
  const labelRect = domInspectionLabelEl.getBoundingClientRect()
  const maxLeft = Math.max(outerPadding, window.innerWidth - Math.ceil(labelRect.width) - outerPadding)
  const left = Math.min(maxLeft, Math.max(outerPadding, Math.round(rect.left)))

  // Keep existing "prefer above target" behavior, but adaptively place below when space is tight.
  const aboveTop = Math.round(rect.top) - Math.ceil(labelRect.height) - outerPadding
  const belowTop = Math.round(rect.bottom) + outerPadding
  const maxTop = Math.max(outerPadding, window.innerHeight - Math.ceil(labelRect.height) - outerPadding)
  const top = aboveTop >= outerPadding ? aboveTop : Math.min(maxTop, belowTop)

  domInspectionLabelEl.style.left = `${left}px`
  domInspectionLabelEl.style.top = `${top}px`
}

export function emitHoveredElement(target: Element | null): void {
  if (!domInspectionEnabled) return
  if (!target) {
    domInspectionHoverKey = ''
    ipcRenderer.send('inspect-node-hover', null)
    if (domInspectionPinnedTarget) {
      const payload = inspectionPayload(domInspectionPinnedTarget)
      domInspectionLastTarget = domInspectionPinnedTarget
      updateDomInspectionOverlay(domInspectionPinnedTarget, payload, { showLabel: false })
      return
    }
    domInspectionLastTarget = null
    hideDomInspectionOverlay()
    return
  }

  const payload = inspectionPayload(target)
  const nextKey = `${payload.id}:${payload.elementPath}`
  if (nextKey === domInspectionHoverKey) {
    updateDomInspectionOverlay(target, payload, {
      pinnedElement: domInspectionPinnedTarget,
      pinnedPayload: domInspectionPinnedTarget ? inspectionPayload(domInspectionPinnedTarget) : null,
      showLabel: false,
    })
    return
  }

  domInspectionHoverKey = nextKey
  domInspectionLastTarget = target
  updateDomInspectionOverlay(target, payload, {
    pinnedElement: domInspectionPinnedTarget,
    pinnedPayload: domInspectionPinnedTarget ? inspectionPayload(domInspectionPinnedTarget) : null,
    showLabel: false,
  })
  ipcRenderer.send('inspect-node-hover', payload)
  ipcRenderer.send('inspect-node-detail-update', payload)
}

function handleDomInspectionMove(event: MouseEvent): void {
  if (!domInspectionEnabled) return
  const target = deepElementFromPoint(event.clientX, event.clientY)
  if (isPageOverlayTarget(target)) {
    emitHoveredElement(null)
    return
  }
  emitHoveredElement(target)
}

function handleDomInspectionClick(event: MouseEvent): void {
  if (!domInspectionEnabled) return
  const target = deepElementFromPoint(event.clientX, event.clientY)
  if (!target) return
  if (isPageOverlayTarget(target)) return
  event.preventDefault()
  event.stopPropagation()
  event.stopImmediatePropagation()
  const payload = inspectionPayload(target)
  domInspectionPinnedTarget = target
  updateDomInspectionOverlay(target, payload, { showLabel: false })
  ipcRenderer.send('inspect-node-select', payload)
  ipcRenderer.send('inspect-node-detail-update', payload)
}

function handleDomInspectionMouseDown(event: MouseEvent): void {
  if (!domInspectionEnabled) return
  const target = deepElementFromPoint(event.clientX, event.clientY)
  if (isPageOverlayTarget(target)) return
  event.preventDefault()
  event.stopPropagation()
  event.stopImmediatePropagation()
}

function handleDomInspectionLeave(): void {
  if (!domInspectionEnabled) return
  emitHoveredElement(null)
}

function handleDomInspectionMouseOut(event: MouseEvent): void {
  if (!domInspectionEnabled) return
  // Some WebContents transitions do not reliably fire `mouseleave` on window.
  // Treat mouseout to nowhere as a leave so hover state is cleared.
  if (event.relatedTarget === null) {
    emitHoveredElement(null)
  }
}

function handleDomInspectionBlur(): void {
  if (!domInspectionEnabled) return
  emitHoveredElement(null)
}

export function setDomInspectionEnabled(enabled: boolean): void {
  domInspectionEnabled = enabled
  applyDomInspectionState()
}

export function applyDomInspectionState(): void {
  window.removeEventListener('mousemove', handleDomInspectionMove, true)
  window.removeEventListener('mousedown', handleDomInspectionMouseDown, true)
  window.removeEventListener('click', handleDomInspectionClick, true)
  window.removeEventListener('mouseleave', handleDomInspectionLeave, true)
  window.removeEventListener('mouseout', handleDomInspectionMouseOut, true)
  window.removeEventListener('blur', handleDomInspectionBlur, true)
  if (domInspectionEnabled) {
    ensureDomInspectionOverlay()
    window.addEventListener('mousemove', handleDomInspectionMove, true)
    window.addEventListener('mousedown', handleDomInspectionMouseDown, true)
    window.addEventListener('click', handleDomInspectionClick, true)
    window.addEventListener('mouseleave', handleDomInspectionLeave, true)
    window.addEventListener('mouseout', handleDomInspectionMouseOut, true)
    window.addEventListener('blur', handleDomInspectionBlur, true)
  } else {
    window.removeEventListener('mousemove', handleDomInspectionMove, true)
    window.removeEventListener('click', handleDomInspectionClick, true)
    window.removeEventListener('mouseleave', handleDomInspectionLeave, true)
    window.removeEventListener('mouseout', handleDomInspectionMouseOut, true)
    window.removeEventListener('blur', handleDomInspectionBlur, true)
    domInspectionHoverKey = ''
    domInspectionLastTarget = null
    domInspectionPinnedTarget = null
    hideDomInspectionOverlay()
  }
}

export function queueRefreshDomInspectionOverlay(): void {
  if (!domInspectionEnabled) return
  const target = domInspectionLastTarget
  if (!target) return
  window.requestAnimationFrame(() => {
    if (!domInspectionEnabled || domInspectionLastTarget !== target) return
    const payload = inspectionPayload(target)
    updateDomInspectionOverlay(target, payload, {
      pinnedElement: domInspectionPinnedTarget,
      pinnedPayload: domInspectionPinnedTarget ? inspectionPayload(domInspectionPinnedTarget) : null,
      showLabel: false,
    })
  })
}

export function handleInspectFocusNode(
  payload: { nodeId?: string | null; pin?: boolean; fromPanel?: boolean } | undefined,
  getInspectableElementByNodeId: (nodeId: string | null) => Element | null,
): void {
  const target = getInspectableElementByNodeId(payload?.nodeId ?? null)
  if (!target) {
    if (payload?.pin) {
      domInspectionPinnedTarget = null
      domInspectionLastTarget = null
      hideDomInspectionOverlay()
    } else {
      emitHoveredElement(null)
    }
    return
  }
  // When driven from the panel, scroll to the element if it's off-screen
  if (payload?.fromPanel && payload?.pin) {
    const rect = target.getBoundingClientRect()
    const isOffScreen =
      rect.bottom < 0 || rect.top > window.innerHeight || rect.right < 0 || rect.left > window.innerWidth
    if (isOffScreen) {
      target.scrollIntoView({ block: 'center', behavior: 'smooth' })
    }
  }

  const detail = inspectionPayload(target)
  updateDomInspectionOverlay(target, detail, {
    pinnedElement: payload?.pin ? target : domInspectionPinnedTarget,
    pinnedPayload:
      payload?.pin || !domInspectionPinnedTarget ? detail : inspectionPayload(domInspectionPinnedTarget),
    showLabel: false,
  })
  domInspectionLastTarget = target
  if (payload?.pin) {
    domInspectionPinnedTarget = target
    // When driven from the panel, main already knows about the selection.
    // Sending inspect-node-select back would trigger a circular
    // selectPageById -> syncInspectionState that can hide the overlay.
    if (!payload.fromPanel) {
      ipcRenderer.send('inspect-node-select', detail)
    }
    ipcRenderer.send('inspect-node-detail-update', detail)
  } else {
    ipcRenderer.send('inspect-node-hover', detail)
    ipcRenderer.send('inspect-node-detail-update', detail)
  }
}

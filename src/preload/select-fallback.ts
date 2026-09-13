/**
 * In-page dropdown for menulist `<select>` elements (ADR 0038).
 *
 * macOS Chromium opens a menulist as an *external* native popup menu through
 * `RenderViewHostDelegateView::ShowPopupMenu`, which Electron's
 * `OffScreenWebContentsView` does not implement, and Blink's
 * `should_disable_external_popups` preference (the switch that would force
 * Blink's own internal popup widget) is not exposed by Electron. On an
 * offscreen page a click on a `<select>` therefore focuses it and shows
 * nothing. Internal popups — `<input type=date>`, autofill — are unaffected;
 * they arrive as popup textures and the canvas draws them.
 *
 * So the page paints the menu itself: one fixed host on `documentElement`
 * carrying a closed shadow root, driven by synthesized input like any other
 * page content. Focus stays on the `<select>` the whole time.
 */

import {
  isSelectableItem,
  nextHighlight,
  selectFallbackModel,
  typeAheadMatch,
  type HighlightKey,
  type SelectFallbackItem,
  type SelectOptionLike,
} from './select-fallback-model'

const HOST_ID = '__canvas-select-fallback'
const TYPE_AHEAD_RESET_MS = 700
const MENU_GAP = 2
const VIEWPORT_MARGIN = 4

const NAVIGATION_KEYS: Record<string, HighlightKey> = {
  ArrowDown: 'ArrowDown',
  ArrowUp: 'ArrowUp',
  Home: 'Home',
  End: 'End',
  PageDown: 'PageDown',
  PageUp: 'PageUp',
}

const MENU_CSS = `
.menu {
  box-sizing: border-box;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', system-ui, sans-serif;
  font-size: 13px;
  line-height: 1;
  color: #1a1a1a;
  background: #ffffff;
  border: 1px solid rgba(0, 0, 0, 0.18);
  border-radius: 6px;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.18);
  max-height: 320px;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 4px 0;
  user-select: none;
}
.row {
  display: flex;
  align-items: center;
  height: 28px;
  padding: 0 10px 0 6px;
  white-space: nowrap;
  cursor: default;
}
.tick { flex: 0 0 14px; text-align: center; }
.row[data-disabled='true'] { color: rgba(0, 0, 0, 0.36); }
.row[data-highlighted='true'] { background: #2f6feb; color: #ffffff; }
.group {
  display: flex;
  align-items: center;
  height: 24px;
  padding: 0 10px 0 6px;
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  color: rgba(0, 0, 0, 0.45);
  white-space: nowrap;
}
`

type OpenMenu = {
  select: HTMLSelectElement
  host: HTMLElement
  menu: HTMLElement
  items: SelectFallbackItem[]
  rows: HTMLElement[]
  selectedIndex: number
  highlight: number
  typedPrefix: string
  typedAt: number
  observer: MutationObserver
}

let open: OpenMenu | null = null
let installed = false

function isMenulist(node: Element | null): node is HTMLSelectElement {
  if (!(node instanceof HTMLSelectElement)) return false
  return !node.multiple && node.size <= 1 && !node.disabled
}

function readOptions(select: HTMLSelectElement): SelectOptionLike[] {
  const options: SelectOptionLike[] = []
  for (const option of select.options) {
    const group =
      option.parentElement instanceof HTMLOptGroupElement ? option.parentElement : null
    options.push({
      label: option.label,
      text: option.text,
      disabled: option.disabled,
      selected: option.selected,
      groupLabel: group?.label ?? null,
      groupDisabled: group?.disabled ?? false,
    })
  }
  return options
}

// `!important` throughout so a page-wide reset (`* { position: static }`,
// `div { display: flex }`) can't detach the menu from the element it belongs to.
function setHostStyle(host: HTMLElement, left: number, top: number): void {
  host.style.cssText = `
    position: fixed !important;
    left: ${left}px !important;
    top: ${top}px !important;
    z-index: 2147483647 !important;
    display: block !important;
    margin: 0 !important;
    padding: 0 !important;
    border: 0 !important;
    visibility: visible !important;
    opacity: 1 !important;
    pointer-events: auto !important;
  `
}

function positionMenu(menu: OpenMenu): void {
  const rect = menu.select.getBoundingClientRect()
  menu.menu.style.minWidth = `${Math.round(rect.width)}px`
  const height = menu.menu.offsetHeight
  const width = menu.menu.offsetWidth

  let top = rect.bottom + MENU_GAP
  const fitsBelow = top + height <= window.innerHeight - VIEWPORT_MARGIN
  const flipped = rect.top - MENU_GAP - height
  if (!fitsBelow && flipped >= VIEWPORT_MARGIN) top = flipped
  top = Math.max(
    VIEWPORT_MARGIN,
    Math.min(top, window.innerHeight - height - VIEWPORT_MARGIN),
  )
  const left = Math.max(
    VIEWPORT_MARGIN,
    Math.min(rect.left, window.innerWidth - width - VIEWPORT_MARGIN),
  )
  setHostStyle(menu.host, left, top)
}

function setHighlight(menu: OpenMenu, index: number, scroll = true): void {
  if (!isSelectableItem(menu.items[index])) return
  const previous = menu.rows[menu.highlight]
  if (previous) previous.dataset.highlighted = 'false'
  menu.highlight = index
  const row = menu.rows[index]
  if (!row) return
  row.dataset.highlighted = 'true'
  menu.menu.setAttribute('aria-activedescendant', row.id)
  if (scroll) row.scrollIntoView({ block: 'nearest' })
}

function commit(menu: OpenMenu, index: number): void {
  const item = menu.items[index]
  if (!isSelectableItem(item)) return
  const select = menu.select
  const changed = select.selectedIndex !== item.index
  closeSelectFallback()
  if (!changed) return
  select.selectedIndex = item.index
  select.dispatchEvent(new Event('input', { bubbles: true }))
  select.dispatchEvent(new Event('change', { bubbles: true }))
}

export function closeSelectFallback(): void {
  if (!open) return
  const menu = open
  open = null
  menu.observer.disconnect()
  menu.host.remove()
  menu.select.removeAttribute('aria-expanded')
}

function buildMenu(select: HTMLSelectElement): OpenMenu | null {
  const model = selectFallbackModel(readOptions(select))
  if (model.items.length === 0) return null

  const host = document.createElement('div')
  host.id = HOST_ID
  const shadow = host.attachShadow({ mode: 'closed' })
  const style = document.createElement('style')
  style.textContent = MENU_CSS
  const list = document.createElement('div')
  list.className = 'menu'
  list.setAttribute('role', 'listbox')

  const rows: HTMLElement[] = []
  model.items.forEach((item, index) => {
    const row = document.createElement('div')
    row.id = `${HOST_ID}-${index}`
    if (item.kind === 'group') {
      row.className = 'group'
      row.setAttribute('role', 'presentation')
      row.textContent = item.label
    } else {
      row.className = 'row'
      row.setAttribute('role', 'option')
      row.dataset.disabled = String(item.disabled)
      row.dataset.highlighted = 'false'
      row.setAttribute('aria-selected', String(index === model.selectedIndex))
      if (item.disabled) row.setAttribute('aria-disabled', 'true')
      const tick = document.createElement('span')
      tick.className = 'tick'
      tick.textContent = index === model.selectedIndex ? '✓' : ''
      row.append(tick, document.createTextNode(item.label || ' '))
    }
    rows.push(row)
    list.appendChild(row)
  })

  shadow.append(style, list)

  const menu: OpenMenu = {
    select,
    host,
    menu: list,
    items: model.items,
    rows,
    selectedIndex: model.selectedIndex,
    highlight: -1,
    typedPrefix: '',
    typedAt: 0,
    observer: new MutationObserver(() => {
      if (!select.isConnected) closeSelectFallback()
    }),
  }

  const rowIndexAt = (target: EventTarget | null): number => {
    if (!(target instanceof Element)) return -1
    // `closest` stops at the shadow boundary, so this only ever finds our rows.
    const row = target.closest('.row')
    return row ? rows.indexOf(row as HTMLElement) : -1
  }

  list.addEventListener('mousemove', (event: MouseEvent) => {
    const index = rowIndexAt(event.target)
    if (index >= 0) setHighlight(menu, index, false)
  })
  // Commit on mouseup, like a native macOS menu: both click-then-click and
  // press-drag-release land on the row under the pointer.
  list.addEventListener('mouseup', (event: MouseEvent) => {
    if (event.button !== 0) return
    const index = rowIndexAt(event.target)
    if (index < 0) return
    event.preventDefault()
    commit(menu, index)
  })

  return menu
}

function openMenu(select: HTMLSelectElement): void {
  closeSelectFallback()
  const menu = buildMenu(select)
  if (!menu) return
  open = menu
  document.documentElement.appendChild(menu.host)
  setHostStyle(menu.host, 0, 0)
  select.setAttribute('aria-expanded', 'true')
  setHighlight(menu, menu.selectedIndex, false)
  if (menu.highlight < 0) {
    setHighlight(menu, nextHighlight(menu.items, -1, 'ArrowDown'), false)
  }
  positionMenu(menu)
  menu.rows[menu.highlight]?.scrollIntoView({ block: 'nearest' })
  menu.observer.observe(document.documentElement, { childList: true, subtree: true })
}

function handleMouseDown(event: MouseEvent): void {
  if (event.button !== 0) return
  if (open && event.composedPath().includes(open.host)) return
  const select = event.target instanceof Element ? event.target.closest('select') : null
  if (!isMenulist(select)) {
    closeSelectFallback()
    return
  }
  // Blink must not start opening the external popup: it would leave the
  // element in a stuck "popup open" state that swallows later clicks. Only the
  // default action is suppressed — propagation continues so the page's own
  // mousedown handlers still see the click, as they would natively.
  event.preventDefault()
  if (open?.select === select) {
    closeSelectFallback()
    return
  }
  // preventDefault also cancels the native focus that mousedown would apply.
  select.focus({ preventScroll: true })
  openMenu(select)
}

function handleOpenKeyDown(menu: OpenMenu, event: KeyboardEvent): void {
  const key = event.key
  const navigation = NAVIGATION_KEYS[key]
  if (navigation) {
    event.preventDefault()
    event.stopPropagation()
    setHighlight(menu, nextHighlight(menu.items, menu.highlight, navigation))
    return
  }
  if (key === 'Enter' || key === ' ') {
    event.preventDefault()
    event.stopPropagation()
    commit(menu, menu.highlight)
    // A no-op commit (nothing highlighted) still dismisses, as a native menu does.
    closeSelectFallback()
    return
  }
  if (key === 'Escape') {
    event.preventDefault()
    event.stopPropagation()
    closeSelectFallback()
    return
  }
  if (key === 'Tab') {
    closeSelectFallback()
    return
  }
  if (key.length !== 1 || event.metaKey || event.ctrlKey || event.altKey) return
  event.preventDefault()
  event.stopPropagation()
  const now = Date.now()
  menu.typedPrefix = now - menu.typedAt > TYPE_AHEAD_RESET_MS ? key : menu.typedPrefix + key
  menu.typedAt = now
  const match = typeAheadMatch(menu.items, menu.typedPrefix, menu.highlight)
  if (match >= 0) setHighlight(menu, match)
}

function handleKeyDown(event: KeyboardEvent): void {
  if (open) {
    handleOpenKeyDown(open, event)
    return
  }
  const select = document.activeElement
  if (!isMenulist(select)) return
  // Bare ArrowUp/ArrowDown stay with Blink, which steps the value natively.
  const opens =
    (event.key === 'ArrowDown' && event.altKey) || event.key === ' ' || event.key === 'Enter'
  if (!opens) return
  event.preventDefault()
  openMenu(select)
}

function handleViewportChange(): void {
  if (!open) return
  if (!open.select.isConnected) {
    closeSelectFallback()
    return
  }
  positionMenu(open)
}

/**
 * Install the fallback. Only ever called from the page-content preload, which
 * main attaches to page webContents alone — never to the app's own renderers.
 */
export function installSelectFallback(): void {
  if (installed) return
  installed = true
  window.addEventListener('mousedown', handleMouseDown, true)
  window.addEventListener('keydown', handleKeyDown, true)
  window.addEventListener('scroll', handleViewportChange, { capture: true, passive: true })
  window.addEventListener('resize', handleViewportChange)
  window.addEventListener('blur', closeSelectFallback)
  window.addEventListener('pagehide', closeSelectFallback)
}

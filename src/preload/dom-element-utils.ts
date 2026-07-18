import type { AgentSnapshotNode } from '../shared/types'
import {
  getInspectableNodeIdForElement,
} from './component-inspector'
import { isPageOverlayTarget } from './gesture-forwarding'

export function elementClasses(element: Element): string[] {
  return [...element.classList]
    .map((name) => name.trim())
    .filter(Boolean)
    .slice(0, 6)
}

export function compactText(value: string | null | undefined, max = 120): string | undefined {
  if (!value) return undefined
  const cleaned = value.replace(/\s+/g, ' ').trim()
  if (!cleaned) return undefined
  return cleaned.length > max ? `${cleaned.slice(0, max - 1)}…` : cleaned
}

export function bestElementName(element: Element): string {
  const html = element as HTMLElement
  const text =
    compactText(html.innerText, 80) ??
    compactText(element.getAttribute('aria-label')) ??
    compactText(element.getAttribute('title')) ??
    compactText((element as HTMLInputElement).value) ??
    compactText(element.getAttribute('placeholder')) ??
    compactText(element.getAttribute('alt'))
  return text ? `${element.tagName.toLowerCase()} "${text}"` : element.tagName.toLowerCase()
}

export function elementSelectorParts(element: Element): { tag: string; remainder: string } {
  const tag = element.tagName.toLowerCase()
  const id = element.getAttribute('id')
  if (id) return { tag, remainder: `#${id}` }
  const role = element.getAttribute('role')
  if (role) return { tag, remainder: `[role="${role}"]` }
  const classes = elementClasses(element)
  if (classes.length) return { tag, remainder: `.${classes.slice(0, 2).join('.')}` }
  return { tag, remainder: '' }
}

function simpleElementSegment(element: Element): string {
  const { tag, remainder } = elementSelectorParts(element)
  return `${tag}${remainder}`
}

/** The descriptor fields shared by the interaction-sync capture bundle and
 *  the resolver's candidate list — same extraction, two wire shapes (one
 *  `undefined`-omitting, one `null`-filled). Keep both derived from here so
 *  they can't drift (ADR 0030). */
export interface LocatorElementDescriptor {
  id: string | null
  testId: string | null
  role: string | null
  name: string | null
  text: string | null
  tag: string
  elementPath: string
  fullPath: string
}

export function describeElementForLocator(element: Element): LocatorElementDescriptor {
  return {
    id: element.getAttribute('id') || null,
    testId: element.getAttribute('data-testid') || null,
    role: element.getAttribute('role') || null,
    name: bestElementName(element) || null,
    text: compactText(element.textContent, 80) ?? null,
    tag: element.tagName.toLowerCase(),
    elementPath: buildElementPath(element, 4),
    fullPath: buildElementPath(element, 10),
  }
}

export function buildElementPath(element: Element, maxDepth: number): string {
  const segments: string[] = []
  let current: Element | null = element
  let depth = 0
  while (current && depth < maxDepth) {
    segments.unshift(simpleElementSegment(current))
    const root = current.getRootNode()
    if (root instanceof ShadowRoot && root.host instanceof Element) {
      segments.unshift('#shadow-root')
      current = root.host
    } else {
      current = current.parentElement
    }
    depth += 1
  }
  return segments.join(' > ')
}

/**
 * A selector that resolves back to *this* element, not merely one shaped like
 * it. `buildElementPath` is a readable 4-deep class path — on a grid or list
 * it matches every sibling card, so re-resolving it lands on the first match.
 * Anything that re-finds an element later (annotation anchors) needs this
 * instead: an nth-of-type chain to the root, unique by construction.
 */
export function buildUniqueSelector(element: Element): string {
  const segments: string[] = []
  let current: Element | null = element
  while (current && current.parentElement) {
    const tag = current.tagName.toLowerCase()
    const id = current.getAttribute('id')
    if (id && current.ownerDocument.querySelectorAll(`#${CSS.escape(id)}`).length === 1) {
      segments.unshift(`#${CSS.escape(id)}`)
      return segments.join(' > ')
    }
    const twins = [...current.parentElement.children].filter(
      (sibling) => sibling.tagName === current!.tagName,
    )
    segments.unshift(twins.length > 1 ? `${tag}:nth-of-type(${twins.indexOf(current) + 1})` : tag)
    current = current.parentElement
  }
  if (current) segments.unshift(current.tagName.toLowerCase())
  return segments.join(' > ')
}

function nearbyElements(element: Element): string[] {
  return [...(element.parentElement?.children ?? [])]
    .filter((candidate) => candidate !== element)
    .slice(0, 3)
    .map((candidate) => bestElementName(candidate))
}

function nearbyText(element: Element): string | undefined {
  const parentText = compactText(element.parentElement?.textContent, 120)
  const ownText = compactText(element.textContent, 120)
  if (parentText && parentText !== ownText) return parentText
  return undefined
}

function accessibilityLines(element: Element): string[] {
  const parts: string[] = []
  const role = element.getAttribute('role')
  const ariaLabel = element.getAttribute('aria-label')
  if (role) parts.push(`role=${role}`)
  if (ariaLabel) parts.push(`aria-label=${ariaLabel}`)
  if ((element as HTMLElement).tabIndex >= 0) parts.push(`tabIndex=${(element as HTMLElement).tabIndex}`)
  return parts
}

function attributePairs(element: Element): Array<{ name: string; value: string }> {
  return [...element.attributes]
    .filter((attribute) =>
      ['id', 'class', 'href', 'src', 'name', 'type', 'role', 'aria-label', 'data-testid'].includes(
        attribute.name,
      ),
    )
    .slice(0, 8)
    .map((attribute) => ({
      name: attribute.name,
      value: compactText(attribute.value, 160) ?? '',
    }))
}

function computedStyleLines(element: Element): string[] {
  const styles = window.getComputedStyle(element)
  return [
    `display=${styles.display}`,
    `position=${styles.position}`,
    `font-family=${styles.fontFamily}`,
    `font-size=${styles.fontSize}`,
    `font-weight=${styles.fontWeight}`,
    `color=${styles.color}`,
    `background=${styles.backgroundColor}`,
    `padding=${styles.padding}`,
    `margin=${styles.margin}`,
  ]
}

export function deepElementFromPoint(x: number, y: number): Element | null {
  let current: Element | null = document.elementFromPoint(x, y)
  while (current) {
    const root = current.shadowRoot
    if (!root) return current
    const nested = root.elementFromPoint(x, y)
    if (!nested || nested === current) return current
    current = nested
  }
  return null
}

/**
 * Pick the deepest *content* element under (x, y), skipping Specular's own
 * page-injected overlays (blocking overlay, comment badges, preview layer,
 * inspect overlay) and drilling into open shadow roots. Used by the comment
 * tool's hover painter and click resolver — without the overlay skip we'd
 * always hit `#__canvas-blocking-overlay` while the page is non-interactive.
 */
export function pickContentElementAtPoint(x: number, y: number): Element | null {
  const stack = document.elementsFromPoint(x, y)
  for (const el of stack) {
    if (isPageOverlayTarget(el)) continue
    let current: Element = el
    while (current.shadowRoot) {
      const nested = current.shadowRoot.elementFromPoint(x, y)
      if (!nested || nested === current) break
      current = nested
    }
    return current
  }
  return null
}

export function inspectionPayload(element: Element) {
  const rect = element.getBoundingClientRect()
  const styles = window.getComputedStyle(element)
  const nodeId = getInspectableNodeIdForElement(element)
  const id =
    nodeId ||
    element.getAttribute('id') ||
    element.getAttribute('data-testid') ||
    `${element.tagName.toLowerCase()}@${Math.round(rect.left)}:${Math.round(rect.top)}`

  return {
    nodeId: id,
    id,
    timestamp: Date.now(),
    tagName: element.tagName.toLowerCase(),
    name: bestElementName(element),
    role: element.getAttribute('role') ?? undefined,
    elementPath: buildElementPath(element, 4),
    fullPath: buildElementPath(element, 10),
    uniqueSelector: buildUniqueSelector(element),
    cssClasses: elementClasses(element),
    textPreview: compactText(element.textContent, 160),
    nearbyText: nearbyText(element),
    nearbyElements: nearbyElements(element),
    accessibility: accessibilityLines(element),
    attributes: attributePairs(element),
    computedStyles: computedStyleLines(element),
    boundingBox: {
      x: Math.round(rect.left),
      y: Math.round(rect.top),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    },
    position: {
      viewportXPercent: window.innerWidth > 0 ? Number(((rect.left / window.innerWidth) * 100).toFixed(1)) : 0,
      documentY: Math.round(rect.top + window.scrollY),
      isFixed: styles.position === 'fixed',
    },
  }
}

export function rectIntersectsRegion(
  box: { left: number; right: number; top: number; bottom: number },
  region: { x: number; y: number; width: number; height: number },
): boolean {
  return !(
    box.right < region.x ||
    box.left > region.x + region.width ||
    box.bottom < region.y ||
    box.top > region.y + region.height
  )
}

export function rectFullyContainedInRegion(
  box: { left: number; right: number; top: number; bottom: number },
  region: { x: number; y: number; width: number; height: number },
): boolean {
  return (
    box.left >= region.x &&
    box.right <= region.x + region.width &&
    box.top >= region.y &&
    box.bottom <= region.y + region.height
  )
}

export function isVisibleForSnapshot(element: Element): boolean {
  const rect = element.getBoundingClientRect()
  if (rect.width === 0 && rect.height === 0) return false
  const styles = window.getComputedStyle(element)
  return styles.display !== 'none' && styles.visibility !== 'hidden'
}

export function isInteractiveForSnapshot(element: Element): boolean {
  const tagName = element.tagName.toLowerCase()
  if (['a', 'button', 'input', 'select', 'textarea', 'summary', 'option', 'label'].includes(tagName)) {
    return true
  }
  const role = element.getAttribute('role')
  if (role && ['button', 'link', 'checkbox', 'textbox', 'menuitem', 'option', 'tab', 'switch'].includes(role)) {
    return true
  }
  if (element.hasAttribute('onclick')) return true
  if ((element as HTMLElement).tabIndex >= 0) return true
  const styles = window.getComputedStyle(element)
  return styles.cursor === 'pointer'
}

function buildStructuredSnapshotNode(
  element: Element,
  depth: number,
  parentRef: string | null,
  nextRefIndex: { value: number },
): AgentSnapshotNode | null {
  if (!isVisibleForSnapshot(element)) return null
  const rect = element.getBoundingClientRect()
  const ref = `@e${nextRefIndex.value}`
  nextRefIndex.value += 1
  return {
    ref,
    parentRef,
    depth,
    tagName: element.tagName.toLowerCase(),
    role: element.getAttribute('role') ?? undefined,
    name: bestElementName(element) || undefined,
    text: compactText(element.textContent, 80) || undefined,
    interactive: isInteractiveForSnapshot(element),
    bounds: {
      x: Math.round(rect.left),
      y: Math.round(rect.top),
      width: Math.round(rect.width),
      height: Math.round(rect.height),
    },
    elementPath: buildElementPath(element, 4),
    fullPath: buildElementPath(element, 10),
  }
}

export function buildStructuredDomSnapshot(maxDepth: number): {
  url: string
  title: string
  nodes: AgentSnapshotNode[]
} {
  const nodes: AgentSnapshotNode[] = []
  const nextRefIndex = { value: 1 }

  function walk(element: Element, depth: number, parentRef: string | null): void {
    if (depth > maxDepth) return
    const node = buildStructuredSnapshotNode(element, depth, parentRef, nextRefIndex)
    if (!node) return
    nodes.push(node)
    for (const child of element.children) {
      walk(child, depth + 1, node.ref)
    }
  }

  walk(document.body, 0, null)
  return {
    url: window.location.href,
    title: document.title,
    nodes,
  }
}

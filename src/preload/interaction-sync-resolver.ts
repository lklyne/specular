import { ipcChannels } from '../shared/ipc-contract'
import { ipcRenderer } from 'electron'
import { resolveLocator } from '../shared/locator-kernel'
import type { LocatorBundle, LocatorCandidate, LocatorResolution } from '../shared/locator-kernel'
import type { LocatorResolveRequest, LocatorResolveResponse } from '../shared/types'
import { describeElementForLocator, isInteractiveForSnapshot, isVisibleForSnapshot } from './dom-element-utils'
import { isPageOverlayTarget } from './gesture-forwarding'

// Mirrors `buildStructuredDomSnapshot`'s depth bound (dom-element-utils.ts):
// invisible subtrees are pruned rather than counted, so this stays cheap even
// on deep pages. A little deeper than the plain snapshot default to leave
// room for open shadow roots nested inside already-deep component trees.
const CANDIDATE_MAX_DEPTH = 12

function toLocatorCandidate(element: Element): LocatorCandidate {
  const rect = element.getBoundingClientRect()
  return {
    ...describeElementForLocator(element),
    interactive: isInteractiveForSnapshot(element),
    rect: { x: rect.left, y: rect.top, width: rect.width, height: rect.height },
  }
}

// Depth + visibility bounded, same as `buildStructuredDomSnapshot`; also
// descends into open shadow roots (closed roots are inaccessible from here
// regardless) and skips Specular's own injected chrome so a synced click can
// never resolve onto our overlay instead of the page underneath it.
function collectCandidates(root: Element, depth: number, out: LocatorCandidate[]): void {
  if (depth > CANDIDATE_MAX_DEPTH) return
  if (isPageOverlayTarget(root)) return
  if (!isVisibleForSnapshot(root)) return
  out.push(toLocatorCandidate(root))
  for (const child of root.children) collectCandidates(child, depth + 1, out)
  if (root.shadowRoot) {
    for (const child of root.shadowRoot.children) collectCandidates(child, depth + 1, out)
  }
}

function enumerateLocatorCandidates(): LocatorCandidate[] {
  // `document.body` is typed non-null by lib.dom, but is genuinely null on
  // early-load and XML/SVG documents — guard rather than let the walk throw.
  if (!document.body) return []
  const candidates: LocatorCandidate[] = []
  collectCandidates(document.body, 0, candidates)
  return candidates
}

// Light-DOM-only depth count (ancestor walk to `document.body`), used solely
// by the identity fast path below: `document.querySelectorAll` never returns
// shadow-root-internal elements, so unlike `collectCandidates` there's no
// shadow-host jump to account for.
function lightDomDepthFromBody(element: Element): number {
  let depth = 0
  let current: Element | null = element
  while (current && current !== document.body) {
    depth += 1
    current = current.parentElement
  }
  return current === document.body ? depth : Number.POSITIVE_INFINITY
}

// A single element carrying the wanted id/data-testid, found by direct
// selector instead of the full tree walk, and passed through the same
// visibility/overlay/depth filters `collectCandidates` applies. Returns null
// on zero or more-than-one survivor so the caller falls back to the full walk
// (which then runs the kernel's own identity fallthrough/ambiguity logic).
//
// Known divergence: `document.querySelectorAll` doesn't pierce shadow roots,
// while the full walk does. A light-DOM-unique id that's duplicated inside an
// open shadow root would fast-path to confident where the walk would call it
// ambiguous. Accepted: duplicate ids across a shadow boundary are already
// invalid-adjacent markup, and this fast path only ever narrows to a *subset*
// of what the walk would find, never invents a match the walk wouldn't make.
function fastPathIdentityCandidate(attribute: 'id' | 'data-testid', value: string): LocatorCandidate | null {
  const selector = `[${attribute}="${CSS.escape(value)}"]`
  let survivor: Element | null = null
  for (const element of document.querySelectorAll(selector)) {
    if (isPageOverlayTarget(element)) continue
    if (!isVisibleForSnapshot(element)) continue
    if (lightDomDepthFromBody(element) > CANDIDATE_MAX_DEPTH) continue
    if (survivor) return null
    survivor = element
  }
  return survivor ? toLocatorCandidate(survivor) : null
}

// When the bundle carries an identity key, try to resolve it without a full
// tree walk. Mirrors the kernel's own key order (id, then testId) and its
// "ambiguous/zero for this key doesn't try the next key" behavior — an
// ambiguous or absent fast-path result falls back to `enumerateLocatorCandidates`
// so the walk's real identity + structural scoring takes over.
function fastPathCandidates(bundle: LocatorBundle): LocatorCandidate[] | null {
  if (bundle.id) {
    const candidate = fastPathIdentityCandidate('id', bundle.id)
    return candidate ? [candidate] : null
  }
  if (bundle.testId) {
    const candidate = fastPathIdentityCandidate('data-testid', bundle.testId)
    return candidate ? [candidate] : null
  }
  return null
}

/**
 * Handle a peer resolve request (ADR 0030 D7): score the bundle against this
 * page's own live DOM and reply immediately. Pure lookup — this never
 * dispatches or mutates anything; that's the CDP dispatcher's job in main,
 * once it trusts a confident resolution.
 *
 * Always sends a response, even on internal failure: a peer that never
 * replies leaves main's pending entry dangling until the caller times out.
 */
export function handleInteractionLocatorResolveRequest(payload: LocatorResolveRequest): void {
  let resolution: LocatorResolution
  try {
    const candidates = fastPathCandidates(payload.bundle) ?? enumerateLocatorCandidates()
    resolution = resolveLocator(payload.bundle, candidates)
  } catch {
    resolution = { kind: 'none' }
  }
  const response: LocatorResolveResponse = { requestId: payload.requestId, resolution }
  ipcRenderer.send(ipcChannels.resolveInteractionLocatorResponse, response)
}

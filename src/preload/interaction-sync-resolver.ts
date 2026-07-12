import { ipcChannels } from '../shared/ipc-contract'
import { ipcRenderer } from 'electron'
import { resolveLocator } from '../shared/locator-kernel'
import type { LocatorCandidate } from '../shared/locator-kernel'
import type { LocatorResolveRequest, LocatorResolveResponse } from '../shared/types'
import {
  bestElementName,
  buildElementPath,
  compactText,
  isInteractiveForSnapshot,
  isVisibleForSnapshot,
} from './dom-element-utils'
import { isPageOverlayTarget } from './gesture-forwarding'

// Mirrors `buildStructuredDomSnapshot`'s depth bound (dom-element-utils.ts):
// invisible subtrees are pruned rather than counted, so this stays cheap even
// on deep pages. A little deeper than the plain snapshot default to leave
// room for open shadow roots nested inside already-deep component trees.
const CANDIDATE_MAX_DEPTH = 12

function toLocatorCandidate(element: Element): LocatorCandidate {
  const rect = element.getBoundingClientRect()
  return {
    id: element.getAttribute('id') || null,
    testId: element.getAttribute('data-testid') || null,
    role: element.getAttribute('role') || null,
    name: bestElementName(element) || null,
    text: compactText(element.textContent, 80) ?? null,
    tag: element.tagName.toLowerCase(),
    elementPath: buildElementPath(element, 4),
    fullPath: buildElementPath(element, 10),
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
  const candidates: LocatorCandidate[] = []
  collectCandidates(document.body, 0, candidates)
  return candidates
}

/**
 * Handle a peer resolve request (ADR 0030 D7): score the bundle against this
 * page's own live DOM and reply immediately. Pure lookup — this never
 * dispatches or mutates anything; that's the CDP dispatcher's job in main,
 * once it trusts a confident resolution.
 */
export function handleInteractionLocatorResolveRequest(payload: LocatorResolveRequest): void {
  const candidates = enumerateLocatorCandidates()
  const resolution = resolveLocator(payload.bundle, candidates)
  const response: LocatorResolveResponse = { requestId: payload.requestId, resolution }
  ipcRenderer.send(ipcChannels.resolveInteractionLocatorResponse, response)
}

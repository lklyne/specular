import type { IncomingMessage } from 'http'
import type {
  AgentSnapshotPage,
  PresenceLabelKey,
  PresenceSurface,
  PresenceTargetQuery,
  PresenceTargetRect,
  PresenceTargetRefSource,
} from '../shared/types'
import { resolvePresencePagePoint } from '../shared/presence-targeting'
import { scoreDescriptorMatch, type DescriptorQuery } from '../shared/locator-kernel'
import { PRESENCE_INTENT_TTL_MS } from '../shared/presence-timing'
import {
  takePageAgentSnapshot,
  queryPageElements,
} from './runtime/page-runtime'
import {
  cacheAgentSnapshot,
  getAgentSnapshot,
  resolveAgentSnapshotNode,
} from './runtime/agent-snapshot-cache'
import {
  getTextEntities,
  getFileEntities,
} from './runtime/document-commands'
import {
  findPageById,
} from './runtime/runtime-context'
import { pageContentSize, projectFramePointToCanvas } from './runtime/runtime-geometry'

import { resolveSession } from './presence-session'
import {
  activePresenceTasks,
  bumpActiveScanId,
  presenceCursors,
  upsertPresenceCursor,
  upsertActivePresenceTask,
  scheduleThinkingState,
  notifyPresenceChanged,
} from './presence-cursor'

// --- Types ---

interface PresenceTargetCandidate {
  ref: string | null
  name: string | null
  text: string | null
  interactive: boolean
  elementPath: string | null
  fullPath: string | null
  bounds: PresenceTargetRect
}

/** One step of a chained browse command's presence queue — coerced and
 *  validated (unlike the wire payload), ready to apply as-is. See
 *  `advancePendingIntent` in routes/session.ts, which pops these one at a
 *  time as each step's own CDP event (or its TTL) consumes the step
 *  before it. */
export interface QueuedPresenceIntent {
  labelKey: PresenceLabelKey
  command: string
  targetRef: string | null
  targetRefSource: PresenceTargetRefSource | null
  targetQuery: PresenceTargetQuery | null
  labelHint: string | null
}

export interface PendingIntent {
  labelKey: PresenceLabelKey
  pageId: string | null
  targetRef: string | null
  targetRefSource: PresenceTargetRefSource | null
  command: string
  receivedAt: number
  expiryTimer: NodeJS.Timeout
  // The remaining steps of a chained browse command, in execution order —
  // empty for a single (unqueued) intent. `taskLabel` rides alongside it so
  // `advancePendingIntent` can re-apply a queued step without needing the
  // original HTTP payload that started the chain.
  queue: QueuedPresenceIntent[]
  taskLabel: string | null
}

// --- State ---

export const pendingIntents = new Map<string, PendingIntent>()
export const PENDING_INTENT_TTL_MS = PRESENCE_INTENT_TTL_MS

// Mirrors MUTATION_VERBS in src/main/shared/browse-handler.ts. Duplicated
// rather than imported: that module pulls in the CLI-side app-client, which
// resolves a session id as a load-time side effect, and this predicate needs
// to run from the main-process CDP bridge, not the CLI process.
const PRESENCE_MUTATING_COMMANDS = new Set(['click', 'fill', 'type', 'select'])

/** Whether a pending intent's command implies a mutating input event
 *  (click/fill/type/select) rather than a read (snapshot, eval, get). Used
 *  to gate box-model-triggered cursor pre-positioning (issue #319) so reads
 *  — which share the same CDP methods — never move the cursor. */
export function isMutatingIntentCommand(command: string): boolean {
  return PRESENCE_MUTATING_COMMANDS.has(command)
}

// --- Derivation helpers ---

export function deriveLabelKey(url: string, method: string): PresenceLabelKey | null {
  if (method === 'GET' && url === '/workspace') return 'scan_workspace'
  if (method === 'POST' && url === '/layout/find-placement') return 'find_placement'
  if (method === 'POST' && (url === '/pages/create' || url === '/pages/create-at-position')) {
    return 'create_page'
  }
  if (method === 'GET' && /^\/pages\/[^/]+\/cdp-target$/.test(url)) return 'attach_page'
  if (method === 'POST' && url === '/selection/select-page') return 'select_page'
  if (method === 'POST' && url === '/annotations') return 'add_annotation'
  if (
    method === 'POST' &&
    (url === '/pages/snapshot' || url === '/pages/agent-snapshot' || url === '/pages/query-elements')
  ) {
    return 'inspect_page'
  }
  if (method === 'GET') return 'read_content'
  return null
}

export function derivePageId(url: string, body: Record<string, unknown>): string | null {
  const match = /^\/pages\/([^/]+)/.exec(url)
  if (match) return decodeURIComponent(match[1])
  if (typeof body.pageId === 'string') return body.pageId
  if (Array.isArray(body.pageIds) && typeof body.pageIds[0] === 'string') {
    return body.pageIds[0]
  }
  return null
}

// --- Canvas position helpers ---

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

export function resolveCanvasPointForPage(
  pageId: string,
  input?: {
    pageX?: number | null
    pageY?: number | null
    targetRect?: PresenceTargetRect | null
  },
): { canvasX: number; canvasY: number } | null {
  const page = findPageById(pageId)
  if (!page) return null
  const { width, height } = pageContentSize(page)
  const point = resolvePresencePagePoint({
    pageX: input?.pageX,
    pageY: input?.pageY,
    targetRect: input?.targetRect ?? null,
    fallbackX: width / 2,
    fallbackY: height / 2,
  })
  // DOM point lives in content coordinates; project to canvas space so
  // the cursor lands on the page body (inside any device-frame bezel).
  const canvas = projectFramePointToCanvas(page, {
    x: clamp(point.x, 0, width),
    y: clamp(point.y, 0, height),
  })
  return { canvasX: canvas.x, canvasY: canvas.y }
}

export function extractCanvasPosition(url: string, body: Record<string, unknown>): { x: number; y: number } | null {
  if (Array.isArray(body.pages) && body.pages.length > 0) {
    const page = body.pages[0] as Record<string, unknown>
    if (typeof page.canvasX === 'number' && typeof page.canvasY === 'number') {
      return { x: page.canvasX, y: page.canvasY }
    }
  }
  if (typeof body.canvasX === 'number' && typeof body.canvasY === 'number') {
    return { x: body.canvasX, y: body.canvasY }
  }
  if (typeof body.canvas_x === 'number' && typeof body.canvas_y === 'number') {
    return { x: body.canvas_x, y: body.canvas_y }
  }
  if (Array.isArray(body.pageIds) && body.pageIds.length > 0) {
    const page = findPageById(body.pageIds[0] as string)
    if (page) return { x: page.canvasX, y: page.canvasY }
  }
  if (typeof body.id === 'string') {
    const textEntity = getTextEntities().find((e) => e.id === body.id)
    if (textEntity) return { x: textEntity.canvasX, y: textEntity.canvasY }
    const fileEntity = getFileEntities().find((e) => e.id === body.id)
    if (fileEntity) return { x: fileEntity.canvasX, y: fileEntity.canvasY }
  }
  return null
}

export function normalizeCanvasPosition(
  position: { x: number; y: number } | { canvasX: number; canvasY: number } | null,
): { x: number; y: number } | null {
  if (!position) return null
  if ('x' in position && 'y' in position) return position
  return { x: position.canvasX, y: position.canvasY }
}

// --- Agent snapshot ---

export function normalizeAgentSnapshot(
  pageId: string,
  payload: unknown,
): AgentSnapshotPage {
  const snapshot = payload as {
    url?: unknown
    title?: unknown
    nodes?: Array<{
      ref?: unknown
      parentRef?: unknown
      depth?: unknown
      tagName?: unknown
      role?: unknown
      name?: unknown
      text?: unknown
      interactive?: unknown
      bounds?: { x?: unknown; y?: unknown; width?: unknown; height?: unknown } | null
      elementPath?: unknown
      fullPath?: unknown
    }>
  }

  return {
    pageId,
    url: typeof snapshot.url === 'string' ? snapshot.url : 'about:blank',
    title: typeof snapshot.title === 'string' ? snapshot.title : '',
    nodes: Array.isArray(snapshot.nodes)
      ? snapshot.nodes.flatMap((node) => {
        if (
          typeof node?.ref !== 'string' ||
          typeof node?.depth !== 'number' ||
          typeof node?.tagName !== 'string' ||
          typeof node?.bounds?.x !== 'number' ||
          typeof node?.bounds?.y !== 'number' ||
          typeof node?.bounds?.width !== 'number' ||
          typeof node?.bounds?.height !== 'number' ||
          typeof node?.interactive !== 'boolean' ||
          typeof node?.elementPath !== 'string' ||
          typeof node?.fullPath !== 'string'
        ) {
          return []
        }
        return [{
          ref: node.ref,
          parentRef: typeof node.parentRef === 'string' ? node.parentRef : null,
          depth: node.depth,
          tagName: node.tagName,
          role: typeof node.role === 'string' ? node.role : undefined,
          name: typeof node.name === 'string' ? node.name : undefined,
          text: typeof node.text === 'string' ? node.text : undefined,
          interactive: node.interactive,
          bounds: {
            x: node.bounds.x,
            y: node.bounds.y,
            width: node.bounds.width,
            height: node.bounds.height,
          },
          elementPath: node.elementPath,
          fullPath: node.fullPath,
        }]
      })
      : [],
  }
}

async function ensureAgentSnapshot(pageId: string): Promise<AgentSnapshotPage> {
  const cached = getAgentSnapshot(pageId)
  if (cached) return cached
  const rawSnapshot = await takePageAgentSnapshot(pageId)
  const snapshot = normalizeAgentSnapshot(pageId, rawSnapshot)
  cacheAgentSnapshot(snapshot)
  return snapshot
}

// --- Target matching ---

function normalizeQueryElementCandidate(candidate: unknown): PresenceTargetCandidate | null {
  if (!candidate || typeof candidate !== 'object') return null
  const payload = candidate as Record<string, unknown>
  const boundingBox =
    payload.boundingBox && typeof payload.boundingBox === 'object'
      ? (payload.boundingBox as Record<string, unknown>)
      : null
  if (
    typeof boundingBox?.x !== 'number' ||
    typeof boundingBox?.y !== 'number' ||
    typeof boundingBox?.width !== 'number' ||
    typeof boundingBox?.height !== 'number'
  ) {
    return null
  }
  return {
    ref: null,
    name: typeof payload.name === 'string' ? payload.name : null,
    text: typeof payload.textPreview === 'string' ? payload.textPreview : null,
    interactive: true,
    elementPath: typeof payload.elementPath === 'string' ? payload.elementPath : null,
    fullPath: typeof payload.fullPath === 'string' ? payload.fullPath : null,
    bounds: {
      x: boundingBox.x,
      y: boundingBox.y,
      width: boundingBox.width,
      height: boundingBox.height,
    },
  }
}

function scorePresenceTargetCandidate(
  candidate: PresenceTargetCandidate,
  query: DescriptorQuery,
): number {
  return scoreDescriptorMatch(
    {
      name: candidate.name,
      text: candidate.text,
      elementPath: candidate.elementPath,
      fullPath: candidate.fullPath,
      interactive: candidate.interactive,
      boundsX: candidate.bounds.x,
      boundsY: candidate.bounds.y,
    },
    query,
  )
}

export async function findPresenceTarget(pageId: string, query: {
  selector?: string | null
  name?: string | null
  text?: string | null
  elementPath?: string | null
  fullPath?: string | null
  interactiveOnly?: boolean
  maxResults?: number
}): Promise<{
  targetRef: string | null
  targetRefSource: PresenceTargetRefSource
  targetName: string | null
  targetRect: PresenceTargetRect
  pageX: number
  pageY: number
} | null> {
  const candidates: PresenceTargetCandidate[] = []

  if (query.selector) {
    const result = await queryPageElements(pageId, query.selector, query.maxResults ?? 20)
    if (Array.isArray(result)) {
      candidates.push(...result.map(normalizeQueryElementCandidate).filter((item): item is PresenceTargetCandidate => Boolean(item)))
    }
  } else {
    const snapshot = await ensureAgentSnapshot(pageId)
    candidates.push(...snapshot.nodes.map((node) => ({
      ref: node.ref,
      name: node.name ?? null,
      text: node.text ?? null,
      interactive: node.interactive,
      elementPath: node.elementPath,
      fullPath: node.fullPath,
      bounds: node.bounds,
    })))
  }

  let best: PresenceTargetCandidate | null = null
  let bestScore = Number.NEGATIVE_INFINITY
  for (const candidate of candidates) {
    const score = scorePresenceTargetCandidate(candidate, query)
    if (score > bestScore) {
      best = candidate
      bestScore = score
    }
  }

  if (!best || !Number.isFinite(bestScore)) return null
  return {
    targetRef: best.ref,
    targetRefSource: 'specular',
    targetName: best.name ?? best.text ?? null,
    targetRect: best.bounds,
    pageX: best.bounds.x + best.bounds.width / 2,
    pageY: best.bounds.y + best.bounds.height / 2,
  }
}

export function resolvePresenceTargetRect(
  pageId: string | null,
  targetRef: string | null,
  targetRefSource: PresenceTargetRefSource | null,
  explicitRect: PresenceTargetRect | null,
): PresenceTargetRect | null {
  if (explicitRect) return explicitRect
  if (targetRefSource === 'agent-browser') return null
  if (!pageId || !targetRef) return null
  return resolveAgentSnapshotNode(pageId, targetRef)?.bounds ?? null
}

// --- Agent-browser ref cache ---
//
// agent-browser's `@eN` refs are opaque to main — resolvePresenceTargetRect
// returns null for them above. `handleBrowse` parses each successful
// `snapshot`'s text output into ref -> {role, name} and POSTs it here
// (routes/session.ts's `/session/presence/agent-browser-refs`), replacing
// the session+page's map wholesale, so the intent handler can synthesize a
// PresenceTargetQuery for a ref it recognizes instead of waiting for the CDP
// proxy to see agent-browser's own box-model query.

interface AgentBrowserRefEntry {
  role: string
  name: string | null
}

// Per-page cap: bounds one pathological huge snapshot. Per-map-count cap:
// bounds total memory across every session+page this process has seen —
// oldest insertion order is a Map's natural iteration order, so evicting the
// first key is a cheap approximate LRU.
const AGENT_BROWSER_REF_MAX_ENTRIES_PER_PAGE = 300
const AGENT_BROWSER_REF_MAX_PAGES = 50

const agentBrowserRefMaps = new Map<string, Map<string, AgentBrowserRefEntry>>()

function agentBrowserRefKey(sessionId: string, pageId: string): string {
  return `${sessionId}::${pageId}`
}

/** Replace a session+page's agent-browser ref map wholesale — called after
 *  every successful `snapshot` (handleBrowse), never merged with what was
 *  there before, since a stale entry is worse than a missing one. */
export function setAgentBrowserRefs(
  sessionId: string,
  pageId: string,
  refs: Array<{ ref: string; role: string; name: string | null }>,
): void {
  const map = new Map<string, AgentBrowserRefEntry>()
  for (const entry of refs.slice(0, AGENT_BROWSER_REF_MAX_ENTRIES_PER_PAGE)) {
    map.set(entry.ref, { role: entry.role, name: entry.name })
  }
  agentBrowserRefMaps.set(agentBrowserRefKey(sessionId, pageId), map)
  while (agentBrowserRefMaps.size > AGENT_BROWSER_REF_MAX_PAGES) {
    const oldestKey = agentBrowserRefMaps.keys().next().value
    if (oldestKey === undefined) break
    agentBrowserRefMaps.delete(oldestKey)
  }
}

/** Drop every session's ref map for a page — the same staleness events that
 *  invalidate main's own agent-snapshot cache (navigation, DOM churn) make
 *  agent-browser's refs just as untrustworthy. Wired in routes/session.ts
 *  via `onAgentSnapshotInvalidated`. */
export function invalidateAgentBrowserRefsForPage(pageId: string): void {
  const suffix = `::${pageId}`
  for (const key of agentBrowserRefMaps.keys()) {
    if (key.endsWith(suffix)) agentBrowserRefMaps.delete(key)
  }
}

/**
 * Synthesize a name-only `PresenceTargetQuery` for an agent-browser `@eN`
 * ref, or null when it isn't safe to. Two conditions gate this:
 *
 * 1. The ref's accessible name is non-empty — an empty name gives
 *    `findPresenceTarget` nothing to match on.
 * 2. That name is unique across every entry in the page's ref map,
 *    regardless of role. `findPresenceTarget`'s snapshot-matching path
 *    (`scorePresenceTargetCandidate`) scores purely on name/text — it never
 *    reads a candidate's role at all — so two same-named elements of
 *    *different* roles are exactly as unresolvable to it as two same-named
 *    links; role can't rescue an otherwise-ambiguous name here.
 *
 * Traveling to the wrong same-named element is worse than not traveling
 * early at all, so any doubt returns null and the caller falls back to the
 * existing box-model-driven pre-move (issue #319).
 */
export function synthesizeAgentBrowserTargetQuery(
  sessionId: string,
  pageId: string,
  ref: string,
): PresenceTargetQuery | null {
  const map = agentBrowserRefMaps.get(agentBrowserRefKey(sessionId, pageId))
  if (!map) return null
  const entry = map.get(ref)
  if (!entry?.name) return null
  let nameMatches = 0
  for (const candidate of map.values()) {
    if (candidate.name === entry.name) nameMatches++
    if (nameMatches > 1) break
  }
  if (nameMatches !== 1) return null
  return { selector: null, text: null, name: entry.name }
}

// --- Orchestrator ---

export function updatePresenceCursor(
  request: IncomingMessage,
  url: string,
  method: string,
  body: Record<string, unknown>,
): void {
  if (url === '/session/presence') return
  if (url === '/session/presence/intent') return
  if (url.startsWith('/mcp/session/')) return

  const resolved = resolveSession(request, body)
  // Scoped to this session so one agent's mutation can never cancel another
  // concurrent session's in-flight scan animation (issue #319 Phase 4).
  if (resolved) bumpActiveScanId(resolved.sessionId)
  const pageId = derivePageId(url, body)
  const labelKey = deriveLabelKey(url, method)
  const existingCursor = resolved ? presenceCursors.get(resolved.sessionId) : null
  const isAttachFrame = labelKey === 'attach_page'
  const preserveSamePagePosition =
    isAttachFrame &&
    pageId !== null &&
    existingCursor?.surface === 'page' &&
    existingCursor.pageId === pageId

  const position = preserveSamePagePosition
    ? { x: existingCursor.canvasX, y: existingCursor.canvasY }
    : normalizeCanvasPosition(
        extractCanvasPosition(url, body) ??
          (pageId ? resolveCanvasPointForPage(pageId) : null),
      )

  upsertPresenceCursor(request, {
    body,
    canvasX: position?.x,
    canvasY: position?.y,
    surface: pageId ? 'page' : 'canvas',
    activity: 'acting',
    pageId,
    labelKey,
  })

  if (resolved && activePresenceTasks.has(resolved.sessionId)) {
    upsertActivePresenceTask(request, {
      body,
      surface: pageId ? 'page' : 'canvas',
      pageId,
      canvasX: position?.x ?? null,
      canvasY: position?.y ?? null,
    })
  }

  scheduleThinkingState(request)
}

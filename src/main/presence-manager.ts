import type { IncomingMessage } from 'http'
import type {
  AgentSnapshotPage,
  PresenceLabelKey,
  PresenceSurface,
  PresenceTargetRect,
  PresenceTargetRefSource,
} from '../shared/types'
import { resolvePresencePagePoint } from '../shared/presence-targeting'
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

export interface PendingIntent {
  labelKey: PresenceLabelKey
  pageId: string | null
  targetRef: string | null
  targetRefSource: PresenceTargetRefSource | null
  command: string
  receivedAt: number
  expiryTimer: NodeJS.Timeout
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

function normalizeSearchText(value: string | null | undefined): string | null {
  if (!value) return null
  const normalized = value.trim().toLowerCase()
  return normalized.length > 0 ? normalized : null
}

function scorePresenceTargetCandidate(
  candidate: PresenceTargetCandidate,
  query: {
    name?: string | null
    text?: string | null
    elementPath?: string | null
    fullPath?: string | null
    interactiveOnly?: boolean
  },
): number {
  if (query.interactiveOnly && !candidate.interactive) return Number.NEGATIVE_INFINITY

  const normalizedName = normalizeSearchText(candidate.name)
  const normalizedText = normalizeSearchText(candidate.text)
  const normalizedElementPath = normalizeSearchText(candidate.elementPath)
  const normalizedFullPath = normalizeSearchText(candidate.fullPath)
  const wantedName = normalizeSearchText(query.name)
  const wantedText = normalizeSearchText(query.text)
  const wantedElementPath = normalizeSearchText(query.elementPath)
  const wantedFullPath = normalizeSearchText(query.fullPath)

  let score = candidate.interactive ? 50 : 0
  let matched = false

  if (wantedName) {
    if (normalizedName === wantedName) {
      score += 400
      matched = true
    } else if (normalizedName?.includes(wantedName)) {
      score += 280
      matched = true
    } else if (normalizedText === wantedName) {
      score += 220
      matched = true
    } else if (normalizedText?.includes(wantedName)) {
      score += 140
      matched = true
    } else {
      return Number.NEGATIVE_INFINITY
    }
  }

  if (wantedText) {
    if (normalizedText === wantedText) {
      score += 320
      matched = true
    } else if (normalizedText?.includes(wantedText)) {
      score += 200
      matched = true
    } else if (normalizedName === wantedText) {
      score += 180
      matched = true
    } else if (normalizedName?.includes(wantedText)) {
      score += 120
      matched = true
    } else {
      return Number.NEGATIVE_INFINITY
    }
  }

  if (wantedElementPath) {
    if (normalizedElementPath === wantedElementPath) {
      score += 260
      matched = true
    } else if (normalizedElementPath?.includes(wantedElementPath)) {
      score += 140
      matched = true
    } else {
      return Number.NEGATIVE_INFINITY
    }
  }

  if (wantedFullPath) {
    if (normalizedFullPath === wantedFullPath) {
      score += 260
      matched = true
    } else if (normalizedFullPath?.includes(wantedFullPath)) {
      score += 140
      matched = true
    } else {
      return Number.NEGATIVE_INFINITY
    }
  }

  if (!matched && (wantedName || wantedText || wantedElementPath || wantedFullPath)) {
    return Number.NEGATIVE_INFINITY
  }

  score += Math.max(0, 100 - candidate.bounds.x * 0.01 - candidate.bounds.y * 0.01)
  return score
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
  bumpActiveScanId()

  const resolved = resolveSession(request, body)
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

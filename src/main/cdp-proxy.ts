import { randomUUID } from 'crypto'
import { webContents } from 'electron'
import { WebSocket, type RawData } from 'ws'
import { DEFAULT_REMOTE_DEBUGGING_PORT } from '../shared/constants'
import type { UiSelection } from '../shared/types'
import { activeSessions } from './presence-session'
import {
  enterGroup as enterSelectionGroup,
  selectEntities as selectSelectionEntities,
  selectEntity as selectSelectionEntity,
  selectNone as clearSelection,
  selectPageById as selectSelectionPageById,
} from './runtime/selection-controller'
import { getUiState } from './ui-state'
import { findPageById } from './runtime/runtime-context'

// --- Constants ---

export const APP_CONTROL_HOST = '127.0.0.1'
const CDP_PROXY_TTL_MS = 5 * 60_000
const REMOTE_DEBUGGING_PORT = Number.parseInt(
  process.env.SPECULAR_REMOTE_DEBUGGING_PORT ?? String(DEFAULT_REMOTE_DEBUGGING_PORT),
  10,
)
const CDP_PROXY_LOG_DEBUG = process.env.SPECULAR_DEBUG_CDP_PROXY === '1'
const CDP_PROXY_TIMING_DEBUG = process.env.SPECULAR_DEBUG_CDP_PROXY_TIMING === '1'

// --- Types ---

interface CdpTargetInfo {
  id: string
  type?: string
  url?: string
  title?: string
  webSocketDebuggerUrl?: string
}

interface CdpVersionInfo {
  webSocketDebuggerUrl?: string
}

export interface PageCdpConnectionInfo {
  pageId: string
  targetId: string
  url: string
  title: string
  browserWebSocketDebuggerUrl: string
}

/** A DOM rect resolved from a live CDP exchange (`DOM.getBoxModel` content
 *  quad, or a rect-shaped `Runtime.callFunctionOn` return value). Always
 *  derived from real page geometry — never predicted or interpolated. */
export interface CdpRect {
  x: number
  y: number
  width: number
  height: number
}

export interface CdpProxyRegistration {
  token: string
  key: string
  pageId: string
  targetId: string
  url: string
  title: string
  browserWebSocketDebuggerUrl: string
  createdAt: number
  updatedAt: number
  lastResolvedAt: number
  sessionId: string | null
  clientName: string | null
  status: 'idle' | 'connecting' | 'open' | 'recovering' | 'closed'
  lastError: string | null
  upstreamSocket: WebSocket | null
  upstreamQueue: string[]
  activeBridge: CdpClientBridge | null
  connectPromise: Promise<WebSocket> | null
  selectionSnapshot: UiSelection | null
  /** Client-issued `DOM.getBoxModel` / `Runtime.callFunctionOn` requests
   *  awaiting their upstream response, keyed by CDP message id, so the
   *  response can be sniffed for a real element rect (issue #319). Capped
   *  and cleared independently of `CdpClientBridge.pendingMethods` — that
   *  map serves unrelated Target.* correlation and isn't meant to carry
   *  presence-specific bookkeeping. */
  pendingRectRequests: Map<number, string>
}

export interface CdpClientBridge {
  clientSocket: WebSocket
  connectedAt: number
  pendingMethods: Map<number, string>
  attachTargetIds: Map<number, string>
  allowedSessionIds: Set<string>
  /** Invoked when a sniffed box-model-shaped response resolves for this
   *  bridge's registration. Set by app-control-server.ts, which owns the
   *  presence-cursor mutation; cdp-proxy.ts only sniffs and extracts. */
  onRectResolved?: (rect: CdpRect, method: string) => void
}

// --- State ---

export const cdpProxyRegistrations = new Map<string, CdpProxyRegistration>()
export const cdpProxyRegistrationsByKey = new Map<string, string>()
export const cdpProxyMetrics = {
  registrationsCreated: 0,
  registrationsReused: 0,
  upstreamConnects: 0,
  upstreamReconnects: 0,
  interceptedClicks: 0,
  interceptedScrolls: 0,
  // Amortization scoreboard (issue #319): how often the pre-act dwell found
  // the cursor already at the target by the time mousePressed arrived, vs.
  // how often it still had to reposition.
  preMoveHits: 0,
  preMoveMisses: 0,
  dwellWaitMsTotal: 0,
  dwellWaitCount: 0,
}

/** Cap on `CdpProxyRegistration.pendingRectRequests` — bounds memory if a
 *  client sends many box-model-style queries whose responses never arrive
 *  (e.g. a crashed agent-browser process). */
const PENDING_RECT_REQUEST_CAP = 32

// --- Logging ---

export function cdpProxyLog(
  category: 'lifecycle' | 'intercept' | 'timing',
  event: string,
  details?: Record<string, unknown>,
): void {
  const enabled =
    category === 'timing'
      ? CDP_PROXY_TIMING_DEBUG
      : CDP_PROXY_LOG_DEBUG
  if (!enabled) return
  console.log(`[cdp-proxy:${category}]`, { ts: Date.now(), event, ...details })
}

// --- Utilities ---

export function cdpProxyKey(sessionId: string | null, pageId: string): string {
  return `${sessionId ?? 'anonymous'}::${pageId}`
}

export function closeSocketQuietly(socket: WebSocket | null | undefined): void {
  if (!socket) return
  if (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING) {
    socket.close()
  }
}

/** Record a client-issued `DOM.getBoxModel` / `Runtime.callFunctionOn`
 *  request awaiting its upstream response, so the response can be
 *  correlated back to a method when it arrives. Drops the oldest entry once
 *  the map exceeds `PENDING_RECT_REQUEST_CAP` — a bounded structure, not an
 *  unbounded ledger. */
export function recordPendingRectRequest(
  registration: CdpProxyRegistration,
  id: number,
  method: string,
): void {
  registration.pendingRectRequests.set(id, method)
  while (registration.pendingRectRequests.size > PENDING_RECT_REQUEST_CAP) {
    const oldestKey = registration.pendingRectRequests.keys().next().value
    if (oldestKey === undefined) break
    registration.pendingRectRequests.delete(oldestKey)
  }
}

/** Parse a `DOM.getBoxModel` or `Runtime.callFunctionOn` CDP response into a
 *  rect, or null if the shape doesn't unambiguously describe one. Pure and
 *  side-effect free so it's unit-testable without a live proxy.
 *
 *  - `DOM.getBoxModel`: the content quad is always rect-shaped (four
 *    corners of the border-box content area), so any well-formed response
 *    yields a rect.
 *  - `Runtime.callFunctionOn`: results here are shared with unrelated
 *    read-path calls (snapshot, eval) that return arbitrary values, so this
 *    only accepts a plain object whose own keys are exactly
 *    `{x, y, width, height}` — anything else is ignored rather than
 *    guessed at. */
export function extractRectFromCdpResult(method: string, result: unknown): CdpRect | null {
  if (!result || typeof result !== 'object') return null

  if (method === 'DOM.getBoxModel') {
    const model = (result as { model?: unknown }).model
    if (!model || typeof model !== 'object') return null
    const content = (model as { content?: unknown }).content
    if (
      !Array.isArray(content) ||
      content.length !== 8 ||
      !content.every((value) => typeof value === 'number' && Number.isFinite(value))
    ) {
      return null
    }
    const xs = [content[0], content[2], content[4], content[6]] as number[]
    const ys = [content[1], content[3], content[5], content[7]] as number[]
    const x = Math.min(...xs)
    const y = Math.min(...ys)
    const width = Math.max(...xs) - x
    const height = Math.max(...ys) - y
    return width > 0 && height > 0 ? { x, y, width, height } : null
  }

  if (method === 'Runtime.callFunctionOn') {
    const inner = (result as { result?: unknown }).result
    if (!inner || typeof inner !== 'object') return null
    const value = (inner as { value?: unknown }).value
    if (!value || typeof value !== 'object' || Array.isArray(value)) return null

    const keys = Object.keys(value as Record<string, unknown>).sort()
    const RECT_KEYS = ['height', 'width', 'x', 'y']
    if (keys.length !== RECT_KEYS.length || !keys.every((key, index) => key === RECT_KEYS[index])) {
      return null
    }

    const rect = value as { x: unknown; y: unknown; width: unknown; height: unknown }
    const { x, y, width, height } = rect
    if (
      typeof x !== 'number' || typeof y !== 'number' ||
      typeof width !== 'number' || typeof height !== 'number' ||
      ![x, y, width, height].every(Number.isFinite)
    ) {
      return null
    }
    return width > 0 && height > 0 ? { x, y, width, height } : null
  }

  return null
}

export function summarizeCdpProxyRegistration(registration: CdpProxyRegistration): Record<string, unknown> {
  return {
    token: registration.token,
    key: registration.key,
    pageId: registration.pageId,
    targetId: registration.targetId,
    url: registration.url,
    title: registration.title,
    sessionId: registration.sessionId,
    clientName: registration.clientName,
    status: registration.status,
    createdAt: registration.createdAt,
    updatedAt: registration.updatedAt,
    lastResolvedAt: registration.lastResolvedAt,
    lastError: registration.lastError,
    hasActiveClient: registration.activeBridge !== null,
    upstreamReadyState: registration.upstreamSocket?.readyState ?? null,
    queuedMessageCount: registration.upstreamQueue.length,
  }
}

export function disposeCdpProxyRegistration(registration: CdpProxyRegistration): void {
  closeSocketQuietly(registration.activeBridge?.clientSocket)
  registration.activeBridge = null
  closeSocketQuietly(registration.upstreamSocket)
  registration.upstreamSocket = null
  registration.connectPromise = null
  registration.status = 'closed'
  registration.pendingRectRequests.clear()
  cdpProxyRegistrations.delete(registration.token)
  cdpProxyRegistrationsByKey.delete(registration.key)
}

// --- Selection restoration ---

function restoreSelectionSnapshot(snapshot: UiSelection): void {
  if (snapshot.kind === 'none') {
    clearSelection()
    return
  }
  if (snapshot.kind === 'single-entity') {
    if (snapshot.entityKind === 'page') {
      selectSelectionPageById(snapshot.entityId)
      return
    }
    selectSelectionEntity(snapshot.entityId, snapshot.entityKind)
    return
  }
  selectSelectionEntities(snapshot.entityIds)
}

export function restoreAutomationSelectionIfNeeded(registration: CdpProxyRegistration): void {
  const snapshot = registration.selectionSnapshot
  if (!snapshot) return
  registration.selectionSnapshot = null

  const current = getUiState().selection
  const currentIsAutomationFrame =
    current.kind === 'single-entity' &&
    current.entityKind === 'page' &&
    current.entityId === registration.pageId

  if (!currentIsAutomationFrame && current.kind !== 'none') {
    return
  }

  restoreSelectionSnapshot(snapshot)
}

// --- Target resolution ---

async function fetchCdpTargets(): Promise<CdpTargetInfo[]> {
  const response = await fetch(`http://${APP_CONTROL_HOST}:${REMOTE_DEBUGGING_PORT}/json`)
  if (!response.ok) {
    throw new Error(`CDP target listing failed with ${response.status}`)
  }
  return await response.json() as CdpTargetInfo[]
}

async function fetchBrowserCdpVersion(): Promise<CdpVersionInfo> {
  const response = await fetch(`http://${APP_CONTROL_HOST}:${REMOTE_DEBUGGING_PORT}/json/version`)
  if (!response.ok) {
    throw new Error(`CDP browser version lookup failed with ${response.status}`)
  }
  return await response.json() as CdpVersionInfo
}

export async function resolvePageCdpConnection(pageId: string): Promise<PageCdpConnectionInfo> {
  const page = findPageById(pageId)
  if (!page) {
    throw new Error('Page not found')
  }

  const pageWebContentsId = page.pageView.webContents.id
  const targets = await fetchCdpTargets()
  const target = targets.find((candidate) => {
    if (!candidate.id || !candidate.webSocketDebuggerUrl) return false
    return webContents.fromDevToolsTargetId(candidate.id)?.id === pageWebContentsId
  })

  if (!target?.webSocketDebuggerUrl) {
    throw new Error('CDP target not found for page')
  }

  const browserVersion = await fetchBrowserCdpVersion()
  if (!browserVersion.webSocketDebuggerUrl) {
    throw new Error('CDP browser target not found')
  }

  return {
    pageId,
    targetId: target.id,
    url: target.url ?? page.pageView.webContents.getURL() ?? 'about:blank',
    title: target.title ?? page.pageView.webContents.getTitle() ?? '',
    browserWebSocketDebuggerUrl: browserVersion.webSocketDebuggerUrl,
  }
}

// --- Registration management ---

export async function refreshCdpProxyRegistration(
  registration: CdpProxyRegistration,
): Promise<CdpProxyRegistration> {
  const next = await resolvePageCdpConnection(registration.pageId)
  const browserTargetChanged =
    registration.browserWebSocketDebuggerUrl !== next.browserWebSocketDebuggerUrl
  registration.targetId = next.targetId
  registration.url = next.url
  registration.title = next.title
  registration.browserWebSocketDebuggerUrl = next.browserWebSocketDebuggerUrl
  registration.lastResolvedAt = Date.now()
  registration.updatedAt = registration.lastResolvedAt
  registration.lastError = null
  if (browserTargetChanged) {
    cdpProxyLog('lifecycle', 'browser-target-changed', {
      token: registration.token,
      pageId: registration.pageId,
    })
    closeSocketQuietly(registration.upstreamSocket)
    registration.upstreamSocket = null
    // Wait for any in-flight connection to complete before clearing the promise.
    // This prevents concurrent ensureCdpProxyUpstream calls from creating a second socket.
    if (registration.connectPromise) {
      await registration.connectPromise.catch(() => {})
    }
    registration.connectPromise = null
    registration.status = 'recovering'
  }
  return registration
}

function flushCdpProxyQueue(registration: CdpProxyRegistration): void {
  if (!registration.upstreamSocket || registration.upstreamSocket.readyState !== WebSocket.OPEN) return
  for (const message of registration.upstreamQueue.splice(0)) {
    registration.upstreamSocket.send(message)
  }
}

export async function ensureCdpProxyUpstream(
  registration: CdpProxyRegistration,
): Promise<WebSocket> {
  if (registration.upstreamSocket?.readyState === WebSocket.OPEN) return registration.upstreamSocket
  if (registration.connectPromise) return registration.connectPromise

  registration.status = registration.upstreamSocket ? 'recovering' : 'connecting'
  registration.lastError = null
  const startedAt = Date.now()
  const existingSocket = registration.upstreamSocket
  if (existingSocket && existingSocket.readyState !== WebSocket.CLOSED) {
    closeSocketQuietly(existingSocket)
  }

  registration.connectPromise = new Promise<WebSocket>((resolve, reject) => {
    const upstreamSocket = new WebSocket(registration.browserWebSocketDebuggerUrl)
    registration.upstreamSocket = upstreamSocket
    if (registration.status === 'recovering') cdpProxyMetrics.upstreamReconnects += 1
    else cdpProxyMetrics.upstreamConnects += 1

    upstreamSocket.once('open', () => {
      registration.status = 'open'
      registration.updatedAt = Date.now()
      registration.connectPromise = null
      flushCdpProxyQueue(registration)
      cdpProxyLog('timing', 'upstream-open', {
        token: registration.token,
        pageId: registration.pageId,
        durationMs: Date.now() - startedAt,
      })
      resolve(upstreamSocket)
    })

    upstreamSocket.on('message', (rawMessage: RawData) => {
      registration.updatedAt = Date.now()
      const bridge = registration.activeBridge
      if (!bridge || bridge.clientSocket.readyState !== WebSocket.OPEN) return
      const text = typeof rawMessage === 'string' ? rawMessage : rawMessage.toString()

      let payload: Record<string, unknown> | null = null
      try {
        payload = JSON.parse(text) as Record<string, unknown>
      } catch {
        bridge.clientSocket.send(text)
        return
      }

      const id = typeof payload.id === 'number' ? payload.id : null
      const method = typeof payload.method === 'string' ? payload.method : null
      const sessionId = typeof payload.sessionId === 'string' ? payload.sessionId : null
      if (sessionId && !bridge.allowedSessionIds.has(sessionId)) return
      if (method && payload.params && typeof payload.params === 'object') {
        const params = payload.params as Record<string, unknown>
        if (!allowCdpTargetEvent(method, params, registration.targetId, bridge.allowedSessionIds)) {
          return
        }
      }

      if (id !== null) {
        const pendingRectMethod = registration.pendingRectRequests.get(id)
        if (pendingRectMethod) {
          registration.pendingRectRequests.delete(id)
          const rect = extractRectFromCdpResult(pendingRectMethod, (payload as { result?: unknown }).result)
          if (rect) bridge.onRectResolved?.(rect, pendingRectMethod)
        }

        const pendingMethod = bridge.pendingMethods.get(id)
        bridge.pendingMethods.delete(id)
        if (pendingMethod === 'Target.getTargets') {
          const result = payload.result as Record<string, unknown> | undefined
          const targetInfos = Array.isArray(result?.targetInfos)
            ? result.targetInfos.filter((item) => {
              const targetInfo = item as Record<string, unknown>
              return targetInfo.targetId === registration.targetId
            })
            : []
          payload = {
            ...payload,
            result: {
              ...(result ?? {}),
              targetInfos,
            },
          }
        }
        if (pendingMethod === 'Target.attachToTarget') {
          const attachedSession = (payload.result as Record<string, unknown> | undefined)?.sessionId
          if (typeof attachedSession === 'string') {
            bridge.allowedSessionIds.add(attachedSession)
          }
        }
      }

      bridge.clientSocket.send(JSON.stringify(payload))
    })

    upstreamSocket.once('close', () => {
      registration.updatedAt = Date.now()
      registration.upstreamSocket = null
      registration.connectPromise = null
      if (registration.status !== 'closed') {
        registration.status = 'recovering'
      }
      cdpProxyLog('lifecycle', 'upstream-closed', {
        token: registration.token,
        pageId: registration.pageId,
        hasActiveClient: registration.activeBridge !== null,
      })
    })

    upstreamSocket.once('error', (error) => {
      registration.lastError = error.message
      registration.connectPromise = null
      registration.status = 'recovering'
      cdpProxyLog('lifecycle', 'upstream-error', {
        token: registration.token,
        pageId: registration.pageId,
        message: error.message,
      })
      reject(error)
    })
  })

  return registration.connectPromise
}

// --- Pruning ---

export function pruneExpiredCdpProxyRegistrations(now = Date.now()): void {
  const activeSessionIds = new Set(activeSessions(now).map((session) => session.id))
  for (const registration of cdpProxyRegistrations.values()) {
    const expired = now - registration.updatedAt > CDP_PROXY_TTL_MS
    const sessionExpired = registration.sessionId !== null && !activeSessionIds.has(registration.sessionId)
    const pageMissing = !findPageById(registration.pageId)
    if (expired || sessionExpired || pageMissing) {
      cdpProxyLog('lifecycle', 'dispose-registration', {
        token: registration.token,
        pageId: registration.pageId,
        reason: expired ? 'ttl' : sessionExpired ? 'session-expired' : 'page-missing',
      })
      disposeCdpProxyRegistration(registration)
    }
  }
}

export function registerPageCdpProxy(
  connection: PageCdpConnectionInfo,
  port: number,
  session: { sessionId: string | null; clientName: string | null },
): {
  pageId: string
  targetId: string
  webSocketDebuggerUrl: string
  url: string
  title: string
} {
  pruneExpiredCdpProxyRegistrations()
  const now = Date.now()
  const key = cdpProxyKey(session.sessionId, connection.pageId)
  const existingToken = cdpProxyRegistrationsByKey.get(key)
  const existing = existingToken ? cdpProxyRegistrations.get(existingToken) : null
  if (existing) {
    existing.targetId = connection.targetId
    existing.url = connection.url
    existing.title = connection.title
    existing.browserWebSocketDebuggerUrl = connection.browserWebSocketDebuggerUrl
    existing.updatedAt = now
    existing.lastResolvedAt = now
    existing.sessionId = session.sessionId
    existing.clientName = session.clientName
    cdpProxyMetrics.registrationsReused += 1
    cdpProxyLog('lifecycle', 'reuse-registration', {
      token: existing.token,
      pageId: existing.pageId,
      sessionId: existing.sessionId,
    })
    return {
      pageId: existing.pageId,
      targetId: existing.targetId,
      webSocketDebuggerUrl: `ws://${APP_CONTROL_HOST}:${port}/cdp/page/${existing.token}`,
      url: existing.url,
      title: existing.title,
    }
  }

  const token = randomUUID()
  const registration: CdpProxyRegistration = {
    token,
    key,
    ...connection,
    createdAt: now,
    updatedAt: now,
    lastResolvedAt: now,
    sessionId: session.sessionId,
    clientName: session.clientName,
    status: 'idle',
    lastError: null,
    upstreamSocket: null,
    upstreamQueue: [],
    activeBridge: null,
    connectPromise: null,
    selectionSnapshot: null,
    pendingRectRequests: new Map(),
  }
  cdpProxyRegistrations.set(token, registration)
  cdpProxyRegistrationsByKey.set(key, token)
  cdpProxyMetrics.registrationsCreated += 1
  cdpProxyLog('lifecycle', 'create-registration', {
    token,
    pageId: connection.pageId,
    sessionId: session.sessionId,
  })
  return {
    pageId: connection.pageId,
    targetId: connection.targetId,
    webSocketDebuggerUrl: `ws://${APP_CONTROL_HOST}:${port}/cdp/page/${token}`,
    url: connection.url,
    title: connection.title,
  }
}

export function allowCdpTargetEvent(method: string, params: Record<string, unknown>, targetId: string, sessionIds: Set<string>): boolean {
  if (method === 'Target.targetCreated' || method === 'Target.targetInfoChanged') {
    const targetInfo = params.targetInfo as Record<string, unknown> | undefined
    return targetInfo?.targetId === targetId
  }
  if (method === 'Target.targetDestroyed') {
    return params.targetId === targetId
  }
  if (method === 'Target.attachedToTarget') {
    const targetInfo = params.targetInfo as Record<string, unknown> | undefined
    const sessionId = typeof params.sessionId === 'string' ? params.sessionId : null
    if (targetInfo?.targetId !== targetId || !sessionId) return false
    sessionIds.add(sessionId)
    return true
  }
  if (method === 'Target.detachedFromTarget') {
    const sessionId = typeof params.sessionId === 'string' ? params.sessionId : null
    if (!sessionId) return false
    const allow = sessionIds.has(sessionId)
    sessionIds.delete(sessionId)
    return allow
  }
  return true
}


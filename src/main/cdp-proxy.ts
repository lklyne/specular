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
}

export interface CdpClientBridge {
  clientSocket: WebSocket
  connectedAt: number
  pendingMethods: Map<number, string>
  attachTargetIds: Map<number, string>
  allowedSessionIds: Set<string>
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
}

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


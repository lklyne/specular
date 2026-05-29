import { randomUUID } from 'crypto'
import { createServer, type IncomingMessage, type Server, type ServerResponse } from 'http'
import { existsSync, readFileSync, writeFileSync, rmSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import type { Duplex } from 'stream'
import { WebSocket, WebSocketServer, type RawData } from 'ws'

import { APP_CONTROL_DISCOVERY_FILE, APP_CONTROL_PORT, APP_CONTROL_VERSION } from '../shared/constants'
import { getUiState } from './ui-state'
import { findPageById, clearAutomationInteractivePageIds, automationInteractivePageCounts, getZoom } from './runtime/runtime-context'
import {
  beginAutomationInteractivePage,
  endAutomationInteractivePage,
  sendInteractiveState,
} from './runtime/overlay-manager'
import { boundIsFillBrowserPage } from './runtime/runtime-geometry'
import {
  activeSessions,
  resolveSession,
  updatePresenceCursor,
  resolveCanvasPointForPage,
  pendingIntents,
  beginPresenceDeparture,
} from './presence-manager'
import {
  upsertPresenceCursor,
  presenceCursors,
  PRESENCE_CURSOR_STEP_DELAY_MS,
  PRESENCE_CURSOR_POSITION_SKIP_PX,
} from './presence-cursor'
import { pagePointMatchesTargetRect } from '../shared/presence-targeting'
import {
  type CdpProxyRegistration,
  type CdpClientBridge,
  APP_CONTROL_HOST,
  cdpProxyRegistrations,
  cdpProxyRegistrationsByKey,
  cdpProxyLog,
  cdpProxyMetrics,
  closeSocketQuietly,
  disposeCdpProxyRegistration,
  restoreAutomationSelectionIfNeeded,
  refreshCdpProxyRegistration,
  ensureCdpProxyUpstream,
  pruneExpiredCdpProxyRegistrations,
} from './cdp-proxy'
import { mcpSessions } from './presence-session'

// Re-export for external consumers
export { getPresenceCursors, onPresenceCursorsChanged } from './presence-manager'
export type { PresenceCursorEntry } from './presence-manager'

// Re-export from http-helpers for callers that import from app-control-server
export type { McpConnectionStatus } from './routes/http-helpers'
export { onMcpConnectionStatusChanged } from './routes/http-helpers'

import {
  writeJson,
  notifyStatusListeners,
  _registerGetStatus,
  _setServerRef,
  type McpConnectionStatus,
} from './routes/http-helpers'

// Register the status getter so routes can call notifyStatusListeners() without
// importing app-control-server (which would recreate the circular dependency).
_registerGetStatus(() => getMcpConnectionStatus())

interface DiscoveryPayload {
  port: number
  secret: string
  version: string
}

let server: Server | null = null
let cdpProxyServer: WebSocketServer | null = null
let secret = ''
const PROBE_TIMEOUT_MS = 1_000

function discoveryFilePath(): string {
  return join(tmpdir(), APP_CONTROL_DISCOVERY_FILE)
}

function removeDiscoveryFile(): void {
  try {
    rmSync(discoveryFilePath(), { force: true })
  } catch (error) {
    console.warn('Failed to remove app control discovery file:', error)
  }
}

async function probeAppControlServer(port: number): Promise<boolean> {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), PROBE_TIMEOUT_MS)
  try {
    const response = await fetch(`http://${APP_CONTROL_HOST}:${port}/health`, {
      signal: controller.signal,
    })
    if (!response.ok) return false
    const payload = (await response.json()) as { version?: unknown }
    return payload.version === APP_CONTROL_VERSION
  } catch {
    return false
  } finally {
    clearTimeout(timer)
  }
}

async function cleanupStaleDiscoveryFile(): Promise<void> {
  if (!existsSync(discoveryFilePath())) return
  try {
    const payload = JSON.parse(readFileSync(discoveryFilePath(), 'utf8')) as Partial<DiscoveryPayload>
    if (typeof payload.port === 'number' && payload.version === APP_CONTROL_VERSION) {
      const active = await probeAppControlServer(payload.port)
      if (active) return
    }
  } catch {
    // Fall through and remove the stale file.
  }
  removeDiscoveryFile()
}

async function readBody(request: IncomingMessage): Promise<unknown> {
  const chunks: Buffer[] = []
  for await (const chunk of request) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk))
  }
  if (!chunks.length) return {}
  return JSON.parse(Buffer.concat(chunks).toString('utf8'))
}

function authorized(request: IncomingMessage): boolean {
  return request.headers['x-specular-secret'] === secret
}


export function getMcpConnectionStatus(): McpConnectionStatus {
  const appServerRunning = server !== null
  const discoveryPresent = existsSync(discoveryFilePath())
  const sessions = activeSessions()
  const lastClientSeenAt =
    sessions.length > 0
      ? new Date(
          Math.max(...sessions.map((session) => session.lastSeenAt)),
        ).toISOString()
      : null

  return {
    // Healthy means the MCP client heartbeat is currently active.
    healthy: appServerRunning && discoveryPresent && sessions.length > 0,
    appServerRunning,
    discoveryFilePresent: discoveryPresent,
    mcpClientConnected: sessions.length > 0,
    activeClientCount: sessions.length,
    lastClientSeenAt,
  }
}

function handleCdpProxyUpgrade(request: IncomingMessage, socket: Duplex, head: Buffer): void {
  if (!cdpProxyServer || !server) {
    socket.destroy()
    return
  }

  const url = request.url ? new URL(request.url, `http://${APP_CONTROL_HOST}`) : null
  const token = url ? /^\/cdp\/page\/([^/]+)$/.exec(url.pathname)?.[1] : null
  if (!token) {
    socket.destroy()
    return
  }

  pruneExpiredCdpProxyRegistrations()
  const registration = cdpProxyRegistrations.get(token)
  if (!registration) {
    socket.destroy()
    return
  }
  registration.updatedAt = Date.now()

  cdpProxyServer.handleUpgrade(request, socket, head, (clientSocket: WebSocket) => {
    cdpProxyServer!.emit('connection', clientSocket, request, registration)
  })
}

import type { Route } from './routes/types'
import { designSystemRoutes } from './routes/design-system'
import { workspaceRoutes } from './routes/workspace'
import { sessionRoutes } from './routes/session'
import { edgesGroupsRoutes } from './routes/edges-groups'
import { recordingRoutes } from './routes/recording'
import { annotationRoutes } from './routes/annotations'
import { entityRoutes } from './routes/entities'
import { pageRoutes } from './routes/pages'
import { stackOrderHttpRoutes } from './routes/stack-order'
import { testRoutes } from './routes/test'

const routes: Route[] = [
  ...pageRoutes,
  ...workspaceRoutes,
  ...sessionRoutes,
  ...edgesGroupsRoutes,
  ...recordingRoutes,
  ...annotationRoutes,
  ...entityRoutes,
  ...stackOrderHttpRoutes,
  ...designSystemRoutes,
  ...testRoutes,
]

async function route(request: IncomingMessage, response: ServerResponse): Promise<void> {
  const url = request.url ?? '/'
  const method = request.method ?? 'GET'

  if (method === 'GET' && url === '/health') {
    writeJson(response, 200, { version: APP_CONTROL_VERSION })
    return
  }

  if (!authorized(request)) {
    writeJson(response, 401, { error: 'Unauthorized' })
    return
  }

  // Track presence for GET requests (no body needed)
  if (method === 'GET') {
    updatePresenceCursor(request, url, method, {})
  }

  try {
    const body = method !== 'GET' ? await readBody(request) : {}

    // Track presence for POST/DELETE requests
    if (method !== 'GET') {
      updatePresenceCursor(request, url, method, body as Record<string, unknown>)
    }

    for (const r of routes) {
      if (r.method !== method) continue

      let params: Record<string, string> = {}
      if (typeof r.pattern === 'string') {
        if (r.pattern !== url) continue
      } else {
        const match = r.pattern.exec(url)
        if (!match) continue
        // Store numbered capture groups
        for (let i = 1; i < match.length; i++) {
          if (match[i] !== undefined) params[String(i - 1)] = match[i]
        }
      }

      await r.handler({ request, response, url, body, params })
      return
    }

    writeJson(response, 404, { error: `Unknown route: ${method} ${url}` })
  } catch (error) {
    writeJson(response, 400, {
      error: error instanceof Error ? error.message : 'Unknown error',
    })
  }
}

export async function startAppControlServer(): Promise<void> {
  if (server) return

  const portOverride = process.env.SPECULAR_PORT ? parseInt(process.env.SPECULAR_PORT, 10) : null
  const effectivePort = portOverride ?? APP_CONTROL_PORT

  await cleanupStaleDiscoveryFile()
  secret = randomUUID()

  const candidateServer = createServer((request, response) => {
    route(request, response).catch((error) => {
      writeJson(response, 500, {
        error: error instanceof Error ? error.message : 'Unexpected server error',
      })
    })
  })
  const candidateCdpProxyServer = new WebSocketServer({ noServer: true })

  candidateCdpProxyServer.on('connection', (clientSocket: WebSocket, request: IncomingMessage, registration: CdpProxyRegistration) => {
    const page = findPageById(registration.pageId)
    if (!page || page.pageView.webContents.isDestroyed()) {
      closeSocketQuietly(clientSocket)
      return
    }

    if (registration.activeBridge) {
      closeSocketQuietly(registration.activeBridge.clientSocket)
      registration.activeBridge = null
    }

    const bridge: CdpClientBridge = {
      clientSocket,
      connectedAt: Date.now(),
      pendingMethods: new Map<number, string>(),
      attachTargetIds: new Map<number, string>(),
      allowedSessionIds: new Set<string>(),
    }
    registration.activeBridge = bridge
    registration.updatedAt = Date.now()
    if (!registration.selectionSnapshot) {
      registration.selectionSnapshot = getUiState().selection
    }
    beginAutomationInteractivePage(registration.pageId)

    // Snapshot emulation scale on mousePressed so mouseReleased uses the
    // same transform even if the user zooms between the two events.
    let clickEmulationScale: number | null = null

    const sendToClient = (message: string): void => {
      if (clientSocket.readyState === WebSocket.OPEN) clientSocket.send(message)
    }

    const sendProtocolError = (id: number | null, message: string): void => {
      if (id === null) return
      sendToClient(JSON.stringify({ id, error: { message } }))
    }

    const pageSessionBody = (): Record<string, unknown> => {
      const sessionBody: Record<string, unknown> = {}
      if (registration.sessionId) sessionBody.sessionId = registration.sessionId
      if (registration.clientName) sessionBody.clientName = registration.clientName
      return sessionBody
    }

    const IPC_TIMEOUT_MS = 5000

    const waitForPresenceDwell = async (sessionId: string | null | undefined): Promise<void> => {
      const cursor = sessionId ? presenceCursors.get(sessionId) : undefined
      const elapsed = cursor ? Date.now() - cursor.lastMoveAt : 0
      const remaining = Math.max(0, PRESENCE_CURSOR_STEP_DELAY_MS - elapsed)
      if (remaining > 0) await new Promise<void>((resolve) => setTimeout(resolve, remaining))
    }

    const sendScrollIpc = (
      wc: Electron.WebContents,
      x: number,
      y: number,
      deltaX: number,
      deltaY: number,
    ): Promise<{ ok: boolean; reason?: string; consumed?: boolean; targetTag?: string; beforeLeft?: number; beforeTop?: number; afterLeft?: number; afterTop?: number }> => {
      const requestId = randomUUID()
      return new Promise((resolve) => {
        const timer = setTimeout(() => {
          wc.ipc.removeAllListeners('dispatch-scroll-result')
          resolve({ ok: false, reason: 'ipc-timeout' })
        }, IPC_TIMEOUT_MS)
        wc.ipc.once(
          'dispatch-scroll-result',
          (_event: Electron.IpcMainEvent, response: { requestId: string; data: { ok: boolean; reason?: string; consumed?: boolean; targetTag?: string; beforeLeft?: number; beforeTop?: number; afterLeft?: number; afterTop?: number } }) => {
            if (response.requestId !== requestId) return
            clearTimeout(timer)
            resolve(response.data)
          },
        )
        wc.send('dispatch-scroll', { requestId, x, y, deltaX, deltaY })
      })
    }

    const emitTypingPresence = async (): Promise<void> => {
      const requestId = randomUUID()
      const target = await new Promise<{
        x: number
        y: number
        width: number
        height: number
        name: string | null
      } | null>((resolve) => {
        const timer = setTimeout(() => {
          page.pageView.webContents.ipc.removeAllListeners(
            'query-active-element-rect-result',
          )
          resolve(null)
        }, IPC_TIMEOUT_MS)
        page.pageView.webContents.ipc.once(
          'query-active-element-rect-result',
          (
            _event: Electron.IpcMainEvent,
            response: {
              requestId: string
              data: {
                x: number
                y: number
                width: number
                height: number
                name: string | null
              } | null
            },
          ) => {
            if (response.requestId !== requestId) return
            clearTimeout(timer)
            resolve(response.data)
          },
        )
        page.pageView.webContents.send('query-active-element-rect', {
          requestId,
        })
      })

      const targetRect =
        target &&
        typeof target.x === 'number' &&
        typeof target.y === 'number' &&
        typeof target.width === 'number' &&
        typeof target.height === 'number'
          ? {
              x: Math.round(target.x),
              y: Math.round(target.y),
              width: Math.round(target.width),
              height: Math.round(target.height),
            }
          : null

      const pageX = targetRect ? targetRect.x + targetRect.width / 2 : undefined
      const pageY = targetRect ? targetRect.y + targetRect.height / 2 : undefined
      const resolved =
        pageX !== undefined && pageY !== undefined
          ? resolveCanvasPointForPage(registration.pageId, { pageX, pageY, targetRect })
          : null

      upsertPresenceCursor(request, {
        body: pageSessionBody(),
        surface: 'page',
        pageId: registration.pageId,
        pageX,
        pageY,
        canvasX: resolved?.canvasX,
        canvasY: resolved?.canvasY,
        activity: 'acting',
        labelKey: 'type_text',
        targetName: target && typeof target.name === 'string' ? target.name : null,
        targetRect,
      })
    }

    const onNavigate = (): void => {
      registration.status = 'recovering'
      registration.updatedAt = Date.now()
      void refreshCdpProxyRegistration(registration).catch((error) => {
        registration.lastError = error instanceof Error ? error.message : 'Failed to refresh target after navigation'
      })
      sendInteractiveState()
    }
    page.pageView.webContents.on('did-finish-load', onNavigate)
    page.pageView.webContents.on('did-navigate-in-page', onNavigate)

    const cleanupClient = (): void => {
      if (page.pageView?.webContents && !page.pageView.webContents.isDestroyed()) {
        page.pageView.webContents.off('did-finish-load', onNavigate)
        page.pageView.webContents.off('did-navigate-in-page', onNavigate)
      }
      if (registration.activeBridge?.clientSocket === clientSocket) {
        registration.activeBridge = null
      }
      registration.updatedAt = Date.now()
      endAutomationInteractivePage(registration.pageId)
      restoreAutomationSelectionIfNeeded(registration)

      // If this was the last active CDP bridge for the session, treat the
      // transport drop as a session departure. Catches CLI crashes and
      // ungraceful exits that never send /mcp/session/close.
      const sessionId = registration.sessionId
      if (sessionId) {
        let hasOtherBridge = false
        for (const other of cdpProxyRegistrations.values()) {
          if (other === registration) continue
          if (other.sessionId === sessionId && other.activeBridge) {
            hasOtherBridge = true
            break
          }
        }
        if (!hasOtherBridge) {
          mcpSessions.delete(sessionId)
          notifyStatusListeners()
          beginPresenceDeparture(sessionId)
        }
      }
    }

    clientSocket.on('message', async (rawMessage: RawData) => {
      const text = typeof rawMessage === 'string' ? rawMessage : rawMessage.toString()
      let payload: Record<string, unknown>
      try {
        payload = JSON.parse(text) as Record<string, unknown>
      } catch {
        return
      }

      const id = typeof payload.id === 'number' ? payload.id : null
      const method = typeof payload.method === 'string' ? payload.method : null
      const params = payload.params && typeof payload.params === 'object'
        ? { ...(payload.params as Record<string, unknown>) }
        : undefined


      if (id !== null && method) {
        bridge.pendingMethods.set(id, method)
        if (method === 'Target.attachToTarget') {
          bridge.attachTargetIds.set(id, typeof params?.targetId === 'string' ? params.targetId : '')
        }
      }

      if ((method === 'Target.attachToTarget' || method === 'Target.activateTarget') && params) {
        try {
          const startedAt = Date.now()
          await refreshCdpProxyRegistration(registration)
          cdpProxyLog('timing', 'refresh-target-before-attach', {
            token: registration.token,
            pageId: registration.pageId,
            durationMs: Date.now() - startedAt,
          })
        } catch (error) {
          sendProtocolError(id, error instanceof Error ? error.message : 'Unable to refresh page target')
          return
        }
      }

      if (params && typeof params.targetId === 'string' && params.targetId !== registration.targetId) {
        if (method === 'Target.attachToTarget' || method === 'Target.activateTarget') {
          params.targetId = registration.targetId
        }
      }

      // Mouse events: update presence cursor and intent delay, then dispatch
      // via Electron's debugger API. Coordinates from CDP clients are in the
      // emulated CSS viewport space, but Input.dispatchMouseEvent (both via
      // wc.debugger and upstream Chromium) interprets them in the pre-scale
      // physical view space. Divide by the emulation scale to compensate.
      if (method === 'Input.dispatchMouseEvent' && params && (params.type as string) !== 'mouseWheel' && page && !page.pageView.webContents.isDestroyed()) {
        const cdpType = params.type as string
        if (cdpType === 'mousePressed' || cdpType === 'mouseReleased' || cdpType === 'mouseMoved') {
          const x = params.x as number
          const y = params.y as number
          const resolved = resolveCanvasPointForPage(registration.pageId, { pageX: x, pageY: y })
          if (resolved) {
            // On mousePressed, if the intent already placed the cursor at the
            // target, suppress the reposition — otherwise the click lands
            // mid-animation instead of on the dwelled cursor.
            let skipPosition = false
            if (cdpType === 'mousePressed') {
              const existing = registration.sessionId
                ? presenceCursors.get(registration.sessionId)
                : undefined
              if (existing && existing.pageId === registration.pageId) {
                const rect = existing.targetRect
                const withinTargetRect =
                  rect != null && pagePointMatchesTargetRect(x, y, rect, 0)
                const canvasDistance = Math.hypot(
                  resolved.canvasX - existing.canvasX,
                  resolved.canvasY - existing.canvasY,
                )
                skipPosition =
                  withinTargetRect ||
                  canvasDistance < PRESENCE_CURSOR_POSITION_SKIP_PX
              }
            }
            upsertPresenceCursor(request, {
              body: pageSessionBody(),
              surface: 'page',
              pageId: registration.pageId,
              // On skip, preserve all rendering-input fields (page coords,
              // targetRect, targetRef/Name). canvas-layout-data recomputes
              // page-cursor canvasX/Y from pageX/Y or targetRect; clearing
              // both would snap the cursor to page center.
              ...(skipPosition
                ? {}
                : {
                    pageX: x,
                    pageY: y,
                    canvasX: resolved.canvasX,
                    canvasY: resolved.canvasY,
                    targetRef: null,
                    targetRefSource: null,
                    targetName: null,
                    targetRect: null,
                  }),
              activity: 'acting',
              labelKey: cdpType === 'mouseMoved' ? undefined : 'click_target',
            })
          }
          if (cdpType === 'mousePressed') {
            const intentSessionId = registration.sessionId ?? ''
            const intent = pendingIntents.get(intentSessionId)
            if (intent) {
              clearTimeout(intent.expiryTimer)
              pendingIntents.delete(intentSessionId)
            }
            // Budget the pre-click dwell from the cursor's last reposition,
            // not from intent arrival — otherwise cold-start / scrollIntoView
            // can consume the head-start before the cursor finishes moving.
            await waitForPresenceDwell(registration.sessionId)
          }

          // DOM.getBoxModel returns CSS viewport coords; Input.dispatchMouseEvent
          // expects physical view coords. With enableDeviceEmulation({ scale }),
          // Chromium maps physical→CSS by dividing by scale. So to hit a CSS
          // target at (x, y), we send (x * scale, y * scale).
          // Snapshot scale on mousePressed; reuse for mouseReleased so a
          // mid-click zoom change doesn't split the pair across scales.
          if (cdpType === 'mousePressed') {
            clickEmulationScale = boundIsFillBrowserPage(page) ? 1 : getZoom()
          }
          const emulationScale = clickEmulationScale ?? (boundIsFillBrowserPage(page) ? 1 : getZoom())
          if (cdpType === 'mouseReleased') {
            clickEmulationScale = null
          }
          const origX = params.x as number
          const origY = params.y as number
          if (emulationScale !== 1) {
            params.x = origX * emulationScale
            params.y = origY * emulationScale
          }

          const wc = page.pageView.webContents
          try {
            if (!wc.debugger.isAttached()) wc.debugger.attach('1.3')

            await wc.debugger.sendCommand('Input.dispatchMouseEvent', params)
            cdpProxyMetrics.interceptedClicks += cdpType === 'mouseMoved' ? 0 : 1

            if (id !== null) sendToClient(JSON.stringify({ id, result: {} }))
          } catch (error) {
            sendProtocolError(id, error instanceof Error ? error.message : 'Mouse dispatch failed')
          }
          return
        }
      }

      if (method === 'Input.dispatchKeyEvent' && params && page && !page.pageView.webContents.isDestroyed()) {
        const wc = page.pageView.webContents
        try {
          await emitTypingPresence()
          if (!wc.debugger.isAttached()) wc.debugger.attach('1.3')
          await wc.debugger.sendCommand('Input.dispatchKeyEvent', params)
          if (id !== null) sendToClient(JSON.stringify({ id, result: {} }))
        } catch (error) {
          sendProtocolError(id, error instanceof Error ? error.message : 'Key dispatch failed')
        }
        return
      }

      if (method === 'Input.insertText' && params && page && !page.pageView.webContents.isDestroyed()) {
        const wc = page.pageView.webContents
        try {
          await emitTypingPresence()
          if (!wc.debugger.isAttached()) wc.debugger.attach('1.3')
          await wc.debugger.sendCommand('Input.insertText', params)
          if (id !== null) sendToClient(JSON.stringify({ id, result: {} }))
        } catch (error) {
          sendProtocolError(id, error instanceof Error ? error.message : 'Insert text failed')
        }
        return
      }

      if (method === 'Input.dispatchMouseEvent' && params && (params.type as string) === 'mouseWheel' && page && !page.pageView.webContents.isDestroyed()) {
        const x = params.x as number
        const y = params.y as number
        const deltaX = typeof params.deltaX === 'number' ? params.deltaX : 0
        const deltaY = typeof params.deltaY === 'number' ? params.deltaY : 0
        upsertPresenceCursor(request, {
          body: pageSessionBody(),
          surface: 'page',
          pageId: registration.pageId,
          pageX: x,
          pageY: y,
          activity: 'acting',
          labelKey: 'scroll_page',
        })
        await waitForPresenceDwell(registration.sessionId)
        try {
          const startedAt = Date.now()
          const result = await sendScrollIpc(
            page.pageView.webContents,
            x,
            y,
            deltaX,
            deltaY,
          )
          if (!result?.ok) {
            sendProtocolError(id, typeof result?.reason === 'string' ? result.reason : 'Scroll target not found')
            return
          }
          if (!result?.consumed) {
            sendProtocolError(id, 'Scroll gesture was not consumed')
            return
          }
          cdpProxyMetrics.interceptedScrolls += 1
          cdpProxyLog('timing', 'intercept-scroll-wheel', {
            token: registration.token,
            pageId: registration.pageId,
            durationMs: Date.now() - startedAt,
          })
          if (id !== null) sendToClient(JSON.stringify({ id, result: {} }))
        } catch (error) {
          sendProtocolError(id, error instanceof Error ? error.message : 'Scroll dispatch failed')
        }
        return
      }

      if (method === 'Input.synthesizeScrollGesture' && params && page && !page.pageView.webContents.isDestroyed()) {
        const x = typeof params.x === 'number' ? params.x : 0
        const y = typeof params.y === 'number' ? params.y : 0
        const xDistance = typeof params.xDistance === 'number' ? params.xDistance : 0
        const yDistance = typeof params.yDistance === 'number' ? params.yDistance : 0
        upsertPresenceCursor(request, {
          body: pageSessionBody(),
          surface: 'page',
          pageId: registration.pageId,
          pageX: x,
          pageY: y,
          activity: 'acting',
          labelKey: 'scroll_page',
        })
        await waitForPresenceDwell(registration.sessionId)
        try {
          const startedAt = Date.now()
          const result = await sendScrollIpc(
            page.pageView.webContents,
            x,
            y,
            -xDistance,
            -yDistance,
          )
          if (!result?.ok) {
            sendProtocolError(id, typeof result?.reason === 'string' ? result.reason : 'Scroll target not found')
            return
          }
          if (!result?.consumed) {
            sendProtocolError(id, 'Scroll gesture was not consumed')
            return
          }
          cdpProxyMetrics.interceptedScrolls += 1
          cdpProxyLog('timing', 'intercept-scroll-gesture', {
            token: registration.token,
            pageId: registration.pageId,
            durationMs: Date.now() - startedAt,
          })
          if (id !== null) sendToClient(JSON.stringify({ id, result: {} }))
        } catch (error) {
          sendProtocolError(id, error instanceof Error ? error.message : 'Scroll gesture failed')
        }
        return
      }

      try {
        await ensureCdpProxyUpstream(registration)
      } catch (error) {
        sendProtocolError(id, error instanceof Error ? error.message : 'Unable to connect to upstream browser target')
        return
      }

      const serialized = JSON.stringify(params ? { ...payload, params } : payload)
      if (registration.upstreamSocket?.readyState === WebSocket.OPEN) {
        registration.upstreamSocket.send(serialized)
      } else {
        registration.upstreamQueue.push(serialized)
      }
      registration.updatedAt = Date.now()
    })

    clientSocket.on('close', cleanupClient)
    clientSocket.on('error', cleanupClient)

    void ensureCdpProxyUpstream(registration).catch((error) => {
      registration.lastError = error instanceof Error ? error.message : 'Unable to establish upstream browser target'
      sendProtocolError(null, registration.lastError)
    })
  })

  candidateServer.on('upgrade', (request, socket, head) => {
    handleCdpProxyUpgrade(request, socket, head)
  })

  const started = await new Promise<boolean>((resolve) => {
    const onError = (error: NodeJS.ErrnoException): void => {
      candidateServer.off('listening', onListening)
      if (error.code === 'EADDRINUSE') {
        resolve(false)
        return
      }
      console.error('[app-control] failed to start server:', error)
      resolve(false)
    }
    const onListening = (): void => {
      candidateServer.off('error', onError)
      resolve(true)
    }
    candidateServer.once('error', onError)
    candidateServer.once('listening', onListening)
    candidateServer.listen(effectivePort, APP_CONTROL_HOST)
  })

  if (!started) {
    const activeAppServer = await probeAppControlServer(effectivePort)
    if (activeAppServer) {
      console.warn(
        `[app-control] port ${effectivePort} already has an active Specular app-control server`,
      )
    } else {
      removeDiscoveryFile()
      console.error(
        `[app-control] port ${effectivePort} is already in use by another process`,
      )
    }
    notifyStatusListeners()
    return
  }

  server = candidateServer
  _setServerRef(server)
  cdpProxyServer = candidateCdpProxyServer
  const payload: DiscoveryPayload = {
    port: effectivePort,
    secret,
    version: APP_CONTROL_VERSION,
  }
  writeFileSync(discoveryFilePath(), JSON.stringify(payload, null, 2), 'utf8')
  console.log(`[app-control] listening on http://${APP_CONTROL_HOST}:${effectivePort}`)
  notifyStatusListeners()
}

export function stopAppControlServer(): void {
  if (!server) return
  cdpProxyServer?.close()
  cdpProxyServer = null
  server.close()
  server = null
  _setServerRef(null)
  for (const registration of [...cdpProxyRegistrations.values()]) {
    disposeCdpProxyRegistration(registration)
  }
  cdpProxyRegistrations.clear()
  cdpProxyRegistrationsByKey.clear()
  clearAutomationInteractivePageIds()
  mcpSessions.clear()
  removeDiscoveryFile()
  notifyStatusListeners()
}

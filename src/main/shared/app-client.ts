import { randomUUID } from 'crypto'
import { readFileSync, writeFileSync } from 'fs'
import { isAbsolute, join } from 'path'
import { homedir, tmpdir } from 'os'
import {
  APP_CONTROL_DISCOVERY_FILE,
  APP_CONTROL_VERSION,
} from '../../shared/constants'

// ---------------------------------------------------------------------------
// Discovery
// ---------------------------------------------------------------------------

interface DiscoveryPayload {
  port: number
  secret: string
  version: string
}

// Mirror the resolver in src/main/app-control-server.ts: SPECULAR_DISCOVERY_FILE
// lets test instances talk to a private discovery file instead of the canonical
// one shared by the dev/production app. The default is home-relative (not
// tmpdir()) so the CLI finds the app even when run inside a tool sandbox that
// overrides TMPDIR (e.g. Claude Code's /tmp/claude-<uid>).
function discoveryFilePath(): string {
  const override = process.env.SPECULAR_DISCOVERY_FILE
  if (override) return isAbsolute(override) ? override : join(tmpdir(), override)
  return join(homedir(), '.specular', APP_CONTROL_DISCOVERY_FILE)
}

function loadDiscovery(): DiscoveryPayload {
  try {
    const payload = JSON.parse(
      readFileSync(discoveryFilePath(), 'utf8'),
    ) as DiscoveryPayload
    if (payload.version !== APP_CONTROL_VERSION) {
      throw new Error(
        `App control API version mismatch. Expected ${APP_CONTROL_VERSION}, got ${payload.version}.`,
      )
    }
    return payload
  } catch (error) {
    throw new Error(
      `Specular app is not available. Launch the app first. ${error instanceof Error ? error.message : ''}`.trim(),
    )
  }
}

// ---------------------------------------------------------------------------
// Session identity
// ---------------------------------------------------------------------------

function resolveSessionId(): string {
  // Explicit override takes priority
  if (process.env.SPECULAR_SESSION_ID) return process.env.SPECULAR_SESSION_ID

  // Fixed session file — all CLI calls share one session ID.
  // Server-side 10s expiry clears the cursor after the last call.
  const sessionFile = join(tmpdir(), 'specular-session.id')
  try {
    return readFileSync(sessionFile, 'utf8').trim()
  } catch {
    const id = randomUUID()
    try { writeFileSync(sessionFile, id, 'utf8') } catch { /* best-effort */ }
    return id
  }
}

export const sessionId = resolveSessionId()
// SPECULAR_CLIENT_NAME wins over setClientName so parallel agents can send
// distinct names — otherwise presence-cursor.ts evicts cursors sharing a
// clientName, leaving only one visible at a time.
const CLIENT_NAME_OVERRIDE = process.env.SPECULAR_CLIENT_NAME
let clientName = CLIENT_NAME_OVERRIDE ?? 'specular-mcp'

export function setClientName(name: string): void {
  if (CLIENT_NAME_OVERRIDE !== undefined) {
    console.warn(
      `[app-client] setClientName(${JSON.stringify(name)}) ignored; SPECULAR_CLIENT_NAME=${JSON.stringify(CLIENT_NAME_OVERRIDE)} has precedence`,
    )
    return
  }
  clientName = name
}

export function getClientName(): string {
  return clientName
}

// ---------------------------------------------------------------------------
// HTTP client
// ---------------------------------------------------------------------------

export async function callApp<T>(path: string, init?: RequestInit): Promise<T> {
  const discovery = loadDiscovery()
  const response = await fetch(`http://127.0.0.1:${discovery.port}${path}`, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      'x-specular-secret': discovery.secret,
      'x-specular-session-id': sessionId,
      'x-specular-client-name': clientName,
      ...(init?.headers ?? {}),
    },
  })
  const payload = (await response.json()) as T & { error?: string }
  if (!response.ok) {
    if (response.status === 401) {
      // The discovery file's secret no longer matches the server answering on
      // its port — typically a stale file left by a restarted app or a test
      // instance. Relaunching the app rewrites the file with a fresh secret.
      throw new Error(
        'Specular rejected the request (stale credentials). The discovery file points at a server with a different secret — quit any extra Specular instances and relaunch the app, then retry.',
      )
    }
    throw new Error(payload.error ?? `Request failed with ${response.status}`)
  }
  return payload
}

// ---------------------------------------------------------------------------
// Session lifecycle
// ---------------------------------------------------------------------------

export async function notifySessionState(
  path: '/mcp/session/open' | '/mcp/session/ping' | '/mcp/session/close',
): Promise<void> {
  try {
    await callApp(path, {
      method: 'POST',
      body: JSON.stringify({
        sessionId,
        clientName,
      }),
    })
  } catch {
    // Ignore bookkeeping failures so the caller remains usable.
  }
}

let heartbeatTimer: NodeJS.Timeout | null = null

export function startHeartbeat(): void {
  if (heartbeatTimer) return
  heartbeatTimer = setInterval(() => {
    void notifySessionState('/mcp/session/ping')
  }, 5_000)
}

export function stopHeartbeat(): void {
  if (!heartbeatTimer) return
  clearInterval(heartbeatTimer)
  heartbeatTimer = null
}

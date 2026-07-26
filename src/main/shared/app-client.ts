import { randomUUID } from 'crypto'
import { execFileSync } from 'child_process'
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

const MAX_ANCESTOR_WALK_DEPTH = 15
// Matches the agent CLI host process whose lifetime spans one conversation —
// every Bash call and MCP subprocess the conversation spawns descends from
// it, so its pid is a stable per-conversation identity. Extend this pattern
// (not the depth bound) to recognize another agent CLI.
const AGENT_HOST_COMMAND_PATTERN = /\bclaude\b/i

export interface ProcessAncestorInfo {
  ppid: number
  command: string
}

/** Reads a process's parent pid and command name. Platform-specific and
 *  side-effecting (shells out / reads /proc); injected into
 *  `findAgentHostAncestorId` so the walk itself is a pure function of its
 *  inputs and testable without spawning real processes. */
export type ParentPidReader = (pid: number) => ProcessAncestorInfo | null

function readParentPidDarwin(pid: number): ProcessAncestorInfo | null {
  try {
    const out = execFileSync('ps', ['-o', 'ppid=,comm=', '-p', String(pid)], {
      encoding: 'utf8',
    }).trim()
    if (!out) return null
    const match = out.match(/^(\d+)\s+(.+)$/)
    if (!match) return null
    return { ppid: Number(match[1]), command: match[2] }
  } catch {
    return null
  }
}

function readParentPidLinux(pid: number): ProcessAncestorInfo | null {
  try {
    const stat = readFileSync(`/proc/${pid}/stat`, 'utf8')
    // Format: "pid (comm) state ppid ...". comm is parenthesized and may
    // itself contain spaces or parens, so split on the *last* ')'.
    const closeParen = stat.lastIndexOf(')')
    if (closeParen === -1) return null
    const command = stat.slice(stat.indexOf('(') + 1, closeParen)
    const rest = stat.slice(closeParen + 2).trim().split(/\s+/)
    const ppid = Number(rest[1])
    if (!Number.isFinite(ppid)) return null
    return { ppid, command }
  } catch {
    return null
  }
}

function defaultParentPidReader(pid: number): ProcessAncestorInfo | null {
  return process.platform === 'linux' ? readParentPidLinux(pid) : readParentPidDarwin(pid)
}

/**
 * Walk the parent-PID chain from `startPid` looking for the nearest ancestor
 * whose command matches the invoking agent CLI. All Bash calls and MCP
 * subprocesses spawned within one agent conversation share that ancestor, so
 * `ancestor-<pid>` groups them into a single presence session — while two
 * concurrent conversations (two different agent-host processes) resolve to
 * two different ids. Returns null if no matching ancestor is found within
 * `maxDepth` (bounded so a detached/reparented process can't walk to pid 1).
 */
export function findAgentHostAncestorId(
  startPid: number,
  readInfo: ParentPidReader = defaultParentPidReader,
  maxDepth: number = MAX_ANCESTOR_WALK_DEPTH,
): string | null {
  let pid = startPid
  for (let depth = 0; depth < maxDepth; depth++) {
    const info = readInfo(pid)
    if (!info) return null
    if (AGENT_HOST_COMMAND_PATTERN.test(info.command)) return `ancestor-${pid}`
    if (!Number.isFinite(info.ppid) || info.ppid <= 1 || info.ppid === pid) return null
    pid = info.ppid
  }
  return null
}

function resolveSessionId(): string {
  // Explicit override takes priority
  if (process.env.SPECULAR_SESSION_ID) return process.env.SPECULAR_SESSION_ID

  // One process tree per agent conversation — group by the nearest
  // recognizable agent-CLI ancestor so many short-lived Bash/MCP
  // invocations within a conversation read as one cursor, while two
  // concurrent conversations (distinct ancestors) don't collide.
  const ancestorId = findAgentHostAncestorId(process.pid)
  if (ancestorId) return ancestorId

  // Fallback for processes with no recognizable agent-CLI ancestor (e.g. run
  // directly by a human): a fixed session file, so all such calls share one
  // ID. Server-side 10s expiry clears the cursor after the last call.
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
// Tab targeting
// ---------------------------------------------------------------------------

// The `--tab` ref for this process, set once from the parsed args. It rides
// every request as a header so the main side can resolve it in one place
// rather than each verb re-deriving a target (issue #360 §3). Encoded because
// tab names are free text and headers are latin-1.
let targetTabRef: string | null = null

export function setTargetTabRef(ref: string | null): void {
  targetTabRef = ref && ref.trim() ? ref.trim() : null
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
      ...(targetTabRef ? { 'x-specular-tab': encodeURIComponent(targetTabRef) } : {}),
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

/**
 * Dev-server manager.
 *
 * Owns the runtime side of "which Vite dev servers are connected to Specular."
 * Each connected repo persists as `{id, absolutePath, label}` in
 * userData/repos.json so connections survive across app launches.
 *
 * The actual `vite dev` child process is started lazily — only when
 * something asks `urlForComponent(repoId, ...)` — so quitting and reopening
 * Specular doesn't immediately spawn a flotilla of dev servers.
 *
 * Designed for testability: pass `{ userDataDir, spawn }` to `initDevServerManager`
 * to inject a tmpdir + a fake spawner. In production, src/main/index.ts wires
 * Electron's `app.getPath('userData')` and node's `child_process.spawn`.
 */

import { existsSync, mkdirSync, readFileSync, writeFileSync, renameSync } from 'node:fs'
import { join } from 'node:path'
import { createHash } from 'node:crypto'
import { EventEmitter } from 'node:events'
import type { ChildProcess } from 'node:child_process'
import type {
  ConnectedRepo,
  OriginBinding,
  OriginBindings,
  RepoOriginBinding,
  RepoStatus,
} from '../../shared/types'

export type { ConnectedRepo, RepoStatus }

export interface PersistedRepo {
  id: string
  absolutePath: string
  label: string
  boundOrigins?: RepoOriginBinding[]
}

/** Legacy preferences shape. Folded into repos at startup, then discarded. */
export interface LegacyOriginBindings {
  [origin: string]: { repoPath: string; autoFix: boolean }
}

export type SpawnFn = (
  command: string,
  args: readonly string[],
  options: { cwd: string },
) => ChildProcess

export interface InitOptions {
  userDataDir: string
  spawn: SpawnFn
}

interface InternalRepo extends ConnectedRepo {
  child: ChildProcess | null
  startupResolvers: Array<(url: string | null) => void>
  startupTimer: NodeJS.Timeout | null
}

function normalizeOrigin(input: string): string {
  try {
    return new URL(input).origin
  } catch {
    return input.replace(/\/+$/, '')
  }
}

const repos = new Map<string, InternalRepo>()
const events = new EventEmitter()
let userDataDir: string | null = null
let spawnFn: SpawnFn | null = null

const LOCAL_URL_REGEX = /Local:\s+(https?:\/\/[^\s/]+(?::\d+)?\/?)/i
const STARTUP_TIMEOUT_MS = 30_000

function reposFile(): string {
  if (!userDataDir) throw new Error('dev-server-manager not initialized')
  return join(userDataDir, 'repos.json')
}

function repoIdFor(absolutePath: string): string {
  return createHash('sha256').update(absolutePath).digest('hex').slice(0, 16)
}

function basename(absolutePath: string): string {
  const trimmed = absolutePath.replace(/\/+$/, '')
  return trimmed.split('/').pop() ?? trimmed
}

function persist(): void {
  const payload: { repos: PersistedRepo[] } = {
    repos: Array.from(repos.values()).map((r) => ({
      id: r.id,
      absolutePath: r.absolutePath,
      label: r.label,
      boundOrigins: r.boundOrigins.length > 0 ? r.boundOrigins : undefined,
    })),
  }
  if (!userDataDir) return
  if (!existsSync(userDataDir)) mkdirSync(userDataDir, { recursive: true })
  const file = reposFile()
  const tmp = `${file}.tmp`
  writeFileSync(tmp, JSON.stringify(payload, null, 2), 'utf8')
  renameSync(tmp, file)
}

function loadPersisted(): void {
  if (!userDataDir) return
  if (!existsSync(reposFile())) return
  try {
    const raw = readFileSync(reposFile(), 'utf8')
    const parsed = JSON.parse(raw) as { repos?: PersistedRepo[] }
    for (const r of parsed.repos ?? []) {
      if (!r.id || !r.absolutePath) continue
      repos.set(r.id, {
        id: r.id,
        absolutePath: r.absolutePath,
        label: r.label ?? basename(r.absolutePath),
        status: 'stopped',
        port: null,
        baseUrl: null,
        boundOrigins: (r.boundOrigins ?? [])
          .filter((b) => b && typeof b.origin === 'string')
          .map((b) => ({ origin: normalizeOrigin(b.origin), autoFix: !!b.autoFix })),
        child: null,
        startupResolvers: [],
        startupTimer: null,
      })
    }
  } catch {
    // Corrupt repos.json — start fresh; better than throwing during boot.
  }
}

function toPublic(r: InternalRepo): ConnectedRepo {
  return {
    id: r.id,
    absolutePath: r.absolutePath,
    label: r.label,
    status: r.status,
    port: r.port,
    baseUrl: r.baseUrl,
    lastError: r.lastError,
    boundOrigins: r.boundOrigins.map((b) => ({ ...b })),
  }
}

function notifyChange(): void {
  events.emit('change', listRepos())
}

export function initDevServerManager(options: InitOptions): void {
  userDataDir = options.userDataDir
  spawnFn = options.spawn
  loadPersisted()
}

export function listRepos(): ConnectedRepo[] {
  return Array.from(repos.values()).map(toPublic)
}

export function getRepo(id: string): ConnectedRepo | null {
  const r = repos.get(id)
  return r ? toPublic(r) : null
}

export function findRepoForPath(absolutePath: string): ConnectedRepo | null {
  // Prefer the longest matching prefix: if the user has both ~/Developer
  // and ~/Developer/my-app connected, a file inside my-app should resolve
  // to my-app, not the parent. Otherwise nested repos silently lose.
  let best: InternalRepo | null = null
  for (const r of repos.values()) {
    if (absolutePath !== r.absolutePath && !absolutePath.startsWith(r.absolutePath + '/')) {
      continue
    }
    if (!best || r.absolutePath.length > best.absolutePath.length) {
      best = r
    }
  }
  return best ? toPublic(best) : null
}

export function connectRepo(absolutePath: string, label?: string): ConnectedRepo {
  const id = repoIdFor(absolutePath)
  const existing = repos.get(id)
  if (existing) return toPublic(existing)
  const entry: InternalRepo = {
    id,
    absolutePath,
    label: label ?? basename(absolutePath),
    status: 'stopped',
    port: null,
    baseUrl: null,
    boundOrigins: [],
    child: null,
    startupResolvers: [],
    startupTimer: null,
  }
  repos.set(id, entry)
  persist()
  notifyChange()
  return toPublic(entry)
}

export function findRepoForOrigin(origin: string): ConnectedRepo | null {
  const normalized = normalizeOrigin(origin)
  for (const r of repos.values()) {
    if (r.boundOrigins.some((b) => b.origin === normalized)) return toPublic(r)
  }
  return null
}

export function getOriginBindingsView(): OriginBindings {
  const view: OriginBindings = {}
  for (const r of repos.values()) {
    for (const b of r.boundOrigins) {
      view[b.origin] = { repoPath: r.absolutePath, autoFix: b.autoFix }
    }
  }
  return view
}

export function getOriginBindingView(origin: string): OriginBinding | null {
  const repo = findRepoForOrigin(origin)
  if (!repo) return null
  const binding = repo.boundOrigins.find((b) => b.origin === normalizeOrigin(origin))
  if (!binding) return null
  return { repoPath: repo.absolutePath, autoFix: binding.autoFix }
}

/** Bound origin, or a connected repo whose running baseUrl matches. */
export function inferRepoPathForOrigin(origin: string): string | null {
  const bound = getOriginBindingView(origin)
  if (bound) return bound.repoPath
  const normalized = normalizeOrigin(origin)
  if (!normalized) return null
  for (const repo of repos.values()) {
    if (!repo.baseUrl) continue
    try {
      if (new URL(repo.baseUrl).origin === normalized) return repo.absolutePath
    } catch {
      continue
    }
  }
  return null
}

/** Bind `origin` to an already-connected repo by id. Replaces any existing
 *  binding for this origin. Returns null if the repo doesn't exist or the
 *  origin can't be parsed. */
export function bindOriginToRepo(
  repoId: string,
  origin: string,
  autoFix = false,
): ConnectedRepo | null {
  const entry = repos.get(repoId)
  if (!entry) return null
  const normalized = normalizeOrigin(origin)
  if (!normalized) return toPublic(entry)
  removeBindingByOriginInternal(normalized, { skipNotify: true })
  const refetched = repos.get(repoId)
  if (!refetched) return null
  refetched.boundOrigins.push({ origin: normalized, autoFix })
  persist()
  notifyChange()
  return toPublic(refetched)
}

/** Bind `origin` to the repo at `repoPath`, connecting the repo first if
 *  needed. Replaces any existing binding for this origin. */
export function bindOriginToRepoPath(
  origin: string,
  repoPath: string,
  autoFix = false,
): ConnectedRepo {
  const repo = connectRepo(repoPath)
  const id = repo.id
  const entry = repos.get(id)
  if (!entry) return repo
  const normalized = normalizeOrigin(origin)
  removeBindingByOriginInternal(normalized, { skipNotify: true })
  const reFetched = repos.get(id)
  if (reFetched) reFetched.boundOrigins.push({ origin: normalized, autoFix })
  persist()
  notifyChange()
  return toPublic(reFetched ?? entry)
}

function removeBindingByOriginInternal(
  origin: string,
  options: { skipNotify?: boolean } = {},
): boolean {
  const normalized = normalizeOrigin(origin)
  let removed = false
  for (const r of repos.values()) {
    const next = r.boundOrigins.filter((b) => b.origin !== normalized)
    if (next.length !== r.boundOrigins.length) {
      r.boundOrigins = next
      removed = true
    }
  }
  if (removed && !options.skipNotify) {
    persist()
    notifyChange()
  }
  return removed
}

export function removeBindingByOrigin(origin: string): boolean {
  return removeBindingByOriginInternal(origin)
}

export function setBindingAutoFix(origin: string, enabled: boolean): boolean {
  const normalized = normalizeOrigin(origin)
  let mutated = false
  for (const r of repos.values()) {
    for (const b of r.boundOrigins) {
      if (b.origin === normalized && b.autoFix !== enabled) {
        b.autoFix = enabled
        mutated = true
      }
    }
  }
  if (mutated) {
    persist()
    notifyChange()
  }
  return mutated
}

export async function disconnectRepo(id: string): Promise<void> {
  const entry = repos.get(id)
  if (!entry) return
  await stopChild(entry)
  repos.delete(id)
  persist()
  notifyChange()
}

function stopChild(entry: InternalRepo): Promise<void> {
  return new Promise((resolve) => {
    const child = entry.child
    if (!child) {
      entry.status = 'stopped'
      entry.port = null
      entry.baseUrl = null
      resolve()
      return
    }
    let settled = false
    const settle = () => {
      if (settled) return
      settled = true
      entry.child = null
      entry.status = 'stopped'
      entry.port = null
      entry.baseUrl = null
      resolve()
    }
    child.once('exit', settle)
    try {
      child.kill('SIGTERM')
    } catch {
      settle()
      return
    }
    setTimeout(() => {
      if (settled) return
      try {
        child.kill('SIGKILL')
      } catch {
        // ignore
      }
      settle()
    }, 2_000)
  })
}

function clearStartupTimer(entry: InternalRepo): void {
  if (entry.startupTimer) {
    clearTimeout(entry.startupTimer)
    entry.startupTimer = null
  }
}

function flushStartupResolvers(entry: InternalRepo, url: string | null): void {
  clearStartupTimer(entry)
  const pending = entry.startupResolvers.splice(0)
  for (const resolve of pending) resolve(url)
}

function attachChildHandlers(entry: InternalRepo, child: ChildProcess): void {
  const handleStdoutLine = (line: string) => {
    const match = LOCAL_URL_REGEX.exec(line)
    if (!match) return
    const url = match[1].replace(/\/$/, '')
    try {
      const parsed = new URL(url)
      entry.port = Number(parsed.port) || null
      entry.baseUrl = url
      entry.status = 'running'
      entry.lastError = undefined
      // Auto-bind the dev server's own origin so pages pointing at it can
      // receive fixes without the user having to link a repo manually. An
      // origin routes fixes to exactly one repo, so take it over from any
      // repo that held it before — another project may have served this
      // port last.
      const origin = normalizeOrigin(url)
      if (origin) {
        const existing = entry.boundOrigins.find((b) => b.origin === origin)
        removeBindingByOriginInternal(origin, { skipNotify: true })
        entry.boundOrigins.push(existing ?? { origin, autoFix: false })
        persist()
      }
      flushStartupResolvers(entry, url)
      notifyChange()
    } catch {
      // ignore malformed URL
    }
  }

  const wireStdout = (stream: NodeJS.ReadableStream | null) => {
    if (!stream) return
    let buffer = ''
    stream.on('data', (chunk: Buffer | string) => {
      buffer += typeof chunk === 'string' ? chunk : chunk.toString('utf8')
      let newlineIdx: number
      while ((newlineIdx = buffer.indexOf('\n')) >= 0) {
        const line = buffer.slice(0, newlineIdx)
        buffer = buffer.slice(newlineIdx + 1)
        handleStdoutLine(line)
      }
    })
  }

  wireStdout(child.stdout)
  // Surface stderr to the main-process console without buffering — vite emits
  // useful error context here that would otherwise be lost.
  child.stderr?.on('data', (chunk: Buffer | string) => {
    process.stderr.write(typeof chunk === 'string' ? chunk : chunk)
  })

  child.once('exit', (code) => {
    entry.child = null
    if (entry.status !== 'errored') {
      entry.status = 'stopped'
    }
    if (code !== 0 && code !== null) {
      entry.lastError = `vite exited with code ${code}`
      entry.status = 'errored'
    }
    entry.port = null
    entry.baseUrl = null
    flushStartupResolvers(entry, null)
    notifyChange()
  })

  child.once('error', (err) => {
    entry.lastError = err.message
    entry.status = 'errored'
    flushStartupResolvers(entry, null)
    notifyChange()
  })
}

function startChild(entry: InternalRepo): Promise<string | null> {
  if (!spawnFn) throw new Error('dev-server-manager not initialized')
  if (entry.status === 'running' && entry.baseUrl) {
    return Promise.resolve(entry.baseUrl)
  }
  if (entry.status === 'starting') {
    return new Promise((resolve) => entry.startupResolvers.push(resolve))
  }
  entry.status = 'starting'
  entry.lastError = undefined
  notifyChange()
  const child = spawnFn('npx', ['vite', 'dev'], { cwd: entry.absolutePath })
  entry.child = child
  attachChildHandlers(entry, child)
  return new Promise((resolve) => {
    entry.startupResolvers.push(resolve)
    if (entry.startupTimer) return
    entry.startupTimer = setTimeout(() => {
      entry.startupTimer = null
      if (entry.status === 'starting') {
        entry.status = 'errored'
        entry.lastError = 'startup timed out'
        flushStartupResolvers(entry, null)
        notifyChange()
      }
    }, STARTUP_TIMEOUT_MS)
  })
}

export async function urlForComponent(
  repoId: string,
  repoRelativePath: string,
): Promise<string | null> {
  const entry = repos.get(repoId)
  if (!entry) return null
  const baseUrl = await startChild(entry)
  if (!baseUrl) return null
  const cleaned = repoRelativePath.replace(/^\/+/, '')
  return `${baseUrl}/__specular?path=${encodeURIComponent(cleaned)}`
}

export function onChange(listener: (repos: ConnectedRepo[]) => void): () => void {
  events.on('change', listener)
  return () => events.off('change', listener)
}

export async function shutdownDevServerManager(): Promise<void> {
  const entries = Array.from(repos.values())
  await Promise.all(entries.map(stopChild))
}

/** Test-only: wipe internal state so a fresh init can run. */
export function __resetDevServerManagerForTests(): void {
  repos.clear()
  events.removeAllListeners()
  userDataDir = null
  spawnFn = null
}

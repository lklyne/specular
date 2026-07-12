#!/usr/bin/env node

import { spawn } from 'node:child_process'
import {
  appendFileSync,
  closeSync,
  existsSync,
  mkdirSync,
  openSync,
  readFileSync,
  readSync,
  renameSync,
  rmSync,
  statSync,
  truncateSync,
  watchFile,
  writeFileSync,
} from 'node:fs'
import { homedir, tmpdir } from 'node:os'
import { dirname, isAbsolute, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const RUNTIME_DIR = join(ROOT, '.dev')
const STATE_FILE = join(RUNTIME_DIR, 'state.json')
const STATE_TMP_FILE = join(RUNTIME_DIR, 'state.json.tmp')
const START_LOCK = join(RUNTIME_DIR, 'start.lock')
const DEV_LOG = join(RUNTIME_DIR, 'dev.log')
const APP_HOST = '127.0.0.1'
const APP_PORT = Number(process.env.SPECULAR_PORT ?? 29979)
const HEARTBEAT_MS = 2_000
const STALE_HEARTBEAT_MS = 60_000
const START_TIMEOUT_MS = 60_000
const STOP_TIMEOUT_MS = 5_000
const command = process.argv[2] ?? 'help'

function ensureRuntimeDir() {
  mkdirSync(RUNTIME_DIR, { recursive: true })
}

function isProcessAlive(pid) {
  if (!Number.isInteger(pid) || pid <= 0) return false
  try {
    process.kill(pid, 0)
    return true
  } catch {
    return false
  }
}

function readState() {
  try {
    const state = JSON.parse(readFileSync(STATE_FILE, 'utf8'))
    if (
      typeof state.pid !== 'number' ||
      typeof state.updatedAt !== 'number' ||
      Date.now() - state.updatedAt > STALE_HEARTBEAT_MS ||
      !isProcessAlive(state.pid)
    ) {
      return null
    }
    return state
  } catch {
    return null
  }
}

function removeStaleState() {
  if (!existsSync(STATE_FILE) || readState()) return
  rmSync(STATE_FILE, { force: true })
}

function writeState(state) {
  ensureRuntimeDir()
  const next = { ...state, updatedAt: Date.now() }
  writeFileSync(STATE_TMP_FILE, `${JSON.stringify(next, null, 2)}\n`, 'utf8')
  renameSync(STATE_TMP_FILE, STATE_FILE)
  return next
}

function removeOwnedState(pid) {
  try {
    const state = JSON.parse(readFileSync(STATE_FILE, 'utf8'))
    if (state.pid === pid) rmSync(STATE_FILE, { force: true })
  } catch {
    // Missing or malformed state is already effectively removed.
  }
}

async function probeApp() {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), 1_000)
  try {
    const response = await fetch(`http://${APP_HOST}:${APP_PORT}/health`, {
      signal: controller.signal,
    })
    if (!response.ok) return null
    const payload = await response.json()
    return typeof payload.version === 'string' ? payload : null
  } catch {
    return null
  } finally {
    clearTimeout(timer)
  }
}

function sleep(ms) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, ms))
}

function acquireStartLock() {
  ensureRuntimeDir()
  try {
    if (Date.now() - statSync(START_LOCK).mtimeMs > START_TIMEOUT_MS) {
      rmSync(START_LOCK, { recursive: true, force: true })
    }
  } catch {
    // No existing lock.
  }
  try {
    mkdirSync(START_LOCK)
    return true
  } catch {
    return false
  }
}

function releaseStartLock() {
  rmSync(START_LOCK, { recursive: true, force: true })
}

async function start() {
  if (process.platform === 'win32') {
    console.error('devctl process management currently supports macOS and Linux.')
    process.exitCode = 1
    return
  }
  if (!acquireStartLock()) {
    console.error('Another devctl start is already in progress.')
    process.exitCode = 1
    return
  }

  try {
    removeStaleState()
    const managed = readState()
    const app = await probeApp()
    if (managed) {
      console.log(`Specular dev is already managed (supervisor ${managed.pid}, ${managed.status}).`)
      return
    }
    if (app) {
      console.log(
        `Specular is already running outside devctl on port ${APP_PORT}. Leaving it untouched.`,
      )
      return
    }

    truncateLog()
    const supervisor = spawn(process.execPath, [fileURLToPath(import.meta.url), 'supervise'], {
      cwd: ROOT,
      detached: true,
      stdio: 'ignore',
      env: { ...process.env, SPECULAR_DEVCTL_SUPERVISOR: '1' },
    })
    supervisor.unref()

    const deadline = Date.now() + START_TIMEOUT_MS
    while (Date.now() < deadline) {
      const state = readState()
      if (state?.status === 'running' && (await probeApp())) {
        console.log(`Specular ready (supervisor ${state.pid}, app-control port ${APP_PORT}).`)
        console.log(`Logs: ${DEV_LOG}`)
        return
      }
      if (state?.status === 'failed' || (supervisor.pid && !isProcessAlive(supervisor.pid))) break
      await sleep(250)
    }

    console.error(`Specular did not become ready. Inspect ${DEV_LOG}`)
    process.exitCode = 1
  } finally {
    releaseStartLock()
  }
}

async function status({ json = false, silent = false } = {}) {
  removeStaleState()
  const managed = readState()
  const app = await probeApp()
  const result = {
    running: Boolean(app),
    ownership: managed ? 'managed' : app ? 'external' : 'none',
    supervisorPid: managed?.pid ?? null,
    childPid: managed?.childPid ?? null,
    status: managed?.status ?? (app ? 'running' : 'stopped'),
    startedAt: managed?.startedAt ?? null,
    restartCount: managed?.restartCount ?? 0,
    appControlPort: app ? APP_PORT : null,
    devLog: DEV_LOG,
    errorLog: errorLogPath(),
  }
  if (!silent) {
    if (json) {
      console.log(JSON.stringify(result, null, 2))
    } else if (managed) {
      console.log(
        `Specular dev is ${result.status} (managed, supervisor ${managed.pid}, child ${managed.childPid ?? 'starting'}).`,
      )
    } else if (app) {
      console.log(`Specular is running outside devctl on app-control port ${APP_PORT}.`)
    } else {
      console.log('Specular is not running.')
    }
  }
  return result
}

async function restart() {
  if (process.platform === 'win32') {
    console.error('devctl restart currently supports macOS and Linux.')
    process.exitCode = 1
    return
  }
  const managed = readState()
  if (!managed) {
    if (await probeApp()) {
      console.error('Specular is running outside devctl. Refusing to restart another terminal.')
    } else {
      console.error('Specular dev is not running. Use `pnpm devctl start`.')
    }
    process.exitCode = 1
    return
  }

  process.kill(managed.pid, 'SIGUSR2')
  const previousCount = managed.restartCount ?? 0
  const deadline = Date.now() + START_TIMEOUT_MS
  while (Date.now() < deadline) {
    const next = readState()
    if (
      next?.status === 'running' &&
      (next.restartCount ?? 0) > previousCount &&
      (await probeApp())
    ) {
      console.log(`Specular restarted (generation ${next.restartCount + 1}).`)
      return
    }
    await sleep(250)
  }
  console.error(`Specular did not become ready after restart. Inspect ${DEV_LOG}`)
  process.exitCode = 1
}

async function stop() {
  const managed = readState()
  if (!managed) {
    if (await probeApp()) {
      console.error('Specular is running outside devctl. Refusing to stop another terminal.')
      process.exitCode = 1
    } else {
      console.log('Specular dev is already stopped.')
    }
    return
  }

  process.kill(managed.pid, 'SIGTERM')
  const deadline = Date.now() + STOP_TIMEOUT_MS + 2_000
  while (Date.now() < deadline) {
    if (!readState() && !(await probeApp())) {
      console.log('Specular dev stopped.')
      return
    }
    await sleep(200)
  }
  console.error(`Supervisor ${managed.pid} did not stop cleanly.`)
  process.exitCode = 1
}

function truncateLog() {
  ensureRuntimeDir()
  if (existsSync(DEV_LOG)) truncateSync(DEV_LOG)
  else writeFileSync(DEV_LOG, '', 'utf8')
}

function logSupervisor(message) {
  ensureRuntimeDir()
  appendFileSync(DEV_LOG, `[${new Date().toISOString()}] [devctl] ${message}\n`)
}

function tailText(path, lineCount) {
  if (!path || !existsSync(path)) return ''
  const content = readFileSync(path, 'utf8')
  return content.split(/\r?\n/).slice(-lineCount).join('\n').trim()
}

function errorLogPath() {
  const candidates =
    process.platform === 'darwin'
      ? [join(homedir(), 'Library', 'Logs', 'Specular', 'errors.log')]
      : process.platform === 'win32'
        ? [join(process.env.APPDATA ?? homedir(), 'Specular', 'logs', 'errors.log')]
        : [
            join(process.env.XDG_CONFIG_HOME ?? join(homedir(), '.config'), 'Specular', 'logs', 'errors.log'),
            join(homedir(), '.config', 'Specular', 'errors.log'),
          ]
  return candidates.find(existsSync) ?? candidates[0]
}

async function logs() {
  const errors = process.argv.includes('--errors')
  const follow = process.argv.includes('--follow') || process.argv.includes('-f')
  const tailArg = process.argv.find((arg) => arg.startsWith('--tail='))
  const lineCount = Number(tailArg?.slice('--tail='.length) ?? 200)
  const path = errors ? errorLogPath() : DEV_LOG
  const initial = tailText(path, Number.isFinite(lineCount) ? lineCount : 200)
  if (initial) process.stdout.write(`${initial}\n`)
  else console.log(`No log output at ${path}`)
  if (!follow) return

  let offset = existsSync(path) ? statSync(path).size : 0
  watchFile(path, { interval: 500 }, (current) => {
    if (current.size < offset) offset = 0
    if (current.size === offset) return
    const byteCount = current.size - offset
    const buffer = Buffer.alloc(byteCount)
    const fd = openSync(path, 'r')
    try {
      const bytesRead = readSync(fd, buffer, 0, byteCount, offset)
      process.stdout.write(buffer.subarray(0, bytesRead))
      offset += bytesRead
    } finally {
      closeSync(fd)
    }
  })
  await new Promise(() => {})
}

function readDiscovery() {
  try {
    const override = process.env.SPECULAR_DISCOVERY_FILE
    const path = override
      ? isAbsolute(override)
        ? override
        : join(tmpdir(), override)
      : join(homedir(), '.specular', 'specular-mcp.json')
    return JSON.parse(readFileSync(path, 'utf8'))
  } catch {
    return null
  }
}

async function getPerfContext() {
  const discovery = readDiscovery()
  if (!discovery?.port || !discovery?.secret) return null
  const headers = { 'x-specular-secret': discovery.secret }
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), 2_000)
  try {
    const [statusResponse, tracesResponse] = await Promise.all([
      fetch(`http://${APP_HOST}:${discovery.port}/perf/trace/status`, {
        headers,
        signal: controller.signal,
      }),
      fetch(`http://${APP_HOST}:${discovery.port}/perf/traces`, {
        headers,
        signal: controller.signal,
      }),
    ])
    if (!statusResponse.ok || !tracesResponse.ok) return null
    const traceStatus = await statusResponse.json()
    const traces = await tracesResponse.json()
    return { ...traceStatus, recentTraces: Array.isArray(traces) ? traces.slice(0, 5) : [] }
  } catch {
    return null
  } finally {
    clearTimeout(timer)
  }
}

async function context() {
  const processStatus = await status({ silent: true })
  const result = {
    generatedAt: new Date().toISOString(),
    process: processStatus,
    recentDevLog: tailText(DEV_LOG, 100),
    recentErrors: tailText(errorLogPath(), 100),
    performance: await getPerfContext(),
  }
  console.log(JSON.stringify(result, null, 2))
}

async function signalProcessGroup(child, signal) {
  if (!child?.pid) return
  try {
    if (process.platform === 'win32') child.kill(signal)
    else process.kill(-child.pid, signal)
  } catch {
    // The process may have exited between the liveness check and signal.
  }
}

async function waitForExit(child, timeoutMs) {
  if (!child || child.exitCode !== null) return true
  return await new Promise((resolvePromise) => {
    const timer = setTimeout(() => resolvePromise(false), timeoutMs)
    child.once('exit', () => {
      clearTimeout(timer)
      resolvePromise(true)
    })
  })
}

async function supervise() {
  if (process.env.SPECULAR_DEVCTL_SUPERVISOR !== '1') {
    console.error('The supervise command is internal. Use `pnpm devctl start`.')
    process.exit(1)
  }
  ensureRuntimeDir()
  const existing = readState()
  if (existing && existing.pid !== process.pid) {
    logSupervisor(`refusing second supervisor while ${existing.pid} owns the dev stack`)
    process.exit(1)
  }
  let child = null
  let state = {
    pid: process.pid,
    childPid: null,
    status: 'starting',
    startedAt: new Date().toISOString(),
    updatedAt: Date.now(),
    restartCount: 0,
  }
  let intentionalExit = false
  let changingChild = false

  const persist = (patch = {}) => {
    state = writeState({ ...state, ...patch })
  }
  const heartbeat = setInterval(() => persist(), HEARTBEAT_MS)

  const stopChild = async () => {
    if (!child || child.exitCode !== null) return
    await signalProcessGroup(child, 'SIGTERM')
    if (!(await waitForExit(child, STOP_TIMEOUT_MS))) {
      logSupervisor(`child ${child.pid} ignored SIGTERM; escalating to SIGKILL`)
      await signalProcessGroup(child, 'SIGKILL')
      await waitForExit(child, 1_000)
    }
  }

  const launchChild = async () => {
    const pnpm = process.platform === 'win32' ? 'pnpm.cmd' : 'pnpm'
    child = spawn(pnpm, ['dev'], {
      cwd: ROOT,
      detached: true,
      stdio: ['ignore', 'pipe', 'pipe'],
      env: process.env,
    })
    child.stdout.on('data', (chunk) => appendFileSync(DEV_LOG, chunk))
    child.stderr.on('data', (chunk) => appendFileSync(DEV_LOG, chunk))
    persist({ childPid: child.pid ?? null, status: 'starting' })
    logSupervisor(`started pnpm dev as child ${child.pid}`)

    child.once('exit', (code, signal) => {
      const exitedChild = child
      logSupervisor(`child ${child?.pid} exited (code ${code}, signal ${signal ?? 'none'})`)
      if (!changingChild && !intentionalExit) {
        changingChild = true
        void (async () => {
          await signalProcessGroup(exitedChild, 'SIGTERM')
          await sleep(250)
          await signalProcessGroup(exitedChild, 'SIGKILL')
          persist({ childPid: null, status: 'failed' })
          clearInterval(heartbeat)
          removeOwnedState(process.pid)
          process.exit(code ?? 1)
        })()
      }
    })

    const deadline = Date.now() + START_TIMEOUT_MS
    while (Date.now() < deadline && child.exitCode === null) {
      if (await probeApp()) {
        persist({ status: 'running' })
        return true
      }
      await sleep(250)
    }
    persist({ status: 'failed' })
    return false
  }

  const shutdown = async () => {
    if (intentionalExit) return
    intentionalExit = true
    persist({ status: 'stopping' })
    await stopChild()
    clearInterval(heartbeat)
    removeOwnedState(process.pid)
    process.exit(0)
  }

  const restartChild = async () => {
    if (changingChild || intentionalExit) return
    changingChild = true
    persist({ status: 'restarting' })
    logSupervisor('restarting managed dev stack')
    await stopChild()
    const shutdownDeadline = Date.now() + STOP_TIMEOUT_MS
    while (Date.now() < shutdownDeadline && (await probeApp())) await sleep(100)
    if (await probeApp()) {
      logSupervisor('old app remained healthy after process-group shutdown; restart aborted')
      persist({ childPid: null, status: 'failed' })
      changingChild = false
      return
    }
    state.restartCount += 1
    child = null
    await launchChild()
    changingChild = false
  }

  process.on('SIGTERM', () => void shutdown())
  process.on('SIGINT', () => void shutdown())
  if (process.platform !== 'win32') process.on('SIGUSR2', () => void restartChild())
  process.on('uncaughtException', (error) => {
    logSupervisor(`uncaught exception: ${error.stack ?? error}`)
    void shutdown()
  })
  process.on('unhandledRejection', (error) => {
    logSupervisor(`unhandled rejection: ${error instanceof Error ? error.stack : String(error)}`)
    void shutdown()
  })

  persist()
  if (!(await launchChild())) await shutdown()
}

function help() {
  console.log(`Usage: pnpm devctl <command>

Commands:
  start              Start a managed background pnpm dev
  status [--json]    Show whether Specular is managed, external, or stopped
  logs [-f]          Show managed Forge, Vite, and Electron output
  logs --errors      Show the persistent Specular error log
  restart            Restart only the dev stack owned by devctl
  stop               Stop only the dev stack owned by devctl
  context            Print agent-ready status, logs, errors, and perf metadata`)
}

switch (command) {
  case 'start':
    await start()
    break
  case 'status':
    await status({ json: process.argv.includes('--json') })
    break
  case 'logs':
    await logs()
    break
  case 'restart':
    await restart()
    break
  case 'stop':
    await stop()
    break
  case 'context':
    await context()
    break
  case 'supervise':
    await supervise()
    break
  case 'help':
  case '--help':
  case '-h':
    help()
    break
  default:
    console.error(`Unknown devctl command: ${command}`)
    help()
    process.exitCode = 1
}

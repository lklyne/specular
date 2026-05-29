import { spawn, type ChildProcess } from 'child_process'
import { mkdtempSync, readFileSync, writeFileSync, rmSync, existsSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'

const DISCOVERY_FILE = join(tmpdir(), 'specular-mcp.json')
const SMOKE_ENV_FILE = join(tmpdir(), 'specular-smoke-env.json')
const POLL_INTERVAL_MS = 500
const POLL_TIMEOUT_MS = 15_000

// Use a random high port to avoid colliding with a running Specular instance
const SMOKE_PORT = 29900 + Math.floor(Math.random() * 99)
const SMOKE_CDP_PORT = 39000 + Math.floor(Math.random() * 1000)

let electronProcess: ChildProcess | null = null
let sandboxDir: string | null = null

async function waitForServer(): Promise<{ port: number; secret: string }> {
  const start = Date.now()
  while (Date.now() - start < POLL_TIMEOUT_MS) {
    if (existsSync(DISCOVERY_FILE)) {
      const payload = JSON.parse(readFileSync(DISCOVERY_FILE, 'utf8'))
      if (payload.port !== SMOKE_PORT) {
        await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS))
        continue
      }
      try {
        const res = await fetch(`http://127.0.0.1:${payload.port}/health`)
        if (res.ok) return payload
      } catch {
        // Server not ready yet
      }
    }
    await new Promise((r) => setTimeout(r, POLL_INTERVAL_MS))
  }
  throw new Error(`Smoke test server not ready after ${POLL_TIMEOUT_MS}ms`)
}

export async function setup() {
  sandboxDir = mkdtempSync(join(tmpdir(), 'specular-smoke-'))

  const electronBin = join(process.cwd(), 'node_modules', '.bin', 'electron')
  const appEntry = join(process.cwd(), '.vite', 'build', 'index.js')

  const extraArgs = process.getuid?.() === 0 ? ['--no-sandbox'] : []
  electronProcess = spawn(electronBin, [appEntry, `--user-data-dir=${sandboxDir}`, ...extraArgs], {
    stdio: 'pipe',
    env: {
      ...process.env,
      NODE_ENV: 'production',
      SPECULAR_PORT: String(SMOKE_PORT),
      SPECULAR_REMOTE_DEBUGGING_PORT: String(SMOKE_CDP_PORT),
      SPECULAR_SKIP_ONBOARDING: '1',
      SPECULAR_BACKGROUND: '1',
    },
  })

  electronProcess.stderr?.on('data', (chunk: Buffer) => {
    const text = chunk.toString()
    if (!text.includes('GPU') && !text.includes('Passthrough') && !text.includes('Security Warning')) {
      process.stderr.write(`[electron] ${text}`)
    }
  })

  const { port, secret } = await waitForServer()

  // Write env to a temp file so test processes can read it
  writeFileSync(SMOKE_ENV_FILE, JSON.stringify({ port, secret }), 'utf8')

  console.log(`Smoke tests: Electron ready on port ${port}, sandbox at ${sandboxDir}`)
}

export async function teardown() {
  if (electronProcess) {
    // Detach stdio to allow vitest to exit cleanly
    electronProcess.stdout?.destroy()
    electronProcess.stderr?.destroy()
    electronProcess.stdin?.destroy()
    if (!electronProcess.killed) {
      electronProcess.kill('SIGTERM')
      await new Promise((r) => setTimeout(r, 1_000))
      if (!electronProcess.killed) electronProcess.kill('SIGKILL')
    }
    electronProcess.unref()
  }

  if (sandboxDir && existsSync(sandboxDir)) {
    rmSync(sandboxDir, { recursive: true, force: true })
  }

  // Only remove the discovery file if it belongs to our smoke test instance
  if (existsSync(DISCOVERY_FILE)) {
    try {
      const payload = JSON.parse(readFileSync(DISCOVERY_FILE, 'utf8'))
      if (payload.port === SMOKE_PORT) rmSync(DISCOVERY_FILE)
    } catch {
      // File may have been removed by another process
    }
  }
  if (existsSync(SMOKE_ENV_FILE)) rmSync(SMOKE_ENV_FILE)
}

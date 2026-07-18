/**
 * Contract test (issue #318, D11): pins the fetched agent-browser binary
 * (resources/bin/agent-browser) against the EXACT CLI surface
 * src/main/shared/browse-handler.ts assumes. Every `describe` block below
 * names the browse-handler assumption it protects so a future agent-browser
 * version bump that fails here points straight at the affected code path
 * instead of a mystery runtime error inside the app.
 *
 * Not wired into CI (see vitest.contract.config.ts) — the fetch script is
 * darwin-arm64-only, so this only runs where the binary can actually be
 * fetched. Run for real on a macOS (arm64) machine:
 *
 *   pnpm fetch:agent-browser
 *   node node_modules/vitest/vitest.mjs run --config vitest.contract.config.ts
 *   # or: pnpm test:contract
 *
 * On any other machine (including this sandbox, which is Linux) the binary
 * is absent and every test below is skipped with a loud banner explaining
 * why and how to run it for real — see BINARY_AVAILABLE below.
 */
import { describe, it, expect } from 'vitest'
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { createServer, type Server } from 'node:http'
import { existsSync, accessSync, constants as fsConstants, readFileSync, mkdtempSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { WebSocket as NodeWebSocket, WebSocketServer } from 'ws'
import { GLOBAL_AB_FLAGS } from '../../src/main/shared/browse-handler'
import { extractRectFromCdpResult } from '../../src/main/cdp-proxy'

// ---------------------------------------------------------------------------
// Binary resolution + skip banner
// ---------------------------------------------------------------------------

const REPO_ROOT = join(__dirname, '..', '..')
// Deliberately the fetched pin, not resolveAgentBrowserPath()'s env-var /
// PATH walk — this contract is specifically about the binary
// scripts/fetch-agent-browser.sh puts in place, not whatever happens to be
// on a developer's PATH. AGENT_BROWSER_CONTRACT_BIN is an escape hatch for
// pointing the suite at a different build while iterating.
const BIN_PATH = process.env.AGENT_BROWSER_CONTRACT_BIN ?? join(REPO_ROOT, 'resources', 'bin', 'agent-browser')

function isExecutable(path: string): boolean {
  try {
    accessSync(path, fsConstants.X_OK)
    return true
  } catch {
    return false
  }
}

const BINARY_AVAILABLE = existsSync(BIN_PATH) && isExecutable(BIN_PATH)

if (!BINARY_AVAILABLE) {
  console.warn(
    `\n${'='.repeat(78)}\n` +
    `SKIPPING agent-browser contract tests: no executable binary at\n` +
    `  ${BIN_PATH}\n` +
    `\n` +
    `resources/bin/agent-browser is fetched by scripts/fetch-agent-browser.sh\n` +
    `and ships darwin-arm64 only (see that script's ASSET pin), so it is not\n` +
    `present on this machine/CI. To run this contract for real, on macOS\n` +
    `(arm64):\n` +
    `  1. pnpm fetch:agent-browser\n` +
    `  2. node node_modules/vitest/vitest.mjs run --config vitest.contract.config.ts\n` +
    `     (or: pnpm test:contract)\n` +
    `${'='.repeat(78)}\n`,
  )
}

// Live-page checks (batch shape, snapshot refs + origin=) spin up
// agent-browser's own browser via `open`, which is heavier (may download/start
// a real Chromium) and unverified against the pinned binary from this sandbox.
// Opt in explicitly so a plain `pnpm test:contract` stays fast and doesn't
// surprise-start a browser. See the batch + "snapshot output shape" blocks.
const LIVE_BROWSER_CHECKS = BINARY_AVAILABLE && process.env.AGENT_BROWSER_CONTRACT_LIVE === '1'

// ---------------------------------------------------------------------------
// Pinned version — read from the fetch script so the pin lives in one place
// ---------------------------------------------------------------------------

function readPinnedVersion(): string {
  const script = readFileSync(join(REPO_ROOT, 'scripts', 'fetch-agent-browser.sh'), 'utf8')
  const match = script.match(/VERSION="(v[0-9.]+)"/)
  if (!match) {
    throw new Error('Could not read VERSION="vX.Y.Z" out of scripts/fetch-agent-browser.sh — pin format changed?')
  }
  return match[1]
}

const PINNED_VERSION = readPinnedVersion()

// ---------------------------------------------------------------------------
// Spawn helper — unlike browse-handler's spawnAsync, this NEVER rejects on a
// non-zero exit code. The contract test needs to inspect the exit code
// itself (see the "batch exits 0" assertion below), so treating non-zero as
// a hard failure here would hide exactly the thing under test.
// ---------------------------------------------------------------------------

interface RunResult {
  stdout: string
  stderr: string
  code: number | null
}

function run(args: string[], opts: { input?: string; timeoutMs?: number } = {}): Promise<RunResult> {
  const timeoutMs = opts.timeoutMs ?? 10_000
  return new Promise((resolve, reject) => {
    const child = spawn(BIN_PATH, args, { stdio: ['pipe', 'pipe', 'pipe'] })
    let stdout = ''
    let stderr = ''
    const timer = setTimeout(() => {
      child.kill('SIGKILL')
      reject(new Error(`agent-browser ${args.join(' ')} timed out after ${timeoutMs}ms`))
    }, timeoutMs)
    child.stdout.on('data', (chunk: Buffer) => { stdout += chunk.toString() })
    child.stderr.on('data', (chunk: Buffer) => { stderr += chunk.toString() })
    child.on('error', (err) => { clearTimeout(timer); reject(err) })
    child.on('close', (code) => { clearTimeout(timer); resolve({ stdout, stderr, code }) })
    if (opts.input != null) {
      child.stdin.write(opts.input)
      child.stdin.end()
    } else {
      child.stdin.end()
    }
  })
}

// A CDP URL that is syntactically valid but never accepts a connection —
// used to prove flags parsed successfully (the process gets far enough to
// *attempt* a connection) without needing a live browser.
const UNREACHABLE_CDP = 'ws://127.0.0.1:1/no-such-target'

// clap-style "I don't recognize this flag" errors, distinguished from a
// connection-stage failure. Kept broad (and every assertion that uses it
// prints the actual output on failure) since the exact wording is upstream's
// to change.
const UNKNOWN_FLAG_PATTERN = /unknown|unrecognized|unexpected argument|wasn't expected|invalid value/i

// ---------------------------------------------------------------------------
// Ref-resolution CDP shape capture (issue #319, Phase 1) — helpers.
//
// agent-browser's own `open` command (used by the other live checks above)
// launches and owns its browser process internally; nothing external gets
// its CDP websocket URL, so there is no traffic to intercept. Instead this
// check launches a real headless Chrome itself and fronts its browser-level
// CDP websocket with a small relaying proxy ("shim"), then points
// agent-browser at the shim via `--cdp` — the same arrangement
// src/main/cdp-proxy.ts uses in production (client <-> proxy <-> real
// browser ws). The shim only relays and records; it owns no app state.
// ---------------------------------------------------------------------------

function findChromeExecutable(): string | null {
  const candidates = [
    process.env.SPECULAR_CONTRACT_CHROME_PATH,
    process.env.CHROME_PATH,
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    '/Applications/Chromium.app/Contents/MacOS/Chromium',
    '/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary',
  ].filter((path): path is string => Boolean(path))
  for (const candidate of candidates) {
    if (existsSync(candidate) && isExecutable(candidate)) return candidate
  }
  return null
}

async function findFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const probe = createServer()
    probe.on('error', reject)
    probe.listen(0, '127.0.0.1', () => {
      const address = probe.address()
      probe.close(() => {
        if (address && typeof address === 'object') resolve(address.port)
        else reject(new Error('failed to find a free port'))
      })
    })
  })
}

async function waitForChromeVersionInfo(
  port: number,
  timeoutMs: number,
): Promise<{ webSocketDebuggerUrl: string }> {
  const deadline = Date.now() + timeoutMs
  let lastError: unknown
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/version`)
      if (response.ok) {
        const info = (await response.json()) as { webSocketDebuggerUrl?: string }
        if (info.webSocketDebuggerUrl) return { webSocketDebuggerUrl: info.webSocketDebuggerUrl }
      }
    } catch (err) {
      lastError = err
    }
    await new Promise((resolve) => setTimeout(resolve, 100))
  }
  throw new Error(`probe Chrome did not become ready on port ${port} within ${timeoutMs}ms: ${String(lastError)}`)
}

async function openChromeTab(port: number, url: string): Promise<void> {
  const response = await fetch(`http://127.0.0.1:${port}/json/new?${encodeURIComponent(url)}`, { method: 'PUT' })
  if (!response.ok) {
    throw new Error(`failed to open a tab in the probe Chrome (${response.status})`)
  }
}

interface CdpFrame {
  direction: 'toUpstream' | 'toClient'
  payload: Record<string, unknown>
}

/** Relays a client (agent-browser) websocket to a real browser-level CDP
 *  websocket, recording every JSON frame that passes through in either
 *  direction. Deliberately dumb — no correlation, no filtering — the test
 *  itself does the interpretation, exactly as production `cdp-proxy.ts`
 *  keeps sniffing (recordPendingRectRequest / extractRectFromCdpResult)
 *  separate from relaying. */
function startCdpProxyShim(upstreamUrl: string): Promise<{
  wsUrl: string
  frames: CdpFrame[]
  close: () => Promise<void>
}> {
  return new Promise((resolve, reject) => {
    const frames: CdpFrame[] = []
    const wss = new WebSocketServer({ port: 0, host: '127.0.0.1' })
    wss.once('error', reject)
    wss.once('listening', () => {
      const address = wss.address()
      if (!address || typeof address !== 'object') {
        reject(new Error('shim proxy failed to bind'))
        return
      }
      wss.on('connection', (clientSocket) => {
        const upstreamSocket = new NodeWebSocket(upstreamUrl)
        const pendingToUpstream: string[] = []
        upstreamSocket.on('open', () => {
          for (const message of pendingToUpstream.splice(0)) upstreamSocket.send(message)
        })
        clientSocket.on('message', (raw) => {
          const text = raw.toString()
          try {
            frames.push({ direction: 'toUpstream', payload: JSON.parse(text) })
          } catch {
            // Non-JSON frame (shouldn't happen for CDP) — still relay it.
          }
          if (upstreamSocket.readyState === NodeWebSocket.OPEN) upstreamSocket.send(text)
          else pendingToUpstream.push(text)
        })
        upstreamSocket.on('message', (raw) => {
          const text = raw.toString()
          try {
            frames.push({ direction: 'toClient', payload: JSON.parse(text) })
          } catch {
            // Non-JSON frame — still relay it.
          }
          if (clientSocket.readyState === NodeWebSocket.OPEN) clientSocket.send(text)
        })
        clientSocket.on('close', () => upstreamSocket.close())
        upstreamSocket.on('close', () => clientSocket.close())
      })
      resolve({
        wsUrl: `ws://127.0.0.1:${address.port}/devtools/browser/contract-shim`,
        frames,
        close: () =>
          new Promise<void>((res) => {
            for (const client of wss.clients) client.close()
            wss.close(() => res())
          }),
      })
    })
  })
}

describe.skipIf(!BINARY_AVAILABLE)(`agent-browser contract (pinned ${PINNED_VERSION})`, () => {
  // -------------------------------------------------------------------------
  // Version pin
  // -------------------------------------------------------------------------
  describe('version pin', () => {
    it('matches VERSION in scripts/fetch-agent-browser.sh', async () => {
      // Proves: the binary actually fetched onto disk is the one we think we
      // pinned — catches a stale local fetch or a script edit that didn't
      // re-fetch.
      const { stdout, stderr, code } = await run(['--version'])
      const output = `${stdout}${stderr}`
      expect(code, `--version exited non-zero.\nstdout: ${stdout}\nstderr: ${stderr}`).toBe(0)
      const bareVersion = PINNED_VERSION.replace(/^v/, '')
      expect(output, `--version output did not mention ${bareVersion}: ${JSON.stringify(output)}`).toContain(bareVersion)
    })
  })

  // -------------------------------------------------------------------------
  // Global flags — browse-handler.ts's GLOBAL_AB_FLAGS + sessionFlags + --cdp
  // are prepended to literally every invocation (single command, batch, and
  // the scrollintoview/get-url/echo side calls). If any of these stop being
  // recognized, every browse call breaks, not just one verb.
  // -------------------------------------------------------------------------
  describe('global flags browse-handler prepends to every call', () => {
    it('--help lists --content-boundaries, --max-output, --session, --cdp', async () => {
      // Proves: the CLI's own --help registers these as known global flags.
      // Does not exercise runtime behavior — see the parse-vs-connect check
      // below for that.
      const { stdout, stderr, code } = await run(['--help'])
      const help = `${stdout}${stderr}`
      expect(code, `--help exited non-zero.\nstdout: ${stdout}\nstderr: ${stderr}`).toBe(0)
      for (const flag of ['--content-boundaries', '--max-output', '--session', '--cdp']) {
        expect(help, `--help output missing ${flag}:\n${help}`).toContain(flag)
      }
    })

    it('rejects a genuinely unknown flag as a parse error (negative control)', async () => {
      // Establishes what an unrecognized-flag error looks like, so the next
      // test can prove GLOBAL_AB_FLAGS do NOT hit that path.
      const { stdout, stderr, code } = await run(['--this-flag-does-not-exist-abc123'])
      const output = `${stdout}${stderr}`
      expect(code, 'a bogus flag unexpectedly exited 0').not.toBe(0)
      expect(output, `expected an unknown-flag error, got:\n${output}`).toMatch(UNKNOWN_FLAG_PATTERN)
    })

    it('accepts --content-boundaries, --max-output <n>, --session <name>, --cdp <url> as parseable (fails only at the connection stage)', async () => {
      // Mirrors browse-handler's exact prefix: GLOBAL_AB_FLAGS + ['--session', id] + ['--cdp', url].
      // An unreachable --cdp target means this MUST fail — the point is
      // proving it fails for "couldn't connect", not "didn't understand the
      // flags".
      const args = [...GLOBAL_AB_FLAGS, '--session', 'contract-test', '--cdp', UNREACHABLE_CDP, 'snapshot']
      const { stdout, stderr, code } = await run(args, { timeoutMs: 15_000 })
      const output = `${stdout}${stderr}`
      expect(code, 'expected a non-zero exit against an unreachable CDP target').not.toBe(0)
      expect(output, `flags were rejected as unknown instead of failing at connection:\n${output}`).not.toMatch(UNKNOWN_FLAG_PATTERN)
    })
  })

  // -------------------------------------------------------------------------
  // batch --json --bail — browse-handler's chained-command path
  // (handleBrowse, the `isChained` branch) pipes a JSON array of argv arrays
  // on stdin and does `JSON.parse(stdout)` expecting
  // Array<{ command, success, error, result }>, AND relies on exit 0 even when
  // a batched command fails under --bail (its spawnAsync rejects on any
  // non-zero exit and would never reach JSON.parse otherwise).
  //
  // v0.31.1 connects to CDP *before* running the batch, so an unreachable
  // --cdp target quits with a single error object + exit 1 and never produces
  // the array — the shape is only observable against a live browser. Same
  // constraint (and same opt-in) as the snapshot live check below.
  // -------------------------------------------------------------------------
  describe('batch --json --bail (chained-command path, live page)', () => {
    it('writes the {command,success,error,result} array to stdout even when a command fails (non-zero exit)', async (ctx) => {
      if (!LIVE_BROWSER_CHECKS) {
        console.warn(
          'SKIPPING live batch check: set AGENT_BROWSER_CONTRACT_LIVE=1 to run it. ' +
          'The pinned binary connects to CDP before running the batch, so the array ' +
          'output shape can only be observed against a real browser (started via ' +
          "agent-browser's own `open`) — run it on macOS with the fetched binary.",
        )
        ctx.skip()
        return
      }

      let server: Server | undefined
      const sessionName = 'contract-test-batch'
      try {
        const html = '<!doctype html><html><body><button id="go">Go</button></body></html>'
        const port = await new Promise<number>((resolve, reject) => {
          server = createServer((_req, res) => {
            res.writeHead(200, { 'content-type': 'text/html' })
            res.end(html)
          })
          server.on('error', reject)
          server.listen(0, '127.0.0.1', () => {
            const address = server!.address()
            if (address && typeof address === 'object') resolve(address.port)
            else reject(new Error('failed to bind local test server'))
          })
        })
        const url = `http://127.0.0.1:${port}/`

        const launch = await run(['--session', sessionName, 'open', url], { timeoutMs: 20_000 })
        if (launch.code !== 0) {
          console.warn(
            `SKIPPING live batch check: \`open\` did not succeed in this environment ` +
            `(exit ${launch.code}). Usually no Chrome/Chromium available here, not a ` +
            `CLI-surface break — investigate manually on a real macOS dev machine.\n` +
            `stdout: ${launch.stdout}\nstderr: ${launch.stderr}`,
          )
          ctx.skip()
          return
        }

        // One command that succeeds (`get url`) and one that fails (a click on
        // a selector that isn't there), under --bail. Verifies the array shape
        // browse-handler's JSON.parse assumes and a reported per-command
        // failure (success:false + error string) — the raw material for the
        // per-command error text + stale-ref hints.
        //
        // The binary exits NON-ZERO the moment a --bail command fails, but
        // still writes the full array to stdout. browse-handler must read that
        // stdout regardless of exit code (spawnAsync's allowNonZeroExit) — if
        // it rejected on the exit code, every chained call containing a failure
        // would surface a raw process error instead of the formatted per-
        // command output. This test pins both halves: array on stdout, and the
        // non-zero exit that makes reading-regardless necessary.
        const batchArgs = [...GLOBAL_AB_FLAGS, '--session', sessionName, 'batch', '--json', '--bail']
        const batchInput = JSON.stringify([['get', 'url'], ['click', '#no-such-element-xyz']])
        const { stdout, stderr, code } = await run(batchArgs, { input: batchInput, timeoutMs: 15_000 })

        let parsed: unknown
        try {
          parsed = JSON.parse(stdout)
        } catch (err) {
          throw new Error(
            `batch --json --bail did not emit parseable JSON on stdout (exit ${code}).\n` +
            `stdout: ${stdout}\nstderr: ${stderr}\nparse error: ${String(err)}`,
          )
        }
        expect(Array.isArray(parsed), `expected a JSON array, got: ${stdout}`).toBe(true)
        const entries = parsed as Array<Record<string, unknown>>
        expect(entries.length, `expected at least one batch entry: ${stdout}`).toBeGreaterThan(0)
        for (const entry of entries) {
          for (const key of ['command', 'success', 'error', 'result']) {
            expect(entry, `batch entry missing "${key}": ${JSON.stringify(entry)}`).toHaveProperty(key)
          }
        }
        const failed = entries.find((e) => e.success === false)
        expect(failed, `expected a reported per-command failure: ${stdout}`).toBeTruthy()
        expect(typeof failed!.error, `expected failed entry.error to be a string: ${JSON.stringify(failed)}`).toBe('string')
        // The per-command failure JSON above lives on stdout despite this
        // non-zero exit — the exact reason browse-handler reads stdout with
        // allowNonZeroExit instead of trusting the exit code.
        expect(code, `expected a non-zero exit when a --bail command fails; if this is now 0, revisit spawnAsync's allowNonZeroExit in the batch path.\nstdout: ${stdout}\nstderr: ${stderr}`).not.toBe(0)
      } finally {
        await run(['--session', sessionName, 'close']).catch(() => {})
        server?.close()
      }
    })
  })

  // -------------------------------------------------------------------------
  // skills get core — the `skills` meta-verb in cli-commands.ts spawns the
  // binary directly with NO global flags (no --content-boundaries,
  // --max-output, --session, or --cdp): `spawnAsync(bin, ['skills', ...rest])`.
  // -------------------------------------------------------------------------
  describe('skills get core', () => {
    it('returns non-empty content with no global flags', async () => {
      // Mirrors cli-commands.ts's `skills` verb handler exactly — bare
      // invocation, no page/session/CDP resolution.
      const { stdout, stderr, code } = await run(['skills', 'get', 'core'], { timeoutMs: 15_000 })
      expect(code, `skills get core exited non-zero.\nstdout: ${stdout}\nstderr: ${stderr}`).toBe(0)
      expect(stdout.trim().length, `expected non-empty content, got: ${JSON.stringify(stdout)}`).toBeGreaterThan(0)
    })
  })

  // -------------------------------------------------------------------------
  // wait --text / --url — cli-commands.ts's buildWaitCommand forwards these
  // as quoted flags; browse-handler.ts must recognize them (D6).
  // -------------------------------------------------------------------------
  describe('wait --text / --url flag acceptance', () => {
    it('accepts --text (fails at connection stage, not as an unknown flag)', async () => {
      const args = [...GLOBAL_AB_FLAGS, '--session', 'contract-test-wait', '--cdp', UNREACHABLE_CDP, 'wait', '--text', 'Order confirmed', '--timeout', '100']
      const { stdout, stderr, code } = await run(args, { timeoutMs: 15_000 })
      const output = `${stdout}${stderr}`
      expect(code, 'expected a non-zero exit against an unreachable CDP target').not.toBe(0)
      expect(output, `--text was rejected as an unknown flag:\n${output}`).not.toMatch(UNKNOWN_FLAG_PATTERN)
    })

    it('accepts --url (fails at connection stage, not as an unknown flag)', async () => {
      const args = [...GLOBAL_AB_FLAGS, '--session', 'contract-test-wait', '--cdp', UNREACHABLE_CDP, 'wait', '--url', '**/checkout', '--timeout', '100']
      const { stdout, stderr, code } = await run(args, { timeoutMs: 15_000 })
      const output = `${stdout}${stderr}`
      expect(code, 'expected a non-zero exit against an unreachable CDP target').not.toBe(0)
      expect(output, `--url was rejected as an unknown flag:\n${output}`).not.toMatch(UNKNOWN_FLAG_PATTERN)
    })
  })

  // -------------------------------------------------------------------------
  // snapshot output shape — checkOriginMismatch() in browse-handler.ts
  // regexes snapshot output for `origin=<url>`, and the snapshot text is
  // passed through to the agent, which needs interactable-element refs in it.
  // The binary prints refs as `[ref=eN]` tokens (agents then target them by
  // typing `@eN`; browse-handler never parses refs back out of snapshot
  // output). Needs a real page behind a real CDP connection, which means
  // agent-browser has to start a browser via `open` — heavier and slower than
  // the other checks, so it's opt-in (AGENT_BROWSER_CONTRACT_LIVE=1) rather
  // than part of the default `pnpm test:contract` run.
  // -------------------------------------------------------------------------
  describe('snapshot output shape (live page)', () => {
    it('contains [ref=eN] tokens and an origin= annotation', async (ctx) => {
      if (!LIVE_BROWSER_CHECKS) {
        console.warn(
          'SKIPPING live snapshot check: set AGENT_BROWSER_CONTRACT_LIVE=1 to run it. ' +
          'It starts a real browser via agent-browser\'s own `open` command against ' +
          'a throwaway local HTTP server, which this sandbox cannot verify (no binary, ' +
          'no display) — run it for real on macOS with the fetched binary.',
        )
        ctx.skip()
        return
      }

      let server: Server | undefined
      const sessionName = 'contract-test-snapshot'
      try {
        const html = '<!doctype html><html><body><button id="go">Go</button></body></html>'
        const port = await new Promise<number>((resolve, reject) => {
          server = createServer((_req, res) => {
            res.writeHead(200, { 'content-type': 'text/html' })
            res.end(html)
          })
          server.on('error', reject)
          server.listen(0, '127.0.0.1', () => {
            const address = server!.address()
            if (address && typeof address === 'object') resolve(address.port)
            else reject(new Error('failed to bind local test server'))
          })
        })
        const url = `http://127.0.0.1:${port}/`

        const launch = await run(['--session', sessionName, 'open', url], { timeoutMs: 20_000 })
        if (launch.code !== 0) {
          console.warn(
            `SKIPPING live snapshot check: \`open\` did not succeed in this environment ` +
            `(exit ${launch.code}). This usually means no Chrome/Chromium is available/installed ` +
            `here, not a CLI-surface break — investigate manually if this also fails on a real ` +
            `macOS dev machine with the app installed.\nstdout: ${launch.stdout}\nstderr: ${launch.stderr}`,
          )
          ctx.skip()
          return
        }

        const snapshot = await run([...GLOBAL_AB_FLAGS, '--session', sessionName, 'snapshot', '-i'], { timeoutMs: 15_000 })
        expect(snapshot.code, `snapshot exited non-zero.\nstdout: ${snapshot.stdout}\nstderr: ${snapshot.stderr}`).toBe(0)
        const output = snapshot.stdout
        expect(output, `no [ref=eN] token found in snapshot output:\n${output}`).toMatch(/\[ref=e\d+\]/)
        expect(output, `no origin= annotation found in snapshot output:\n${output}`).toMatch(/origin=\S+/)
      } finally {
        await run(['--session', sessionName, 'close']).catch(() => {})
        server?.close()
      }
    })
  })

  // -------------------------------------------------------------------------
  // Ref-resolution CDP shape (issue #319, Phase 1) — cdp-proxy.ts's
  // recordPendingRectRequest/extractRectFromCdpResult sniff the CDP bridge
  // for DOM.getBoxModel / Runtime.callFunctionOn request/response pairs so
  // the presence cursor can pre-position before the click's
  // Input.dispatchMouseEvent arrives. That sniff assumes agent-browser
  // resolves an @eN ref to a rect using one of those two methods, in that
  // shape, before dispatching the click. If a future binary bump changes the
  // element-resolution strategy (different method, different response
  // shape), the pre-move head start dies silently — no error, just a cursor
  // that always pays the full dwell. This pins the assumption against the
  // real binary.
  // -------------------------------------------------------------------------
  describe('ref-resolution CDP shape (live browser, click on @eN ref)', () => {
    it('issues DOM.getBoxModel or Runtime.callFunctionOn (parseable by extractRectFromCdpResult) before Input.dispatchMouseEvent(mousePressed)', async (ctx) => {
      if (!LIVE_BROWSER_CHECKS) {
        console.warn(
          'SKIPPING ref-resolution CDP shape check: set AGENT_BROWSER_CONTRACT_LIVE=1 to run it. ' +
          'It launches a real headless Chrome and fronts its CDP websocket with a relaying proxy ' +
          'to capture traffic — unverifiable in this sandbox (no binary, no Chrome) — run it for ' +
          'real on macOS with the fetched binary.',
        )
        ctx.skip()
        return
      }

      const chromePath = findChromeExecutable()
      if (!chromePath) {
        console.warn(
          'SKIPPING ref-resolution CDP shape check: no Chrome/Chromium executable found. Set ' +
          'SPECULAR_CONTRACT_CHROME_PATH (or CHROME_PATH) to a Chrome/Chromium binary, or install ' +
          'Google Chrome, to run this for real.',
        )
        ctx.skip()
        return
      }

      let server: Server | undefined
      let chrome: ChildProcessWithoutNullStreams | undefined
      let shim: { wsUrl: string; frames: CdpFrame[]; close: () => Promise<void> } | undefined
      let userDataDir: string | undefined
      const sessionName = 'contract-test-ref-resolution'
      try {
        // A moving-nothing static page is enough here — the assertion is
        // about *what CDP calls precede the click*, not about staleness
        // races (that's tests/agent/scenarios/presence-staleness.md).
        const html = '<!doctype html><html><body style="margin:40px"><button id="go" style="width:120px;height:40px">Go</button></body></html>'
        const port = await new Promise<number>((resolve, reject) => {
          server = createServer((_req, res) => {
            res.writeHead(200, { 'content-type': 'text/html' })
            res.end(html)
          })
          server.on('error', reject)
          server.listen(0, '127.0.0.1', () => {
            const address = server!.address()
            if (address && typeof address === 'object') resolve(address.port)
            else reject(new Error('failed to bind local test server'))
          })
        })
        const pageUrl = `http://127.0.0.1:${port}/`

        const chromePort = await findFreePort()
        userDataDir = mkdtempSync(join(tmpdir(), 'specular-contract-chrome-'))
        chrome = spawn(chromePath, [
          '--headless=new',
          '--disable-gpu',
          '--no-first-run',
          '--no-default-browser-check',
          `--remote-debugging-port=${chromePort}`,
          `--user-data-dir=${userDataDir}`,
          'about:blank',
        ]) as ChildProcessWithoutNullStreams
        chrome.on('error', () => {}) // surfaced via waitForChromeVersionInfo's timeout instead

        const { webSocketDebuggerUrl } = await waitForChromeVersionInfo(chromePort, 15_000)
        await openChromeTab(chromePort, pageUrl)

        shim = await startCdpProxyShim(webSocketDebuggerUrl)

        const snapshot = await run(
          [...GLOBAL_AB_FLAGS, '--session', sessionName, '--cdp', shim.wsUrl, 'snapshot', '-i'],
          { timeoutMs: 15_000 },
        )
        expect(snapshot.code, `snapshot exited non-zero.\nstdout: ${snapshot.stdout}\nstderr: ${snapshot.stderr}`).toBe(0)
        const refMatch = snapshot.stdout.match(/\[ref=(e\d+)\]/)
        expect(refMatch, `no [ref=eN] token found in snapshot output:\n${snapshot.stdout}`).toBeTruthy()
        const ref = refMatch![1]

        const framesBeforeClick = shim.frames.length
        const click = await run(
          [...GLOBAL_AB_FLAGS, '--session', sessionName, '--cdp', shim.wsUrl, 'click', `@${ref}`],
          { timeoutMs: 15_000 },
        )
        expect(click.code, `click exited non-zero.\nstdout: ${click.stdout}\nstderr: ${click.stderr}`).toBe(0)

        const clickFrames = shim.frames.slice(framesBeforeClick)
        const mousePressedIndex = clickFrames.findIndex((frame) => {
          if (frame.direction !== 'toUpstream') return false
          if (frame.payload.method !== 'Input.dispatchMouseEvent') return false
          const params = frame.payload.params as Record<string, unknown> | undefined
          return params?.type === 'mousePressed'
        })
        expect(
          mousePressedIndex,
          `no Input.dispatchMouseEvent(mousePressed) frame observed during the click:\n${JSON.stringify(clickFrames, null, 2)}`,
        ).toBeGreaterThanOrEqual(0)

        const requestsBeforeClick = clickFrames.slice(0, mousePressedIndex).filter(
          (frame) =>
            frame.direction === 'toUpstream' &&
            (frame.payload.method === 'DOM.getBoxModel' || frame.payload.method === 'Runtime.callFunctionOn'),
        )
        expect(
          requestsBeforeClick.length,
          `expected a DOM.getBoxModel or Runtime.callFunctionOn request before mousePressed — the exact ` +
          `CDP shape src/main/cdp-proxy.ts's recordPendingRectRequest sniffs for. Observed frames:\n` +
          `${JSON.stringify(clickFrames, null, 2)}`,
        ).toBeGreaterThan(0)

        // Every such request must have a response extractRectFromCdpResult
        // can actually parse into a rect — cdp-proxy.ts's pre-move head
        // start is dead weight otherwise, even if the method name matches.
        let parsedAtLeastOneRect = false
        for (const request of requestsBeforeClick) {
          const requestId = request.payload.id
          const method = request.payload.method as string
          const response = clickFrames.find(
            (frame) => frame.direction === 'toClient' && frame.payload.id === requestId,
          )
          if (!response) continue
          const rect = extractRectFromCdpResult(method, response.payload.result)
          if (rect) {
            parsedAtLeastOneRect = true
            expect(rect.width, `parsed rect had non-positive width: ${JSON.stringify(rect)}`).toBeGreaterThan(0)
            expect(rect.height, `parsed rect had non-positive height: ${JSON.stringify(rect)}`).toBeGreaterThan(0)
          }
        }
        expect(
          parsedAtLeastOneRect,
          `none of the DOM.getBoxModel/Runtime.callFunctionOn responses before mousePressed were parseable ` +
          `by extractRectFromCdpResult — the sniffed shape has drifted from what the binary actually emits. ` +
          `Observed frames:\n${JSON.stringify(clickFrames, null, 2)}`,
        ).toBe(true)
      } finally {
        await run(['--session', sessionName, 'close']).catch(() => {})
        await shim?.close().catch(() => {})
        chrome?.kill('SIGKILL')
        server?.close()
        if (userDataDir) rmSync(userDataDir, { recursive: true, force: true })
      }
    })
  })
})

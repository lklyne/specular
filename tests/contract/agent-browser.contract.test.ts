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
import { spawn } from 'node:child_process'
import { createServer, type Server } from 'node:http'
import { existsSync, accessSync, constants as fsConstants, readFileSync } from 'node:fs'
import { join } from 'node:path'
import { GLOBAL_AB_FLAGS } from '../../src/main/shared/browse-handler'

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

// Live-page checks (snapshot refs + origin=) spin up agent-browser's own
// browser via `launch`, which is heavier (may download/launch a real
// Chromium) and unverified against the pinned binary from this sandbox.
// Opt in explicitly so a plain `pnpm test:contract` stays fast and doesn't
// surprise-launch a browser. See the "snapshot output shape" describe block.
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
  // Array<{ command, success, error, result }>.
  // -------------------------------------------------------------------------
  describe('batch --json --bail (chained-command path)', () => {
    const batchArgs = [...GLOBAL_AB_FLAGS, '--session', 'contract-test-batch', '--cdp', UNREACHABLE_CDP, 'batch', '--json', '--bail']
    const batchInput = JSON.stringify([['get', 'url']])

    it('emits a JSON array of {command, success, error, result} on stdout', async () => {
      // Proves the output shape browse-handler's JSON.parse(stdout) assumes,
      // using a command that's guaranteed to fail (unreachable CDP) so no
      // live browser is required.
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
      const entry = entries[0]
      for (const key of ['command', 'success', 'error', 'result']) {
        expect(entry, `batch entry missing "${key}": ${JSON.stringify(entry)}`).toHaveProperty(key)
      }
      expect(entry.command).toEqual(['get', 'url'])
      expect(entry.success).toBe(false)
      expect(typeof entry.error, `expected entry.error to be a non-empty string: ${JSON.stringify(entry)}`).toBe('string')
      expect((entry.error as string).length).toBeGreaterThan(0)
    })

    it('exits 0 even though the batched command failed', async () => {
      // browse-handler.ts's spawnAsync rejects the whole call on ANY
      // non-zero exit and never reaches JSON.parse (see handleBrowse's
      // isChained branch, lines around the `batch --json --bail` spawnAsync
      // call). If agent-browser exits non-zero when --bail stops a failing
      // chain, EVERY chained browse command breaks with a raw process error
      // instead of the formatted per-command failure text — this pins that
      // exit-0-on-reported-failure assumption specifically, separate from
      // the JSON-shape assertion above, so a break here points straight at
      // that spawnAsync call rather than looking like a JSON parsing bug.
      const { stdout, stderr, code } = await run(batchArgs, { input: batchInput, timeoutMs: 15_000 })
      expect(code, `batch --json --bail exited ${code} instead of 0 — browse-handler's spawnAsync would reject this and never see the JSON below.\nstdout: ${stdout}\nstderr: ${stderr}`).toBe(0)
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
  // regexes snapshot output for `origin=<url>`, and every ref-targeted
  // mutation assumes `@eN` tokens in snapshot output. Both require a real
  // page behind a real CDP connection, which needs agent-browser to launch
  // its own browser via `launch` — heavier and slower than the other checks,
  // so it's opt-in (AGENT_BROWSER_CONTRACT_LIVE=1) rather than part of the
  // default `pnpm test:contract` run.
  // -------------------------------------------------------------------------
  describe('snapshot output shape (live page)', () => {
    it('contains @eN refs and an origin= annotation', async (ctx) => {
      if (!LIVE_BROWSER_CHECKS) {
        console.warn(
          'SKIPPING live snapshot check: set AGENT_BROWSER_CONTRACT_LIVE=1 to run it. ' +
          'It launches a real browser via agent-browser\'s own `launch` command against ' +
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

        const launch = await run(['--session', sessionName, 'launch', url], { timeoutMs: 20_000 })
        if (launch.code !== 0) {
          console.warn(
            `SKIPPING live snapshot check: \`launch\` did not succeed in this environment ` +
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
        expect(output, `no @eN ref found in snapshot output:\n${output}`).toMatch(/@e\d+/)
        expect(output, `no origin= annotation found in snapshot output:\n${output}`).toMatch(/origin=\S+/)
      } finally {
        await run(['--session', sessionName, 'close']).catch(() => {})
        server?.close()
      }
    })
  })
})

import { spawnSync } from 'child_process'
import { existsSync, readFileSync } from 'fs'
import { join } from 'path'
import { tmpdir } from 'os'
import { randomUUID } from 'crypto'

// ---------------------------------------------------------------------------
// CLI probe harness
//
// These probes exercise the real `specular` CLI binary the way an agent would,
// pointed at the ephemeral smoke app (see tests/smoke/global-setup.ts). Unlike
// the HTTP AppClient, they go through the actual agent-facing surface — flags,
// stdout shape, stderr messages, exit codes — so friction in CLI *ergonomics*
// shows up as a failing assertion, not just a broken feature.
//
// Transport: the smoke app writes a private discovery file at
// `${tmpdir}/specular-mcp-smoke-${port}.json`. We point the CLI at it via
// SPECULAR_DISCOVERY_FILE so probes never touch the canonical specular-mcp.json
// a developer's real app and CLI share.
// ---------------------------------------------------------------------------

const CLI_BUNDLE = join(process.cwd(), 'out', 'main', 'cli.js')
const SMOKE_ENV_FILE = join(tmpdir(), 'specular-smoke-env.json')

function smokePort(): number {
  const raw = readFileSync(SMOKE_ENV_FILE, 'utf8')
  return (JSON.parse(raw) as { port: number }).port
}

function discoveryFile(): string {
  return join(tmpdir(), `specular-mcp-smoke-${smokePort()}.json`)
}

export interface CliResult {
  code: number
  stdout: string
  stderr: string
  /** stdout parsed as JSON, or undefined if it was not valid JSON. */
  json: unknown
}

/**
 * Run the built CLI against the smoke app. Mirrors how an agent invokes it:
 * `specular <args...>`, optional stdin, JSON or text on stdout, errors on stderr.
 */
export function runCli(args: string[], opts: { input?: string } = {}): CliResult {
  if (!existsSync(CLI_BUNDLE)) {
    throw new Error(
      `CLI bundle missing at ${CLI_BUNDLE}. Run \`pnpm build:cli\` before the CLI probes ` +
        `(the canonical command is \`pnpm build:cli && pnpm test:smoke -- cli\`).`,
    )
  }
  const res = spawnSync('node', [CLI_BUNDLE, ...args], {
    encoding: 'utf8',
    input: opts.input,
    timeout: 20_000,
    env: {
      ...process.env,
      SPECULAR_DISCOVERY_FILE: discoveryFile(),
      // Unique identity per call so the app's presence layer never evicts one
      // probe's cursor for another's.
      SPECULAR_SESSION_ID: `probe-${randomUUID()}`,
      SPECULAR_CLIENT_NAME: `probe-${randomUUID().slice(0, 8)}`,
    },
  })
  const stdout = res.stdout ?? ''
  let json: unknown
  try {
    json = JSON.parse(stdout)
  } catch {
    json = undefined
  }
  return { code: res.status ?? -1, stdout, stderr: res.stderr ?? '', json }
}

import { existsSync } from 'fs'
import { dirname, join } from 'path'
import { setClientName } from './shared/app-client'
import { notifySessionState } from './shared/app-client'
import { dispatch } from './cli-commands'

setClientName('specular-cli')

// Point the browse resolver at the bundled agent-browser before any command
// runs. The Electron main does this via configureBundledAgentBrowser(), but the
// CLI is a standalone node process that never boots the app — without this it
// falls through to PATH and picks up a stale global install (or nothing at all).
// Packaged: bin/ sits beside cli.js in Contents/Resources. Dev: cli.js is in
// out/main/, the binary in resources/bin at the repo root.
function configureBundledAgentBrowser(): void {
  if (process.env.AGENT_BROWSER_PATH) return
  const candidates = [
    join(__dirname, 'bin', 'agent-browser'),
    join(__dirname, '..', '..', 'resources', 'bin', 'agent-browser'),
  ]
  const bundled = candidates.find(existsSync)
  if (bundled) process.env.AGENT_BROWSER_PATH = bundled
}

configureBundledAgentBrowser()

async function main(): Promise<void> {
  // Ping the session open — but never close it explicitly.
  // The server's 15s session timeout handles cleanup, and the cursor
  // auto-transitions to "Thinking…" between invocations so it stays
  // visible across a chain of tool calls.
  await notifySessionState('/mcp/session/open')
  try {
    const exitCode = await dispatch(process.argv.slice(2))
    process.exitCode = exitCode
  } catch (error) {
    process.stderr.write(
      `error: ${error instanceof Error ? error.message : String(error)}\n`,
    )
    process.exitCode = 1
  }
}

function shutdown(exitCode: number): void {
  setTimeout(() => process.exit(exitCode), 50)
}

process.on('SIGINT', () => shutdown(0))
process.on('SIGTERM', () => shutdown(0))

main().catch((error) => {
  process.stderr.write(
    `fatal: ${error instanceof Error ? error.message : String(error)}\n`,
  )
  process.exit(1)
})

/**
 * The `specular` command: the CLI an agent in a terminal drives the app
 * with. It is the Electron app's CLI source, which speaks to whichever app
 * answers the routes in `~/.specular/specular-mcp.json`.
 */

import { dispatch } from '../../../src/main/cli-commands'
import { notifySessionState, setClientName } from '../../../src/main/shared/app-client'

export async function cli(): Promise<void> {
  setClientName('specular-cli')
  // The session is opened and never closed: the app times it out, so a
  // cursor stays up across a chain of commands.
  await notifySessionState('/mcp/session/open')
  try {
    process.exitCode = await dispatch(process.argv.slice(3))
  } catch (error) {
    process.stderr.write(`error: ${error instanceof Error ? error.message : String(error)}\n`)
    process.exitCode = 1
  }
}

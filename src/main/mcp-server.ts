import { Server } from '@modelcontextprotocol/sdk/server/index.js'
import packageJson from '../../package.json'

// Re-export shared app-client for existing consumers
export {
  sessionId,
  getClientName,
  callApp,
  withTargetTab,
  notifySessionState,
  startHeartbeat,
  stopHeartbeat,
} from './shared/app-client'

/** The shape every tool handler resolves to — a `CallToolResult`. The index
 *  signature matches the SDK's own (it allows server-side task/metadata
 *  fields we never set) so a plain object literal satisfies both this type
 *  and `setRequestHandler`'s wider expected return type. */
export interface ToolResult {
  content: Array<
    | { type: 'text'; text: string }
    | { type: 'image'; data: string; mimeType: string }
  >
  isError?: boolean
  [key: string]: unknown
}

export type ToolHandler = (args: Record<string, unknown>) => Promise<ToolResult>

export function asText(payload: unknown): ToolResult {
  return {
    content: [{ type: 'text' as const, text: JSON.stringify(payload, null, 2) }],
  }
}

/** A tool-level error (bad arguments), reported through the result rather
 *  than thrown — thrown errors surface to the client as protocol errors,
 *  which reads worse for "you passed something invalid" than a normal
 *  result with `isError: true`. */
export function toolError(message: string): ToolResult {
  return {
    content: [{ type: 'text' as const, text: message }],
    isError: true,
  }
}

export const server = new Server(
  {
    name: 'specular-mcp',
    // Sourced from package.json (not app.getVersion()) — the MCP helper is a
    // plain node process with no Electron `app` module available.
    version: packageJson.version,
  },
  {
    capabilities: {
      tools: {},
    },
  },
)

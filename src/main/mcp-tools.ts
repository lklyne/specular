import type { Server } from '@modelcontextprotocol/sdk/server/index.js'
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from '@modelcontextprotocol/sdk/types.js'
import { withTargetTab, type ToolHandler } from './mcp-server'
import { toolSchemas } from './mcp-tool-schemas'
import { canvasToolHandlers } from './mcp-tools-canvas'
import { arrangeToolHandlers } from './mcp-tools-arrange'
import { tabToolHandlers } from './mcp-tools-tabs'
import { annotationToolHandlers } from './mcp-tools-annotations'
import { captureToolHandlers } from './mcp-tools-capture'
import { designToolHandlers } from './mcp-tools-design'
import { presenceToolHandlers } from './mcp-tools-presence'
import { browseToolHandlers } from './mcp-tools-browse'

// One handler table assembled from the per-area files above — each area owns
// its own request shaping, this just dispatches by tool name. Grouped by area
// (canvas / arrange / tabs / annotations / capture / design / presence /
// browse) rather than one file, mirroring the specular skill's own sections.
export const toolHandlers: Record<string, ToolHandler> = {
  ...canvasToolHandlers,
  ...arrangeToolHandlers,
  ...tabToolHandlers,
  ...annotationToolHandlers,
  ...captureToolHandlers,
  ...designToolHandlers,
  ...presenceToolHandlers,
  ...browseToolHandlers,
}

export function registerTools(mcpServer: Server): void {
  mcpServer.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: toolSchemas,
  }))

  mcpServer.setRequestHandler(CallToolRequestSchema, async (request) => {
    const handler = toolHandlers[request.params.name]
    if (!handler) throw new Error(`Unknown tool: ${request.params.name}`)
    const args = (request.params.arguments ?? {}) as Record<string, unknown>
    // Scoped per call, not module-level — the MCP helper is long-lived and
    // may service overlapping tool calls (see withTargetTab).
    return withTargetTab(args.tab as string | undefined, () => handler(args))
  })
}

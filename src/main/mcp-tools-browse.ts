// ---------------------------------------------------------------------------
// MCP tool — the agent-browser passthrough (CLI's snapshot/click/fill/etc.)
// ---------------------------------------------------------------------------

import { handleBrowse } from './mcp-browse'
import type { ToolHandler } from './mcp-server'

export const browseToolHandlers: Record<string, ToolHandler> = {
  browse: (args) => handleBrowse(args),
}

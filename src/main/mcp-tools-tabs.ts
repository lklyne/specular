// ---------------------------------------------------------------------------
// MCP tools — tab (canvas) identity, mirroring `specular tab` (issue #360)
// ---------------------------------------------------------------------------

import { asText, callApp, type ToolHandler } from './mcp-server'

export const tabToolHandlers: Record<string, ToolHandler> = {
  list_tabs: async () => asText(await callApp('/tabs')),

  create_tab: async (args) =>
    asText(await callApp('/tabs', { method: 'POST', body: JSON.stringify({ name: args.name }) })),

  switch_tab: async (args) =>
    asText(await callApp('/tabs/switch', { method: 'POST', body: JSON.stringify({ ref: args.ref }) })),

  delete_tab: async (args) =>
    asText(await callApp('/tabs/delete', { method: 'POST', body: JSON.stringify({ ref: args.ref }) })),
}

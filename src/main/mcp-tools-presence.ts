// ---------------------------------------------------------------------------
// MCP tools — bracketing a multi-step task so the agent's cursor holds
// position between calls instead of idling out (see presence-cursor.ts)
// ---------------------------------------------------------------------------

import { asText, callApp, getClientName, sessionId, type ToolHandler } from './mcp-server'

export const presenceToolHandlers: Record<string, ToolHandler> = {
  start_task: async (args) => {
    await callApp('/session/presence', {
      method: 'POST',
      body: JSON.stringify({
        sessionId,
        clientName: getClientName(),
        eventType: 'start',
        hold: true,
        taskLabel: args.label,
        surface: 'canvas',
        phase: 'thinking',
      }),
    })
    return asText({ ok: true })
  },

  finish_task: async () => {
    await callApp('/session/presence', {
      method: 'POST',
      body: JSON.stringify({
        sessionId,
        clientName: getClientName(),
        eventType: 'done',
      }),
    })
    return asText({ ok: true })
  },
}

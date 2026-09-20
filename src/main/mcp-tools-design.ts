// ---------------------------------------------------------------------------
// MCP tools — design system manifests and component-state layouts
// ---------------------------------------------------------------------------

import { asText, callApp, type ToolHandler } from './mcp-server'

export const designToolHandlers: Record<string, ToolHandler> = {
  register_design_system: async (args) =>
    asText(
      await callApp('/design-system/register', {
        method: 'POST',
        body: JSON.stringify({ manifest: args.manifest }),
      }),
    ),

  get_design_system: async () => asText(await callApp('/design-system')),

  layout_component_states: async (args) =>
    asText(
      await callApp('/tasks/component-states', {
        method: 'POST',
        body: JSON.stringify({
          component: args.component,
          url: args.url,
          vary: args.vary,
          values: args.values,
          states: args.states,
          tokens: args.tokens,
          selector: args.selector,
          anchor: args.anchor ?? 'selection_or_empty_region',
          focus: args.focus ?? true,
          label: args.label,
        }),
      }),
    ),
}

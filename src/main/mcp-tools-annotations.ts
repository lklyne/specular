// ---------------------------------------------------------------------------
// MCP tools — annotation (comment thread) lifecycle
// ---------------------------------------------------------------------------

import { asText, callApp, type ToolHandler } from './mcp-server'
import { getAnnotationsSlim, getAnnotationDetail } from './shared/entity-ops'

export const annotationToolHandlers: Record<string, ToolHandler> = {
  create_annotation: async (args) =>
    asText(
      await callApp('/annotations', {
        method: 'POST',
        body: JSON.stringify({
          text: args.text,
          kind: args.kind,
          anchor: args.anchor,
          author: 'agent',
          metadata: args.metadata,
        }),
      }),
    ),

  get_annotations: async (args) => {
    const result = await getAnnotationsSlim({
      status: args.status as string | undefined,
      url: args.url as string | undefined,
      page_id: args.page_id as string | undefined,
    })
    return { content: [{ type: 'text' as const, text: JSON.stringify(result, null, 2) }] }
  },

  get_annotation_detail: (args) =>
    getAnnotationDetail({
      annotation_id: args.annotation_id as string,
      include_screenshot: args.include_screenshot as boolean | undefined,
    }),

  acknowledge_annotation: async (args) =>
    asText(
      await callApp(`/annotations/${args.annotation_id}/acknowledge`, {
        method: 'POST',
        body: '{}',
      }),
    ),

  resolve_annotation: async (args) =>
    asText(
      await callApp(`/annotations/${args.annotation_id}/resolve`, {
        method: 'POST',
        body: '{}',
      }),
    ),

  dismiss_annotation: async (args) =>
    asText(
      await callApp(`/annotations/${args.annotation_id}/dismiss`, {
        method: 'POST',
        body: JSON.stringify({ reason: args.reason }),
      }),
    ),

  reply_to_annotation: async (args) =>
    asText(
      await callApp(`/annotations/${args.annotation_id}/reply`, {
        method: 'POST',
        body: JSON.stringify({ author: 'agent', text: args.text }),
      }),
    ),

  // One region comment over a multi-selection's union bounds (mirrors
  // `specular annotate-selection`) — no ids uses the current selection.
  annotate_selection: async (args) => {
    const entityIds = args.entity_ids as string[] | undefined
    return asText(
      await callApp('/selection/annotate', {
        method: 'POST',
        body: JSON.stringify({
          text: args.text,
          ...(entityIds && entityIds.length > 0 ? { entityIds } : {}),
        }),
      }),
    )
  },
}

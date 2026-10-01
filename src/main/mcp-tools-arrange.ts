// ---------------------------------------------------------------------------
// MCP tools — connecting, grouping, and arranging existing entities
// ---------------------------------------------------------------------------

import { asText, callApp, type ToolHandler } from './mcp-server'

export const arrangeToolHandlers: Record<string, ToolHandler> = {
  link_pages: async (args) =>
    asText(
      await callApp('/edges/create', {
        method: 'POST',
        body: JSON.stringify({ edges: args.edges }),
      }),
    ),

  unlink_pages: async (args) =>
    asText(
      await callApp('/edges/delete', {
        method: 'POST',
        body: JSON.stringify({ edgeIds: args.edgeIds }),
      }),
    ),

  focus_pages: async (args) =>
    asText(
      await callApp('/camera/focus', {
        method: 'POST',
        body: JSON.stringify({
          pageIds: args.pageIds,
          groupIds: args.groupIds,
          bounds: args.bounds,
        }),
      }),
    ),

  create_group: async (args) =>
    asText(
      await callApp('/groups/create', {
        method: 'POST',
        body: JSON.stringify({
          entityIds: args.entity_ids,
          label: args.label,
        }),
      }),
    ),

  ungroup_group: async (args) =>
    asText(
      await callApp('/groups/ungroup', {
        method: 'POST',
        body: JSON.stringify({
          groupId: args.group_id,
        }),
      }),
    ),

  delete_groups: async (args) =>
    asText(
      await callApp('/groups/delete', {
        method: 'POST',
        body: JSON.stringify({
          groupIds: args.group_ids,
          deleteMemberPages: args.delete_member_pages ?? true,
          focusAfter: args.focus_after ?? false,
        }),
      }),
    ),

  arrange_entities: async (args) => {
    const entityIds = args.entity_ids as string[] | undefined
    return asText(
      await callApp('/selection/arrange', {
        method: 'POST',
        body: JSON.stringify({
          mode: args.mode,
          ...(entityIds && entityIds.length > 0 ? { entityIds } : {}),
          ...(args.gap !== undefined ? { gap: args.gap } : {}),
          ...(args.cols !== undefined ? { cols: args.cols } : {}),
        }),
      }),
    )
  },

  // A single group id converts it in place (mirrors `specular auto-layout`);
  // otherwise the ids become a fresh group.
  auto_layout: async (args) => {
    const entityIds = args.entity_ids as string[]
    const onlyGroup =
      entityIds.length === 1 && entityIds[0].startsWith('group_') ? entityIds[0] : undefined
    return asText(
      await callApp('/groups/auto-layout', {
        method: 'POST',
        body: JSON.stringify(
          onlyGroup
            ? { groupId: onlyGroup, label: args.label, gap: args.gap }
            : { entityIds, label: args.label, gap: args.gap },
        ),
      }),
    )
  },
}

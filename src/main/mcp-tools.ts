import type { Server } from '@modelcontextprotocol/sdk/server/index.js'
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from '@modelcontextprotocol/sdk/types.js'
import {
  DEFAULT_BREAKPOINT_PRESET_LABELS,
} from '../shared/constants'
import { callApp, asText } from './mcp-server'
import { toolSchemas } from './mcp-tool-schemas'
import { handleBrowse } from './mcp-browse'
import { upsertEntities, getAnnotationsSlim, getAnnotationDetail } from './shared/entity-ops'

// ---------------------------------------------------------------------------
// Tool registration
// ---------------------------------------------------------------------------

export function registerTools(mcpServer: Server): void {
  mcpServer.setRequestHandler(ListToolsRequestSchema, async () => ({
    tools: toolSchemas,
  }))

  mcpServer.setRequestHandler(CallToolRequestSchema, async (request) => {
  const args = request.params.arguments ?? {}

  switch (request.params.name) {
    case 'get_workspace':
      return asText(await callApp('/canvas'))
    case 'get_selection':
      return asText(await callApp('/selection'))
    case 'find_placement':
      return asText(
        await callApp('/layout/find-placement', {
          method: 'POST',
          body: JSON.stringify({
            width: args.width,
            height: args.height,
            anchor: args.anchor ?? 'selection_or_empty_region',
          }),
        }),
      )
    case 'apply_task_layout':
      return asText(
        await callApp('/tasks/apply', {
          method: 'POST',
          body: JSON.stringify({
            taskKind: args.task_kind ?? 'breakpoint_map',
            input: {
              url: args.url,
              presets: args.presets ?? DEFAULT_BREAKPOINT_PRESET_LABELS,
              label: args.label,
            },
            options: {
              anchor: args.anchor ?? 'selection_or_empty_region',
              focus: args.focus ?? true,
            },
          }),
        }),
      )
    case 'upsert_entities': {
      const results = await upsertEntities(args.items as Array<Record<string, unknown>>)
      return { content: [{ type: 'text', text: JSON.stringify(results) }] }
    }
    case 'link_pages':
      return asText(
        await callApp('/edges/create', {
          method: 'POST',
          body: JSON.stringify({ edges: args.edges }),
        }),
      )
    case 'unlink_pages':
      return asText(
        await callApp('/edges/delete', {
          method: 'POST',
          body: JSON.stringify({ edgeIds: args.edgeIds }),
        }),
      )
    case 'focus_pages':
      return asText(
        await callApp('/camera/focus', {
          method: 'POST',
          body: JSON.stringify({
            pageIds: args.pageIds,
            groupIds: args.groupIds,
            bounds: args.bounds,
          }),
        }),
      )
    case 'create_group':
      return asText(
        await callApp('/groups/create', {
          method: 'POST',
          body: JSON.stringify({
            entityIds: args.entity_ids,
            label: args.label,
          }),
        }),
      )
    case 'ungroup_group':
      return asText(
        await callApp('/groups/ungroup', {
          method: 'POST',
          body: JSON.stringify({
            groupId: args.group_id,
          }),
        }),
      )
    case 'delete_groups':
      return asText(
        await callApp('/groups/delete', {
          method: 'POST',
          body: JSON.stringify({
            groupIds: args.group_ids,
            deleteMemberPages: args.delete_member_pages ?? true,
            focusAfter: args.focus_after ?? false,
          }),
        }),
      )
    case 'register_design_system':
      return asText(
        await callApp('/design-system/register', {
          method: 'POST',
          body: JSON.stringify({ manifest: args.manifest }),
        }),
      )
    case 'get_design_system':
      return asText(await callApp('/design-system'))
    case 'layout_component_states':
      return asText(
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
      )
    case 'browse':
      return handleBrowse(args)
    case 'create_annotation':
      return asText(
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
      )
    case 'get_annotations': {
      const result = await getAnnotationsSlim({
        status: args.status as string | undefined,
        url: args.url as string | undefined,
        page_id: args.page_id as string | undefined,
      })
      return {
        content: [{ type: 'text' as const, text: JSON.stringify(result, null, 2) }],
      }
    }
    case 'get_annotation_detail':
      return getAnnotationDetail({
        annotation_id: args.annotation_id as string,
        include_screenshot: args.include_screenshot as boolean | undefined,
      })
    case 'acknowledge_annotation':
      return asText(
        await callApp(`/annotations/${args.annotation_id}/acknowledge`, {
          method: 'POST',
          body: '{}',
        }),
      )
    case 'resolve_annotation':
      return asText(
        await callApp(`/annotations/${args.annotation_id}/resolve`, {
          method: 'POST',
          body: '{}',
        }),
      )
    case 'dismiss_annotation':
      return asText(
        await callApp(`/annotations/${args.annotation_id}/dismiss`, {
          method: 'POST',
          body: JSON.stringify({ reason: args.reason }),
        }),
      )
    case 'reply_to_annotation':
      return asText(
        await callApp(`/annotations/${args.annotation_id}/reply`, {
          method: 'POST',
          body: JSON.stringify({ author: 'agent', text: args.text }),
        }),
      )
    case 'get_text_entities':
      return asText(await callApp('/text-entities'))
    case 'get_file_entities':
      return asText(await callApp('/file-entities'))
    case 'delete_entities':
      return asText(
        await callApp('/canvas/apply', {
          method: 'POST',
          body: JSON.stringify({
            // Kind resolves from the doc by id (ADR 0019 §4); the per-kind
            // payload shape is gone.
            delete: (args.items as Array<{ id: string }> | undefined)?.map((i) => i.id) ?? [],
          }),
        }),
      )
    case 'start_recording':
      return asText(
        await callApp('/recording/start', {
          method: 'POST',
          body: JSON.stringify({
            pageId: args.page_id,
            outputPath: args.output_path,
            fps: args.fps,
            quality: args.quality,
          }),
        }),
      )
    case 'stop_recording':
      return asText(
        await callApp('/recording/stop', { method: 'POST' }),
      )
    case 'get_recording_status':
      return asText(await callApp('/recording/status'))
    case 'trim_recording':
      return asText(
        await callApp('/recording/trim', {
          method: 'POST',
          body: JSON.stringify({
            inputPath: args.input_path,
            outputPath: args.output_path,
            minIdleMs: args.min_idle_ms,
            idleSpeedFactor: args.idle_speed_factor,
          }),
        }),
      )
    default:
      throw new Error(`Unknown tool: ${request.params.name}`)
  }
})
}

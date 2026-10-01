// ---------------------------------------------------------------------------
// MCP tools — reading and mutating the canvas document (ADR 0019)
// ---------------------------------------------------------------------------

import { DEFAULT_BREAKPOINT_PRESET_LABELS } from '../shared/constants'
import type { LayoutDirective } from '../shared/types'
import { validateLayoutDirective } from '../shared/layout-directive'
import { asText, callApp, toolError, type ToolHandler, type ToolResult } from './mcp-server'
import { upsertEntities, applyPatch, type CanvasPatch } from './shared/entity-ops'

function withValidatedLayout(
  toolName: string,
  layout: unknown,
  run: (directive: LayoutDirective | undefined) => Promise<ToolResult>,
): Promise<ToolResult> {
  if (layout === undefined) return run(undefined)
  const err = validateLayoutDirective(layout)
  if (err) return Promise.resolve(toolError(`${toolName}: ${err}`))
  return run(layout as LayoutDirective)
}

export const canvasToolHandlers: Record<string, ToolHandler> = {
  get_workspace: async () => asText(await callApp('/canvas')),
  get_selection: async () => asText(await callApp('/selection')),
  get_text_entities: async () => asText(await callApp('/text-entities')),
  get_file_entities: async () => asText(await callApp('/file-entities')),

  find_placement: async (args) =>
    asText(
      await callApp('/layout/find-placement', {
        method: 'POST',
        body: JSON.stringify({
          width: args.width,
          height: args.height,
          anchor: args.anchor ?? 'selection_or_empty_region',
        }),
      }),
    ),

  apply_task_layout: async (args) =>
    asText(
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
    ),

  upsert_entities: (args) =>
    withValidatedLayout('upsert_entities', args.layout, async (directive) =>
      asText(
        await upsertEntities(
          args.items as Array<Record<string, unknown>>,
          directive ? { directive } : undefined,
        ),
      ),
    ),

  apply_patch: (args) =>
    withValidatedLayout('apply_patch', args.layout, async (directive) => {
      const patch: CanvasPatch = {
        entities: args.entities as CanvasPatch['entities'],
        edges: args.edges as CanvasPatch['edges'],
        delete: args.delete as string[] | undefined,
        layout: directive,
      }
      return asText(await applyPatch(patch))
    }),

  delete_entities: async (args) =>
    asText(
      await callApp('/canvas/apply', {
        method: 'POST',
        body: JSON.stringify({
          // Kind resolves from the doc by id (ADR 0019 §4); the per-kind
          // payload shape is gone.
          delete: (args.items as Array<{ id: string }> | undefined)?.map((i) => i.id) ?? [],
        }),
      }),
    ),
}

import { describe, expect, it } from 'vitest'
import { VERBS } from '../../src/main/cli-commands'
import { toolSchemas } from '../../src/main/mcp-tool-schemas'
import { toolHandlers } from '../../src/main/mcp-tools'

// A CLI verb landing without a decided MCP story is exactly the drift this
// suite exists to catch — every VERBS key must either map onto an MCP tool
// (VERB_TO_TOOLS) or carry a one-line reason it deliberately has no tool
// (EXCLUDED_VERBS). Add a case to whichever list is correct when a verb
// changes; do not delete an assertion to make this pass.
const VERB_TO_TOOLS: Record<string, string[]> = {
  canvas: ['get_workspace'],
  tab: ['list_tabs', 'create_tab', 'switch_tab', 'delete_tab'],
  selection: ['get_selection'],
  'find-placement': ['find_placement'],
  breakpoints: ['apply_task_layout'],
  upsert: ['upsert_entities'],
  apply: ['apply_patch'],
  update: ['upsert_entities'], // an id-only item (no `kind`) is an update — same tool
  delete: ['delete_entities'],
  arrange: ['arrange_entities'],
  focus: ['focus_pages'],
  link: ['link_pages'],
  unlink: ['unlink_pages'],
  group: ['create_group'],
  ungroup: ['ungroup_group'],
  'auto-layout': ['auto_layout'],
  annotations: ['get_annotations'],
  annotation: ['get_annotation_detail'],
  annotate: ['create_annotation'],
  'annotate-selection': ['annotate_selection'],
  ack: ['acknowledge_annotation'],
  resolve: ['resolve_annotation'],
  dismiss: ['dismiss_annotation'],
  reply: ['reply_to_annotation'],
  record: ['start_recording', 'stop_recording', 'get_recording_status', 'trim_recording'],
  presence: ['start_task', 'finish_task'],
  'print-pdf': ['print_pdf'],
  'design-system': ['get_design_system'],
  'register-design-system': ['register_design_system'],
  'component-states': ['layout_component_states'],
  // CLI parity with the MCP browse tool: a raw (possibly chained) command
  // string forwarded to handleBrowse exactly as the tool does.
  browse: ['browse'],
}

const EXCLUDED_VERBS: Record<string, string> = {
  workspace: 'hidden alias for canvas (get_workspace), kept so old agent skills do not break',
  add: 'sugar over upsert_entities — no separate MCP tool',
  skills: 'agent-browser meta-verb (lists driver docs) — not a canvas operation',
  // Browser shortcut + read-only passthrough verbs: all covered by the
  // single `browse` tool, which forwards a raw agent-browser command.
  snapshot: 'covered by the browse tool',
  click: 'covered by the browse tool',
  fill: 'covered by the browse tool',
  type: 'covered by the browse tool',
  select: 'covered by the browse tool',
  screenshot: 'covered by the browse tool',
  scroll: 'covered by the browse tool',
  wait: 'covered by the browse tool',
  get: 'covered by the browse tool',
  console: 'covered by the browse tool',
  errors: 'covered by the browse tool',
  'query-elements': 'covered by the browse tool',
}

describe('MCP / CLI parity', () => {
  it('accounts for every CLI verb: mapped to a tool, or explicitly excluded with a reason', () => {
    const unaccounted = Object.keys(VERBS).filter(
      (verb) => !VERB_TO_TOOLS[verb] && !EXCLUDED_VERBS[verb],
    )
    expect(unaccounted).toEqual([])
  })

  it('every mapped tool actually exists in toolSchemas', () => {
    const toolNames = new Set(toolSchemas.map((tool) => tool.name))
    const missing = Object.values(VERB_TO_TOOLS)
      .flat()
      .filter((name) => !toolNames.has(name))
    expect(missing).toEqual([])
  })

  it('every schema in toolSchemas has a registered handler', () => {
    const unhandled = toolSchemas.map((tool) => tool.name).filter((name) => !toolHandlers[name])
    expect(unhandled).toEqual([])
  })

  it('every registered handler has a schema in toolSchemas', () => {
    const toolNames = new Set(toolSchemas.map((tool) => tool.name))
    const undocumented = Object.keys(toolHandlers).filter((name) => !toolNames.has(name))
    expect(undocumented).toEqual([])
  })
})

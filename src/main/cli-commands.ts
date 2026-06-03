import { randomUUID } from 'crypto'
import { DEFAULT_BREAKPOINT_PRESET_LABELS } from '../shared/constants'
import { validateLayoutDirective } from '../shared/types'
import { createWireframeNode, type WireframePaletteType } from '../shared/wireframe/wireframe-node-factory'
import type { WireframeNode } from '../shared/wireframe/wireframe-types'
import type { WireframeOp } from './runtime/wireframe-content-state'
import { callApp } from './shared/app-client'
import { handleBrowse, shellQuote } from './shared/browse-handler'
import { upsertEntities, type UpsertOptions, getAnnotationsSlim, getAnnotationDetail } from './shared/entity-ops'
import { printJson, printText, printError, printContentBlocks } from './cli-output'
import { parseArgs, type ParsedArgs } from './cli-parser'
import { emitPresenceForVerb } from './cli-presence'

// ---------------------------------------------------------------------------
// Verb handlers
// ---------------------------------------------------------------------------

type VerbHandler = (args: ParsedArgs) => Promise<number>

function pageId(args: ParsedArgs): string | undefined {
  return args.flags.page ?? args.flags.f ?? undefined
}

// --- Canvas verbs ---

const workspace: VerbHandler = async () => {
  printJson(await callApp('/workspace'))
  return 0
}

const selection: VerbHandler = async () => {
  printJson(await callApp('/selection'))
  return 0
}

const findPlacement: VerbHandler = async (args) => {
  printJson(await callApp('/layout/find-placement', {
    method: 'POST',
    body: JSON.stringify({
      width: Number(args.flags.width) || 800,
      height: Number(args.flags.height) || 600,
      anchor: args.flags.anchor ?? 'selection_or_empty_region',
    }),
  }))
  return 0
}

const breakpoints: VerbHandler = async (args) => {
  const url = args.positional[0]
  if (!url) { printError('usage: specular breakpoints <url>'); return 1 }
  printJson(await callApp('/tasks/apply', {
    method: 'POST',
    body: JSON.stringify({
      taskKind: 'breakpoint_map',
      input: {
        url,
        presets: args.flags.presets?.split(',') ?? DEFAULT_BREAKPOINT_PRESET_LABELS,
        label: args.flags.label,
      },
      options: {
        anchor: args.flags.anchor ?? 'selection_or_empty_region',
        focus: !args.boolFlags.has('no-focus'),
      },
    }),
  }))
  return 0
}

const upsert: VerbHandler = async (args) => {
  // Read JSON from stdin: either an array of items (legacy) or
  // { layout: LayoutDirective, items: [...] } (directive form).
  if (!args.boolFlags.has('json')) {
    printError('usage: specular upsert --json < items.json')
    return 1
  }
  const input = await readStdin()
  const parsed = JSON.parse(input)
  const options: UpsertOptions = {}
  let items: Array<Record<string, unknown>>
  if (Array.isArray(parsed)) {
    items = parsed
  } else if (parsed && typeof parsed === 'object' && Array.isArray(parsed.items)) {
    items = parsed.items
    if (parsed.layout) {
      const err = validateLayoutDirective(parsed.layout)
      if (err) {
        printError(`upsert: ${err}`)
        return 1
      }
      options.directive = parsed.layout as UpsertOptions['directive']
    }
  } else {
    printError('upsert: expected an array of items or { layout, items }')
    return 1
  }
  // CLI flags still work for the legacy auto-placement path.
  if (args.flags.layout && !options.directive) {
    options.layout = args.flags.layout as UpsertOptions['layout']
  }
  if (args.flags.gap && !options.directive) options.gap = Number(args.flags.gap)
  printJson(await upsertEntities(items, options))
  return 0
}

const create: VerbHandler = async (args) => {
  const subverb = args.positional[0]
  if (subverb === 'page') {
    const url = args.positional[1]
    if (!url) { printError('usage: specular create page <url>'); return 1 }
    const item: Record<string, unknown> = { kind: 'page', url }
    item.presetIndex = args.flags.preset ? Number(args.flags.preset) : 6 // default to Laptop
    if (args.flags.at) {
      const [x, y] = args.flags.at.split(',').map(Number)
      if (!isNaN(x)) item.canvasX = x
      if (!isNaN(y)) item.canvasY = y
    }
    if (args.boolFlags.has('landscape')) item.orientation = 'landscape'
    if (args.boolFlags.has('no-device-frame')) item.showDeviceFrame = false
    printJson(await upsertEntities([item]))
    return 0
  }
  if (subverb === 'note') {
    const text = args.positional.slice(1).join(' ')
    if (!text) { printError('usage: specular create note <text>'); return 1 }
    const item: Record<string, unknown> = { kind: 'text', text }
    if (args.flags.at) {
      const [x, y] = args.flags.at.split(',').map(Number)
      if (!isNaN(x)) item.canvasX = x
      if (!isNaN(y)) item.canvasY = y
    }
    if (args.flags.color) item.color = args.flags.color
    // --kind text: force text entity even for long content
    // --kind file: force file entity even for short content
    if (args.flags.kind === 'text') item.forceKind = true
    if (args.flags.kind === 'file') {
      // Route directly to file entity by removing kind override protection
      // and ensuring it gets auto-routed regardless of length
      item.kind = 'text'
      item.forceKind = false
      // Force auto-route by setting a flag the grouping loop checks
      item._forceFile = true
    }
    printJson(await upsertEntities([item]))
    return 0
  }
  printError('usage: specular create <page|note> ...')
  return 1
}

const update: VerbHandler = async (args) => {
  const id = args.positional[0]
  if (!id) { printError('usage: specular update <id> [--preset N] [--at x,y] [--text T] [--color C]'); return 1 }
  const kind = kindFromId(id)
  const item: Record<string, unknown> = { kind, id }
  if (args.flags.at) {
    const [x, y] = args.flags.at.split(',').map(Number)
    if (!isNaN(x)) item.canvasX = x
    if (!isNaN(y)) item.canvasY = y
  }
  // Page-specific flags
  if (args.flags.preset) item.presetIndex = Number(args.flags.preset)
  if (args.boolFlags.has('landscape')) item.orientation = 'landscape'
  if (args.boolFlags.has('portrait')) item.orientation = 'portrait'
  if (args.boolFlags.has('no-device-frame')) item.showDeviceFrame = false
  // Text note flags
  if (args.flags.text) item.text = args.flags.text
  if (args.flags.color) item.color = args.flags.color
  printJson(await upsertEntities([item]))
  return 0
}

function kindFromId(id: string): 'page' | 'text' | 'file' | 'group' {
  if (id.startsWith('page_')) return 'page'
  if (id.startsWith('text_')) return 'text'
  if (id.startsWith('group_')) return 'group'
  return 'file'
}

const deleteEntities: VerbHandler = async (args) => {
  if (args.boolFlags.has('json')) {
    const input = await readStdin()
    const items = JSON.parse(input) as Array<{ id: string; kind?: string }>
    const withKind = items.map((item) => ({
      ...item,
      kind: item.kind ?? kindFromId(item.id),
    }))
    printJson(await callApp('/entities/delete', {
      method: 'POST',
      body: JSON.stringify({ items: withKind }),
    }))
  } else if (args.positional.length > 0) {
    const items = args.positional.map((id) => ({ id, kind: kindFromId(id) }))
    printJson(await callApp('/entities/delete', {
      method: 'POST',
      body: JSON.stringify({ items }),
    }))
  } else {
    printError('usage: specular delete <id> [id...] or specular delete --json')
    return 1
  }
  return 0
}

const focus: VerbHandler = async (args) => {
  if (args.positional.length === 0) { printError('usage: specular focus <pageId> [pageId...]'); return 1 }
  printJson(await callApp('/camera/focus', {
    method: 'POST',
    body: JSON.stringify({ pageIds: args.positional }),
  }))
  return 0
}

const link: VerbHandler = async (args) => {
  if (args.positional.length >= 2) {
    const [fromEntityId, toEntityId] = args.positional
    const edge: Record<string, unknown> = { fromEntityId, toEntityId, kind: 'connection' }
    if (args.flags.label) edge.label = args.flags.label
    printJson(await callApp('/edges/create', {
      method: 'POST',
      body: JSON.stringify({ edges: [edge] }),
    }))
    return 0
  }
  if (args.positional.length === 1 || (args.positional.length === 0 && process.stdin.isTTY)) {
    printError('usage: specular link <fromId> <toId> [--label <text>]  (or pipe a JSON edges array on stdin)')
    return 1
  }
  const input = await readStdin()
  printJson(await callApp('/edges/create', {
    method: 'POST',
    body: JSON.stringify({ edges: JSON.parse(input) }),
  }))
  return 0
}

const unlink: VerbHandler = async (args) => {
  if (args.positional.length === 0) { printError('usage: specular unlink <edgeId> [edgeId...]'); return 1 }
  printJson(await callApp('/edges/delete', {
    method: 'POST',
    body: JSON.stringify({ edgeIds: args.positional }),
  }))
  return 0
}

const group: VerbHandler = async (args) => {
  if (args.positional.length === 0) { printError('usage: specular group <entityId> [entityId...]'); return 1 }
  printJson(await callApp('/groups/create', {
    method: 'POST',
    body: JSON.stringify({
      entityIds: args.positional,
      label: args.flags.label,
    }),
  }))
  return 0
}

const ungroup: VerbHandler = async (args) => {
  const groupId = args.positional[0]
  if (!groupId) { printError('usage: specular ungroup <groupId>'); return 1 }
  printJson(await callApp('/groups/ungroup', {
    method: 'POST',
    body: JSON.stringify({ groupId }),
  }))
  return 0
}

// Mark a group (or a fresh group made from 2+ entities) as an auto-layout row:
// children reflow left-to-right and can be drag-reordered. A single group id
// converts that group in place. (ADR 0015)
const autoLayout: VerbHandler = async (args) => {
  if (args.positional.length === 0) {
    printError('usage: specular auto-layout <entityId> [entityId...]  (or a single groupId)')
    return 1
  }
  const onlyGroup =
    args.positional.length === 1 && args.positional[0].startsWith('group_')
      ? args.positional[0]
      : undefined
  printJson(await callApp('/groups/auto-layout', {
    method: 'POST',
    body: JSON.stringify(
      onlyGroup
        ? { groupId: onlyGroup, label: args.flags.label }
        : { entityIds: args.positional, label: args.flags.label },
    ),
  }))
  return 0
}

// Even out gaps between 3+ loose entities along their dominant axis. Endpoints
// stay fixed; the middle items slide to equalize edge-to-edge spacing. (ADR 0015 D7)
const distribute: VerbHandler = async (args) => {
  if (args.positional.length > 0 && args.positional.length < 3) {
    printError('usage: specular distribute <entityId> <entityId> <entityId> [entityId...]  (or no args to use current selection)')
    return 1
  }
  const body = args.positional.length > 0
    ? JSON.stringify({ entityIds: args.positional })
    : JSON.stringify({})
  printJson(await callApp('/selection/distribute', { method: 'POST', body }))
  return 0
}

// --- Wireframe verbs (3.4 — agent CLI parity) ---

// Coerce a bare CLI value to its JSON-ish type: booleans, numbers, else string.
// Keeps `set frame gap 8` → 8 and `set toggle on true` → true while leaving
// `set text level h1` a string. Use --props for values that must stay strings
// despite looking numeric, or for arrays/objects.
function coerceValue(value: string): unknown {
  if (value === 'true') return true
  if (value === 'false') return false
  if (value !== '' && !Number.isNaN(Number(value))) return Number(value)
  return value
}

function parseKeyValues(tokens: string[]): Record<string, unknown> | null {
  if (tokens.length % 2 !== 0) return null
  const patch: Record<string, unknown> = {}
  for (let i = 0; i < tokens.length; i += 2) patch[tokens[i]] = coerceValue(tokens[i + 1])
  return patch
}

// Build a WireframeOp from the parsed verb + args, or print a usage error and
// return null. The op shape mirrors the canvas/IPC op set so humans and agents
// drive the same apply path.
function buildWireframeOp(verb: string, args: ParsedArgs): WireframeOp | null {
  const p = args.positional
  switch (verb) {
    case 'insert': {
      const parentId = p[2]
      const index = Number(p[3])
      if (!parentId || Number.isNaN(index)) {
        printError('usage: specular wireframe <target> insert <parentId> <index> <nodeType> | --node <json>')
        return null
      }
      let node: WireframeNode
      if (args.flags.node) {
        try {
          node = JSON.parse(args.flags.node) as WireframeNode
        } catch (err) {
          printError(`insert: invalid --node JSON: ${(err as Error).message}`)
          return null
        }
      } else {
        const nodeType = p[4]
        if (!nodeType) {
          printError('insert: provide a node type (text, button, frame, …) or --node <json>')
          return null
        }
        const id = args.flags.id ?? `${nodeType}-${randomUUID().slice(0, 8)}`
        node = createWireframeNode(nodeType as WireframePaletteType, () => id)
      }
      return { kind: 'insert', parentId, index, node }
    }
    case 'delete': {
      const nodeId = p[2]
      if (!nodeId) { printError('usage: specular wireframe <target> delete <nodeId>'); return null }
      return { kind: 'delete', nodeId }
    }
    case 'duplicate': {
      const nodeId = p[2]
      if (!nodeId) { printError('usage: specular wireframe <target> duplicate <nodeId>'); return null }
      return { kind: 'duplicate', nodeId }
    }
    case 'reorder': {
      const nodeId = p[2]
      const targetParentId = p[3]
      const targetIndex = Number(p[4])
      if (!nodeId || !targetParentId || Number.isNaN(targetIndex)) {
        printError('usage: specular wireframe <target> reorder <nodeId> <targetParentId> <targetIndex>')
        return null
      }
      return { kind: 'reorder', nodeId, targetParentId, targetIndex }
    }
    case 'set': {
      const nodeId = p[2]
      if (!nodeId) {
        printError('usage: specular wireframe <target> set <nodeId> <key> <value> [<key> <value> …] | --props <json>')
        return null
      }
      let patch: Record<string, unknown> | null
      if (args.flags.props) {
        try {
          patch = JSON.parse(args.flags.props) as Record<string, unknown>
        } catch (err) {
          printError(`set: invalid --props JSON: ${(err as Error).message}`)
          return null
        }
      } else {
        patch = parseKeyValues(p.slice(3))
        if (!patch) { printError('set: expected an even number of <key> <value> tokens'); return null }
      }
      if (!patch || Object.keys(patch).length === 0) {
        printError('set: provide at least one key/value pair or --props <json>')
        return null
      }
      return { kind: 'setProps', nodeId, patch }
    }
    default:
      printError(`unknown wireframe verb: ${verb} (expected insert|delete|duplicate|reorder|set)`)
      return null
  }
}

const wireframe: VerbHandler = async (args) => {
  const target = args.positional[0]
  const verb = args.positional[1]
  if (!target || !verb) {
    printError('usage: specular wireframe <fileId|path> <insert|delete|duplicate|reorder|set> ...')
    return 1
  }
  const op = buildWireframeOp(verb, args)
  if (!op) return 1
  printJson(await callApp('/wireframe/op', {
    method: 'POST',
    body: JSON.stringify({ target, op }),
  }))
  return 0
}

// --- Annotation verbs ---

const annotations: VerbHandler = async (args) => {
  const status = args.boolFlags.has('all')
    ? 'all'
    : (args.flags.status ?? 'unresolved')
  const result = await getAnnotationsSlim({
    status,
    url: args.flags.url,
    page_id: args.flags['page-id'],
  })
  printJson(result)
  return 0
}

const annotation: VerbHandler = async (args) => {
  const id = args.positional[0]
  if (!id) { printError('usage: specular annotation <id>'); return 1 }
  const result = await getAnnotationDetail({
    annotation_id: id,
    include_screenshot: !args.boolFlags.has('no-screenshot'),
  })
  printContentBlocks(result.content)
  return 0
}

const annotate: VerbHandler = async (args) => {
  const text = args.positional.join(' ')
  if (!text) { printError('usage: specular annotate <text>'); return 1 }
  // Construct anchor: page-specific if --page-id given, else viewport.
  // ADR 0006 retired the `--kind` flag — `specular annotate` always creates
  // a comment; the anchor type discriminates element / canvas / region.
  const anchor = args.flags['page-id']
    ? { type: 'page', pageId: args.flags['page-id'] }
    : { type: 'viewport' }
  printJson(await callApp('/annotations', {
    method: 'POST',
    body: JSON.stringify({
      text,
      anchor,
      author: 'agent',
    }),
  }))
  return 0
}

const ack: VerbHandler = async (args) => {
  const id = args.positional[0]
  if (!id) { printError('usage: specular ack <annotation-id>'); return 1 }
  printJson(await callApp(`/annotations/${id}/acknowledge`, { method: 'POST', body: '{}' }))
  return 0
}

const resolve: VerbHandler = async (args) => {
  const id = args.positional[0]
  if (!id) { printError('usage: specular resolve <annotation-id>'); return 1 }
  printJson(await callApp(`/annotations/${id}/resolve`, { method: 'POST', body: '{}' }))
  return 0
}

const dismiss: VerbHandler = async (args) => {
  const id = args.positional[0]
  if (!id) { printError('usage: specular dismiss <annotation-id>'); return 1 }
  printJson(await callApp(`/annotations/${id}/dismiss`, {
    method: 'POST',
    body: JSON.stringify({ reason: args.flags.reason }),
  }))
  return 0
}

const reply: VerbHandler = async (args) => {
  const id = args.positional[0]
  const text = args.positional.slice(1).join(' ')
  if (!id || !text) { printError('usage: specular reply <annotation-id> <text>'); return 1 }
  printJson(await callApp(`/annotations/${id}/reply`, {
    method: 'POST',
    body: JSON.stringify({ author: 'agent', text }),
  }))
  return 0
}

// --- Recording verbs ---

const record: VerbHandler = async (args) => {
  const sub = args.positional[0]
  if (sub === 'start') {
    const fid = args.positional[1] ?? pageId(args)
    if (!fid) { printError('usage: specular record start <pageId>'); return 1 }
    printJson(await callApp('/recording/start', {
      method: 'POST',
      body: JSON.stringify({
        pageId: fid,
        outputPath: args.flags.output,
        fps: args.flags.fps ? Number(args.flags.fps) : undefined,
        quality: args.flags.quality,
      }),
    }))
    return 0
  }
  if (sub === 'stop') {
    printJson(await callApp('/recording/stop', { method: 'POST' }))
    return 0
  }
  if (sub === 'status') {
    printJson(await callApp('/recording/status'))
    return 0
  }
  if (sub === 'trim') {
    const input = args.positional[1]
    if (!input) { printError('usage: specular record trim <input-path>'); return 1 }
    printJson(await callApp('/recording/trim', {
      method: 'POST',
      body: JSON.stringify({
        inputPath: input,
        outputPath: args.flags.output,
        minIdleMs: args.flags['min-idle'] ? Number(args.flags['min-idle']) : undefined,
        idleSpeedFactor: args.flags['speed-factor'] ? Number(args.flags['speed-factor']) : undefined,
      }),
    }))
    return 0
  }
  printError('usage: specular record <start|stop|status|trim>')
  return 1
}

// --- Design system verbs ---

const designSystem: VerbHandler = async () => {
  printJson(await callApp('/design-system'))
  return 0
}

const registerDesignSystem: VerbHandler = async () => {
  const input = await readStdin()
  printJson(await callApp('/design-system/register', {
    method: 'POST',
    body: JSON.stringify({ manifest: JSON.parse(input) }),
  }))
  return 0
}

const componentStates: VerbHandler = async (args) => {
  const component = args.positional[0]
  const url = args.positional[1]
  if (!component || !url) { printError('usage: specular component-states <component> <url>'); return 1 }
  printJson(await callApp('/tasks/component-states', {
    method: 'POST',
    body: JSON.stringify({
      component,
      url,
      anchor: args.flags.anchor ?? 'selection_or_empty_region',
      focus: !args.boolFlags.has('no-focus'),
      label: args.flags.label,
    }),
  }))
  return 0
}

// --- Browser shortcut verbs ---

function browseCommand(args: ParsedArgs, command: string): Promise<number> {
  return browseRaw(args, command)
}

async function browseRaw(args: ParsedArgs, command: string): Promise<number> {
  const result = await handleBrowse({ page_id: pageId(args), command })
  printContentBlocks(result.content)
  return 0
}

const snapshot: VerbHandler = async (args) => {
  // Reconstruct agent-browser snapshot command from flags
  let cmd = 'snapshot'
  if (args.boolFlags.has('i')) cmd += ' -i'
  if (args.flags.s) cmd += ` -s "${args.flags.s}"`
  if (args.flags.selector) cmd += ` -s "${args.flags.selector}"`
  if (args.flags.depth) cmd += ` -d ${args.flags.depth}`
  if (args.flags.format) cmd += ` --format ${args.flags.format}`
  return browseCommand(args, cmd)
}

const click: VerbHandler = async (args) => {
  const ref = args.positional[0]
  if (!ref) { printError('usage: specular click <ref>'); return 1 }
  return browseCommand(args, `click ${ref}`)
}

const fill: VerbHandler = async (args) => {
  const ref = args.positional[0]
  const text = args.positional.slice(1).join(' ')
  if (!ref || !text) { printError('usage: specular fill <ref> <text>'); return 1 }
  return browseCommand(args, `fill ${ref} "${text}"`)
}

const type_: VerbHandler = async (args) => {
  const ref = args.positional[0]
  const text = args.positional.slice(1).join(' ')
  if (!ref || !text) { printError('usage: specular type <ref> <text>'); return 1 }
  return browseCommand(args, `type ${ref} "${text}"`)
}

const select: VerbHandler = async (args) => {
  const ref = args.positional[0]
  const value = args.positional.slice(1).join(' ')
  if (!ref || !value) { printError('usage: specular select <ref> <value>'); return 1 }
  return browseCommand(args, `select ${ref} "${value}"`)
}

const screenshot: VerbHandler = async (args) => {
  let cmd = 'screenshot'
  if (args.boolFlags.has('annotate')) cmd += ' --annotate'
  return browseCommand(args, cmd)
}

const scroll: VerbHandler = async (args) => {
  const direction = args.positional[0] ?? 'down'
  const amount = args.positional[1]
  let cmd = `scroll ${direction}`
  if (amount) cmd += ` ${amount}`
  return browseCommand(args, cmd)
}

const wait: VerbHandler = async (args) => {
  let cmd = 'wait'
  if (args.flags.load) cmd += ` --load ${args.flags.load}`
  if (args.positional[0]) cmd += ` ${args.positional[0]}`
  if (args.flags.timeout) cmd += ` --timeout ${args.flags.timeout}`
  return browseCommand(args, cmd)
}

// --- Passthrough: unknown verbs go to agent-browser ---

/** Flags consumed by specular that must not leak into agent-browser commands. */
const SPECULAR_ONLY_FLAGS = new Set(['--page', '-f'])

function stripSpecularFlags(argv: string[]): string[] {
  const out: string[] = []
  for (let i = 0; i < argv.length; i++) {
    if (SPECULAR_ONLY_FLAGS.has(argv[i])) { i++; continue } // skip flag + value
    out.push(argv[i])
  }
  return out
}

const browsePassthrough: VerbHandler = async (args) => {
  const command = [args.verb, ...stripSpecularFlags(args.rest).map(shellQuote)].join(' ')
  return browseRaw(args, command)
}

// ---------------------------------------------------------------------------
// Verb dispatch map
// ---------------------------------------------------------------------------

const VERBS: Record<string, VerbHandler> = {
  workspace,
  selection,
  'find-placement': findPlacement,
  breakpoints,
  upsert,
  create,
  update,
  delete: deleteEntities,
  focus,
  link,
  unlink,
  group,
  ungroup,
  'auto-layout': autoLayout,
  distribute,
  wireframe,
  annotations,
  annotation,
  annotate,
  ack,
  resolve,
  dismiss,
  reply,
  record,
  'design-system': designSystem,
  'register-design-system': registerDesignSystem,
  'component-states': componentStates,
  // Browser shortcut verbs
  snapshot,
  click,
  fill,
  type: type_,
  select,
  screenshot,
  scroll,
  wait,
  // Read-only browser verbs
  get: browsePassthrough,
  console: browsePassthrough,
  errors: browsePassthrough,
  'query-elements': browsePassthrough,
}

export async function dispatch(argv: string[]): Promise<number> {
  const args = parseArgs(argv)
  if (!args.verb || args.verb === '--help' || args.verb === '-h') {
    printText('usage: specular <verb> [args...] [--flag value]')
    printText('')
    printText('Canvas: workspace, create, update, delete, focus, group, ungroup')
    printText('Browse: snapshot, click, fill, type, select, screenshot, scroll, wait')
    printText('Annotations: annotations, annotation, annotate, ack, resolve, dismiss, reply')
    printText('Recording: record <start|stop|status|trim>')
    printText('Other: breakpoints, upsert, link, unlink, find-placement')
    printText('')
    printText('Unknown verbs are passed to agent-browser as raw commands.')
    return 0
  }
  emitPresenceForVerb(args.verb)
  const handler = VERBS[args.verb] ?? browsePassthrough
  return handler(args)
}

// ---------------------------------------------------------------------------
// Stdin helper
// ---------------------------------------------------------------------------

function readStdin(): Promise<string> {
  return new Promise((resolve, reject) => {
    const chunks: Buffer[] = []
    process.stdin.on('data', (chunk: Buffer) => chunks.push(chunk))
    process.stdin.on('end', () => resolve(Buffer.concat(chunks).toString('utf-8')))
    process.stdin.on('error', reject)
  })
}

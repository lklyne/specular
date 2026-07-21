import { DEFAULT_BREAKPOINT_PRESET_LABELS } from '../shared/constants'
import { validateLayoutDirective } from '../shared/layout-directive'
import { callApp } from './shared/app-client'
import { handleBrowse, shellQuote, spawnAsync, resolveAgentBrowserPath, BLOCKED_BROWSE_VERBS } from './shared/browse-handler'
import { upsertEntities, applyPatch, type UpsertOptions, type CanvasPatch, getAnnotationsSlim, getAnnotationDetail } from './shared/entity-ops'
import { printJson, printText, printError, printContentBlocks } from './cli-output'
import { parseArgs, type ParsedArgs } from './cli-parser'
import { emitPresenceForVerb } from './cli-presence'
import { connectCommand } from './connect-command'

// ---------------------------------------------------------------------------
// Verb handlers
// ---------------------------------------------------------------------------

type VerbHandler = (args: ParsedArgs) => Promise<number>

function pageId(args: ParsedArgs): string | undefined {
  return args.flags.page ?? args.flags.f ?? undefined
}

// --- Canvas verbs ---

// Reads the canvas as a JSON Canvas document (GET /canvas — the doc read shape).
const workspace: VerbHandler = async () => {
  printJson(await callApp('/canvas'))
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

// The one declarative door (ADR 0019). A patch is { entities, edges, delete,
// layout } — no id creates, id present updates, id in delete removes — applied
// in one transaction. Documented as the batch fallback; verbs are primary.
const apply: VerbHandler = async () => {
  const patch = JSON.parse(await readStdin()) as CanvasPatch
  if (patch.layout) {
    const err = validateLayoutDirective(patch.layout)
    if (err) { printError(`apply: ${err}`); return 1 }
  }
  printJson(await applyPatch(patch))
  return 0
}

// Pure option→patch shaping for the `add` verb: each builder returns the
// create item for its kind, or a usage string when the required positional is
// missing.
function atPosition(args: ParsedArgs): Record<string, unknown> {
  if (!args.flags.at) return {}
  const [x, y] = args.flags.at.split(',').map(Number)
  return {
    ...(isNaN(x) ? {} : { canvasX: x }),
    ...(isNaN(y) ? {} : { canvasY: y }),
  }
}

function deviceFrameFlag(args: ParsedArgs): Record<string, unknown> {
  if (args.boolFlags.has('device-frame')) return { showDeviceFrame: true }
  if (args.boolFlags.has('no-device-frame')) return { showDeviceFrame: false }
  return {}
}

const ADD_ITEM_BUILDERS: Record<string, (args: ParsedArgs) => Record<string, unknown> | string> = {
  page: (args) => {
    const url = args.positional[1]
    if (!url) return 'usage: specular add page <url> [--at x,y] [--preset N]'
    return {
      kind: 'page',
      url,
      presetIndex: args.flags.preset ? Number(args.flags.preset) : 6, // default to Laptop
      ...atPosition(args),
      ...(args.boolFlags.has('landscape') ? { orientation: 'landscape' } : {}),
      ...deviceFrameFlag(args),
    }
  },
  note: (args) => {
    const text = args.positional.slice(1).join(' ')
    if (!text) return 'usage: specular add note <text> [--at x,y] [--color N]'
    // Long / structured text auto-routes to a `.md` note file on the apply
    // spine (file kind's claimsAsNote); short text stays a text entity.
    return {
      kind: 'text',
      text,
      ...atPosition(args),
      ...(args.flags.color ? { color: args.flags.color } : {}),
    }
  },
  file: (args) => {
    const path = args.positional[1]
    if (!path) return 'usage: specular add file <path> [--at x,y]'
    // The file handler infers the renderer from the extension (md / html /
    // image / video) and sizes images/video from the file.
    return {
      kind: 'file',
      file: path,
      ...atPosition(args),
      ...deviceFrameFlag(args),
    }
  },
}

// Add — kind is the subcommand (ADR 0019 §1, like `git remote add`). Each
// builder shapes one create item; `applyPatch` runs the placement pre-pass and
// routes it through the single apply spine. `add` is the only create surface;
// drawing/shape (no ergonomic verb) are created via `apply` with a patch.
const add: VerbHandler = async (args) => {
  const build = ADD_ITEM_BUILDERS[args.positional[0]]
  const item = build ? build(args) : 'usage: specular add <page|note|file> ...'
  if (typeof item === 'string') {
    printError(item)
    return 1
  }
  printJson({ created: (await applyPatch({ entities: [item] })).created })
  return 0
}

// Rearrange existing entities into a row / column / grid (ADR 0019 §1) through
// the same brain as the popup toolbar (`/selection/arrange` → `arrangeEntities`).
// Default tidies in place (keeps the footprint, evens the gaps); `--gap` packs
// tight to a fixed gap in reading order instead. No ids uses the current selection.
const arrange: VerbHandler = async (args) => {
  const mode = args.positional[0]
  if (mode !== 'row' && mode !== 'column' && mode !== 'grid') {
    printError('usage: specular arrange <row|column|grid> [id...] [--gap m] [--cols N]  (no ids uses current selection)')
    return 1
  }
  const entityIds = args.positional.slice(1)
  const gapFlag = args.flags.gap
  const gap =
    gapFlag === undefined ? undefined : isNaN(Number(gapFlag)) ? gapFlag : Number(gapFlag)
  const body = JSON.stringify({
    mode,
    ...(entityIds.length > 0 ? { entityIds } : {}),
    ...(gap !== undefined ? { gap } : {}),
    ...(args.flags.cols ? { cols: Number(args.flags.cols) } : {}),
  })
  printJson(await callApp('/selection/arrange', { method: 'POST', body }))
  return 0
}

// fallow-ignore-next-line complexity
const update: VerbHandler = async (args) => {
  const id = args.positional[0]
  if (!id) { printError('usage: specular update <id> [--at x,y] [--size w,h] [--preset N] [--text T] [--color C] [--url U] [--gap N]'); return 1 }
  // No kind: the apply route resolves it from the doc by id (ADR 0019 §4).
  const item: Record<string, unknown> = { id }
  if (args.flags.at) {
    const [x, y] = args.flags.at.split(',').map(Number)
    if (!isNaN(x)) item.canvasX = x
    if (!isNaN(y)) item.canvasY = y
  }
  // move/resize fold into update flags (ADR 0019 §1). The registry's per-kind
  // `fields` decides which take effect — pages size via preset, not w/h.
  if (args.flags.size) {
    const [w, h] = args.flags.size.split(',').map(Number)
    if (!isNaN(w)) item.width = w
    if (!isNaN(h)) item.height = h
  }
  // Page-specific flags
  if (args.flags.preset) item.presetIndex = Number(args.flags.preset)
  if (args.flags.url) item.url = args.flags.url
  if (args.boolFlags.has('landscape')) item.orientation = 'landscape'
  if (args.boolFlags.has('portrait')) item.orientation = 'portrait'
  if (args.boolFlags.has('device-frame')) item.showDeviceFrame = true
  if (args.boolFlags.has('no-device-frame')) item.showDeviceFrame = false
  // Text note flags
  if (args.flags.text) item.text = args.flags.text
  if (args.flags.color) item.color = args.flags.color
  // Group flags — the gap patches a managed group's packing gap (layoutGap).
  if (args.flags.gap !== undefined) item.layoutGap = Number(args.flags.gap)
  printJson(await upsertEntities([item]))
  return 0
}

// Shim over `apply`: kind is resolved from the doc by id, not an id prefix.
const deleteEntities: VerbHandler = async (args) => {
  let ids: string[]
  if (args.boolFlags.has('json')) {
    const input = await readStdin()
    const items = JSON.parse(input) as Array<{ id: string }>
    ids = items.map((item) => item.id)
  } else if (args.positional.length > 0) {
    ids = args.positional
  } else {
    printError('usage: specular delete <id> [id...] or specular delete --json')
    return 1
  }
  const result = await applyPatch({ delete: ids })
  printJson({ deleted: result.deleted })
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

// Shim over `apply`: builds an edges patch and routes through /canvas/apply.
const link: VerbHandler = async (args) => {
  let edges: CanvasPatch['edges']
  if (args.positional.length >= 2) {
    const [fromEntityId, toEntityId] = args.positional
    const edge: Record<string, unknown> = { fromEntityId, toEntityId, kind: 'connection' }
    if (args.flags.label) edge.label = args.flags.label
    edges = [edge] as CanvasPatch['edges']
  } else if (args.positional.length === 1 || (args.positional.length === 0 && process.stdin.isTTY)) {
    printError('usage: specular link <fromId> <toId> [--label <text>]  (or pipe a JSON edges array on stdin)')
    return 1
  } else {
    edges = JSON.parse(await readStdin()) as CanvasPatch['edges']
  }
  const result = await applyPatch({ edges })
  printJson({ edgeIds: result.edges })
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

// Shim over `apply`: a group is created as a `group` entity around existing ids.
const group: VerbHandler = async (args) => {
  if (args.positional.length === 0) { printError('usage: specular group <entityId> [entityId...]'); return 1 }
  const result = await applyPatch({
    entities: [{ kind: 'group', entityIds: args.positional, label: args.flags.label }],
  })
  printJson({ id: result.created[0], entityIds: args.positional })
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
    printError('usage: specular auto-layout <entityId> [entityId...] [--gap N]  (or a single groupId)')
    return 1
  }
  const onlyGroup =
    args.positional.length === 1 && args.positional[0].startsWith('group_')
      ? args.positional[0]
      : undefined
  const gap = args.flags.gap !== undefined ? Number(args.flags.gap) : undefined
  printJson(await callApp('/groups/auto-layout', {
    method: 'POST',
    body: JSON.stringify(
      onlyGroup
        ? { groupId: onlyGroup, label: args.flags.label, gap }
        : { entityIds: args.positional, label: args.flags.label, gap },
    ),
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
  return browseRaw(args, command, { echo: args.boolFlags.has('echo') })
}

async function browseRaw(args: ParsedArgs, command: string, opts?: { echo?: boolean }): Promise<number> {
  const result = await handleBrowse({ page_id: pageId(args), command, echo: opts?.echo })
  printContentBlocks(result.content)
  return 0
}

/** Build a `<verb> <ref> [text]` browse command, quoting each part so
 * selectors/text containing spaces or embedded quotes survive splitShellArgs
 * as single tokens. */
export function buildTargetCommand(verb: string, ref: string, text?: string): string {
  const parts = [verb, shellQuote(ref)]
  if (text !== undefined) parts.push(shellQuote(text))
  return parts.join(' ')
}

/** Build a `wait` browse command from parsed flags, quoting --text/--url values. */
export function buildWaitCommand(opts: {
  load?: string
  positional?: string
  timeout?: string
  text?: string
  url?: string
}): string {
  let cmd = 'wait'
  if (opts.load) cmd += ` --load ${opts.load}`
  if (opts.positional) cmd += ` ${opts.positional}`
  if (opts.timeout) cmd += ` --timeout ${opts.timeout}`
  if (opts.text) cmd += ` --text ${shellQuote(opts.text)}`
  if (opts.url) cmd += ` --url ${shellQuote(opts.url)}`
  return cmd
}

const snapshot: VerbHandler = async (args) => {
  return browseCommand(args, buildSnapshotCommand(args))
}

export function buildSnapshotCommand(args: ParsedArgs): string {
  let cmd = 'snapshot'
  if (args.boolFlags.has('i') || args.boolFlags.has('interactive')) cmd += ' -i'
  if (args.boolFlags.has('c') || args.boolFlags.has('compact')) cmd += ' -c'
  if (args.boolFlags.has('u') || args.boolFlags.has('urls')) cmd += ' -u'
  if (args.flags.s) cmd += ` -s ${shellQuote(args.flags.s)}`
  if (args.flags.selector) cmd += ` -s ${shellQuote(args.flags.selector)}`
  if (args.flags.d) cmd += ` -d ${args.flags.d}`
  if (args.flags.depth) cmd += ` -d ${args.flags.depth}`
  if (args.flags.format) cmd += ` --format ${args.flags.format}`
  return cmd
}

const click: VerbHandler = async (args) => {
  const ref = args.positional[0]
  if (!ref) { printError('usage: specular click <ref> [--echo]'); return 1 }
  return browseCommand(args, buildTargetCommand('click', ref))
}

const fill: VerbHandler = async (args) => {
  const ref = args.positional[0]
  const text = args.positional.slice(1).join(' ')
  if (!ref || !text) { printError('usage: specular fill <ref> <text> [--echo]'); return 1 }
  return browseCommand(args, buildTargetCommand('fill', ref, text))
}

const type_: VerbHandler = async (args) => {
  const ref = args.positional[0]
  const text = args.positional.slice(1).join(' ')
  if (!ref || !text) { printError('usage: specular type <ref> <text> [--echo]'); return 1 }
  return browseCommand(args, buildTargetCommand('type', ref, text))
}

const select: VerbHandler = async (args) => {
  const ref = args.positional[0]
  const value = args.positional.slice(1).join(' ')
  if (!ref || !value) { printError('usage: specular select <ref> <value> [--echo]'); return 1 }
  return browseCommand(args, buildTargetCommand('select', ref, value))
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
  return browseCommand(args, buildWaitCommand({
    load: args.flags.load,
    positional: args.positional[0],
    timeout: args.flags.timeout,
    text: args.flags.text,
    url: args.flags.url,
  }))
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
  // handleBrowse enforces the blocklist too (it's the shared choke point for
  // CLI and MCP); checking here as well fails fast without page resolution.
  const blockReason = BLOCKED_BROWSE_VERBS[args.verb]
  if (blockReason) {
    printError(`specular ${args.verb}: blocked — ${blockReason}`)
    return 1
  }
  const command = [args.verb, ...stripSpecularFlags(args.rest).map(shellQuote)].join(' ')
  return browseRaw(args, command)
}

// `skills` is a meta-verb, not a page-scoped browse command: it spawns
// agent-browser directly (no page resolution, no --cdp/--session) so agents
// can discover agent-browser's own documented workflows.
const skills: VerbHandler = async (args) => {
  const { stdout } = await spawnAsync(
    resolveAgentBrowserPath(),
    ['skills', ...stripSpecularFlags(args.rest)],
    { timeout: 30_000 },
  )
  printText(stdout.trimEnd())
  return 0
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
  apply,
  add,
  update,
  delete: deleteEntities,
  arrange,
  focus,
  link,
  unlink,
  group,
  ungroup,
  'auto-layout': autoLayout,
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
  // Headless cloud peer (talks to the sync server, not a running app)
  connect: connectCommand,
  // Read-only browser verbs
  get: browsePassthrough,
  console: browsePassthrough,
  errors: browsePassthrough,
  'query-elements': browsePassthrough,
  // agent-browser meta-verb
  skills,
}

export async function dispatch(argv: string[]): Promise<number> {
  const args = parseArgs(argv)
  if (!args.verb || args.verb === '--help' || args.verb === '-h') {
    printText('usage: specular <verb> [args...] [--flag value]')
    printText('')
    printText('Canvas: workspace, add, update, delete, arrange, focus, group, ungroup')
    printText('Browse: snapshot, click, fill, type, select, screenshot, scroll, wait')
    printText('Annotations: annotations, annotation, annotate, ack, resolve, dismiss, reply')
    printText('Recording: record <start|stop|status|trim>')
    printText('Cloud: connect <link> --html <path> | --status  (headless sync peer)')
    printText('Other: breakpoints, apply, upsert, link, unlink, auto-layout, find-placement')
    printText('')
    printText('Unknown verbs pass through to the bundled agent-browser as raw commands')
    printText('(some — launch/close/quit/install/upgrade/open — are blocked in')
    printText('favor of specular equivalents). See the specular skill\'s passthrough')
    printText('section for the full command surface, or run `specular skills get core`.')
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

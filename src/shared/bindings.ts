import type { Tool } from './tool'

export type KeyboardSourceView =
  | 'aboveView'
  | 'canvasBg'
  | 'toolbar'
  | 'leftSidebar'
  | 'rightDetailsPanel'
  | 'devtoolsHeader'
  | 'devtoolsResizeHandle'
  | 'page'

// Convenience shorthand used in scope arrays throughout this file
export const CANVAS_REGION: KeyboardSourceView[] = [
  'aboveView',
  'canvasBg',
  'toolbar',
  'rightDetailsPanel',
]

const CANVAS_OR_PAGE_REGION: KeyboardSourceView[] = [
  ...CANVAS_REGION,
  'page',
]

export const ALL_VIEWS: KeyboardSourceView[] = [
  'aboveView',
  'canvasBg',
  'toolbar',
  'leftSidebar',
  'rightDetailsPanel',
  'devtoolsHeader',
  'devtoolsResizeHandle',
  'page',
]

export type NormalizedKey = {
  key: string // lowercased, e.g. 'v', 'escape', 'arrowleft'
  cmd: boolean // CmdOrCtrl
  alt: boolean
  shift: boolean
}

export type BindingId =
  | 'tool-select'
  | 'tool-add-page'
  | 'tool-add-text'
  | 'tool-add-sticky'
  | 'tool-add-shape-rectangle'
  | 'tool-add-shape-ellipse'
  | 'tool-add-shape-diamond'
  | 'tool-comment'
  | 'tool-draw-pen'
  | 'tool-draw-highlight'
  | 'tool-inspect'
  | 'undo'
  | 'redo'
  | 'reset-viewport'
  | 'group'
  | 'ungroup'
  | 'select-all'
  | 'duplicate'
  | 'delete-selection'
  | 'stack-bring-forward'
  | 'stack-send-backward'
  | 'stack-bring-to-front'
  | 'stack-send-to-back'
  | 'nav-left'
  | 'nav-right'
  | 'nav-up'
  | 'nav-down'
  | 'escape-tool'
  | 'escape-page-focus'
  | 'close-tab'
  | 'annotation-close-thread'
  | 'annotation-clear-draft'

export type BindingTarget = 'main' | KeyboardSourceView

export type BindingContext = {
  activeTool: Tool
  isTextEditing: boolean
  pageFocusActive: boolean
  sourceView: KeyboardSourceView
  viewMode: 'canvas' | 'browser'
  hasOpenAnnotationThread: boolean
  hasPendingAnnotation: boolean
}

export type Binding = {
  id: BindingId
  defaultKey: NormalizedKey
  scope: KeyboardSourceView[]
  target: BindingTarget
  firesWhileTyping?: boolean
  firesFromPageFocus?: boolean
  when?: (ctx: BindingContext) => boolean
  label: string
}

function k(key: string, cmd = false, shift = false, alt = false): NormalizedKey {
  return { key, cmd, alt, shift }
}

function canvasModeOnly(ctx: BindingContext): boolean {
  return ctx.viewMode === 'canvas'
}

// Table order determines dispatch priority. Escape resolution relies on:
//   annotation-close-thread / annotation-clear-draft → escape-page-focus → escape-tool
export const BINDINGS: readonly Binding[] = [
  // Tool selection (canvas-region, plain keys, suppressed while typing)
  { id: 'tool-select', defaultKey: k('v'), scope: CANVAS_REGION, target: 'main', label: 'Select' },
  { id: 'tool-add-page', defaultKey: k('p'), scope: CANVAS_REGION, target: 'main', label: 'Add page' },
  { id: 'tool-add-text', defaultKey: k('t'), scope: CANVAS_REGION, target: 'main', label: 'Add text' },
  { id: 'tool-add-sticky', defaultKey: k('s'), scope: CANVAS_REGION, target: 'main', label: 'Add sticky' },
  { id: 'tool-add-shape-rectangle', defaultKey: k('r'), scope: CANVAS_REGION, target: 'main', label: 'Rectangle' },
  { id: 'tool-add-shape-ellipse', defaultKey: k('o'), scope: CANVAS_REGION, target: 'main', label: 'Ellipse' },
  { id: 'tool-add-shape-diamond', defaultKey: k('r', false, true), scope: CANVAS_REGION, target: 'main', label: 'Diamond' },
  { id: 'tool-comment', defaultKey: k('c'), scope: CANVAS_REGION, target: 'main', label: 'Comment' },
  { id: 'tool-draw-pen', defaultKey: k('m'), scope: CANVAS_REGION, target: 'main', label: 'Pen' },
  { id: 'tool-draw-highlight', defaultKey: k('m', false, true), scope: CANVAS_REGION, target: 'main', label: 'Highlight' },
  { id: 'tool-inspect', defaultKey: k('i'), scope: CANVAS_REGION, target: 'main', label: 'Inspect' },

  // Global shortcuts — fire from all views, fire while typing
  { id: 'undo', defaultKey: k('z', true), scope: ALL_VIEWS, target: 'main', firesWhileTyping: true, label: 'Undo' },
  { id: 'redo', defaultKey: k('z', true, true), scope: ALL_VIEWS, target: 'main', firesWhileTyping: true, label: 'Redo' },
  {
    id: 'reset-viewport',
    defaultKey: k('1', true),
    scope: ALL_VIEWS,
    target: 'main',
    firesWhileTyping: true,
    firesFromPageFocus: true,
    label: 'Reset viewport',
  },
  { id: 'close-tab', defaultKey: k('w', true), scope: ALL_VIEWS, target: 'main', firesWhileTyping: true, label: 'Close tab' },

  // Canvas-region modifier shortcuts
  { id: 'group', defaultKey: k('g', true), scope: CANVAS_REGION, target: 'main', label: 'Group' },
  { id: 'ungroup', defaultKey: k('g', true, true), scope: CANVAS_REGION, target: 'main', label: 'Ungroup' },
  { id: 'select-all', defaultKey: k('a', true), scope: CANVAS_REGION, target: 'main', label: 'Select all' },
  { id: 'duplicate', defaultKey: k('d', true), scope: CANVAS_REGION, target: 'main', label: 'Duplicate' },
  {
    id: 'stack-bring-forward',
    defaultKey: k(']', true),
    scope: [...CANVAS_REGION, 'leftSidebar'],
    target: 'main',
    when: canvasModeOnly,
    label: 'Bring forward',
  },
  {
    id: 'stack-send-backward',
    defaultKey: k('[', true),
    scope: [...CANVAS_REGION, 'leftSidebar'],
    target: 'main',
    when: canvasModeOnly,
    label: 'Send backward',
  },
  {
    id: 'stack-bring-to-front',
    defaultKey: k(']', true, true),
    scope: [...CANVAS_REGION, 'leftSidebar'],
    target: 'main',
    when: canvasModeOnly,
    label: 'Bring to front',
  },
  {
    id: 'stack-send-to-back',
    defaultKey: k('[', true, true),
    scope: [...CANVAS_REGION, 'leftSidebar'],
    target: 'main',
    when: canvasModeOnly,
    label: 'Send to back',
  },
  {
    id: 'stack-bring-forward',
    defaultKey: k('arrowup'),
    scope: ['leftSidebar'],
    target: 'main',
    when: canvasModeOnly,
    label: 'Bring forward',
  },
  {
    id: 'stack-send-backward',
    defaultKey: k('arrowdown'),
    scope: ['leftSidebar'],
    target: 'main',
    when: canvasModeOnly,
    label: 'Send backward',
  },

  // Canvas-region plain shortcuts
  {
    id: 'delete-selection',
    defaultKey: k('delete'),
    scope: CANVAS_OR_PAGE_REGION,
    target: 'main',
    firesFromPageFocus: true,
    label: 'Delete',
  },
  {
    id: 'delete-selection',
    defaultKey: k('backspace'),
    scope: CANVAS_OR_PAGE_REGION,
    target: 'main',
    firesFromPageFocus: true,
    label: 'Delete',
  },
  { id: 'nav-left', defaultKey: k('arrowleft'), scope: CANVAS_REGION, target: 'main', label: 'Navigate left' },
  { id: 'nav-right', defaultKey: k('arrowright'), scope: CANVAS_REGION, target: 'main', label: 'Navigate right' },
  { id: 'nav-up', defaultKey: k('arrowup'), scope: CANVAS_REGION, target: 'main', label: 'Navigate up' },
  { id: 'nav-down', defaultKey: k('arrowdown'), scope: CANVAS_REGION, target: 'main', label: 'Navigate down' },

  // Annotation Escape bindings — renderer-targeted, ordered before escape-tool
  {
    id: 'annotation-close-thread',
    defaultKey: k('escape'),
    scope: ['aboveView'],
    target: 'aboveView',
    when: (ctx) => ctx.hasOpenAnnotationThread,
    label: 'Close annotation thread',
  },
  {
    id: 'annotation-clear-draft',
    defaultKey: k('escape'),
    scope: ['aboveView'],
    target: 'aboveView',
    when: (ctx) => ctx.hasPendingAnnotation,
    label: 'Clear annotation draft',
  },

  // Escape resolution: page-focus exit before tool exit
  {
    id: 'escape-page-focus',
    defaultKey: k('escape'),
    scope: ['page'],
    target: 'main',
    firesFromPageFocus: true,
    when: (ctx) => ctx.pageFocusActive,
    label: 'Exit page focus',
  },
  {
    id: 'escape-tool',
    defaultKey: k('escape'),
    scope: ALL_VIEWS,
    target: 'main',
    firesWhileTyping: true,
    firesFromPageFocus: true,
    when: (ctx) => ctx.activeTool.kind !== 'select',
    label: 'Exit tool',
  },
]

export function dispatchKey(
  table: readonly Binding[],
  key: NormalizedKey,
  ctx: BindingContext,
): BindingId | null {
  for (const binding of table) {
    if (!keysMatch(binding.defaultKey, key)) continue
    if (!binding.scope.includes(ctx.sourceView)) continue
    if (ctx.pageFocusActive && !binding.firesFromPageFocus) continue
    if (ctx.isTextEditing && !binding.firesWhileTyping) continue
    if (binding.when !== undefined && !binding.when(ctx)) continue
    return binding.id
  }
  return null
}

function keysMatch(a: NormalizedKey, b: NormalizedKey): boolean {
  return a.key === b.key && a.cmd === b.cmd && a.alt === b.alt && a.shift === b.shift
}

// Minimal Electron Input interface — matches the shape of Electron's Input
// event without importing from Electron (shared/ has no process-specific deps).
export interface ElectronInputEvent {
  type: string
  key: string
  code: string
  isAutoRepeat: boolean
  shift: boolean
  control: boolean
  alt: boolean
  meta: boolean
}

export function normalizeElectronInput(input: ElectronInputEvent): NormalizedKey | null {
  if (input.type !== 'keyDown') return null
  const bracketKey =
    input.code === 'BracketLeft'
      ? '['
      : input.code === 'BracketRight'
        ? ']'
        : null
  return {
    key: bracketKey ?? input.key.toLowerCase(),
    cmd: input.meta || input.control,
    alt: input.alt,
    shift: input.shift,
  }
}

export function acceleratorString(key: NormalizedKey): string {
  const parts: string[] = []
  if (key.cmd) parts.push('CmdOrCtrl')
  if (key.alt) parts.push('Alt')
  if (key.shift) parts.push('Shift')
  parts.push(toAcceleratorKey(key.key))
  return parts.join('+')
}

function toAcceleratorKey(key: string): string {
  switch (key) {
    case 'arrowleft':
      return 'Left'
    case 'arrowright':
      return 'Right'
    case 'arrowup':
      return 'Up'
    case 'arrowdown':
      return 'Down'
    case 'escape':
      return 'Escape'
    case 'delete':
      return 'Delete'
    case 'backspace':
      return 'Backspace'
    case '[':
      return '['
    case ']':
      return ']'
    case 'tab':
      return 'Tab'
    case 'enter':
    case 'return':
      return 'Return'
    default:
      return key.toUpperCase()
  }
}

export function bindingById(id: BindingId): Binding {
  const binding = BINDINGS.find((b) => b.id === id)
  if (!binding) throw new Error(`No binding for id: ${id}`)
  return binding
}

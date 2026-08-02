/**
 * Obsidian-style live preview for the markdown editor.
 *
 * Markdown punctuation (`#`, `**`, `[](…)`, `- `) is collapsed to zero width
 * so the text reads as formatted prose, and revealed again on whatever line
 * the cursor is on so the source stays directly editable. A non-editable
 * editor has no cursor, so it reveals nothing — that is what makes a
 * read-only sticky render as clean text through the same component.
 *
 * Markup that occupies a whole line on its own (a setext underline, a code
 * fence) is hidden with a line decoration rather than a replacement: zeroing
 * the characters would leave an empty line box where the markup was.
 */

import { syntaxTree } from '@codemirror/language'
import type { EditorState, Extension, Range } from '@codemirror/state'
import {
  Decoration,
  EditorView,
  ViewPlugin,
  WidgetType,
  type DecorationSet,
  type ViewUpdate,
} from '@codemirror/view'
import type { SyntaxNodeRef } from '@lezer/common'
import { looksLikeUrl } from '../../../shared/url'
import { selectedLineNumbers } from './markdown-commands'

/** Punctuation that carries no meaning once the syntax highlighter has styled the text. */
const INLINE_MARKS = new Set([
  'EmphasisMark',
  'StrikethroughMark',
  'CodeMark',
  'CodeInfo',
  'SubscriptMark',
  'SuperscriptMark',
  'LinkMark',
])

/** Only the marker pairs the sticky variant still formats. */
const STICKY_INLINE_MARKS = new Set(['EmphasisMark', 'StrikethroughMark'])

/**
 * Knobs `buildMarkdownDecorations` and `revealedLines` gate their decoration
 * cases on. The full editor (`FULL_LIVE_PREVIEW`) is Obsidian-style: every
 * markdown construct collapses, and collapsed markup reveals itself on the
 * cursor's line. The sticky-note variant (`STICKY_LIVE_PREVIEW`) formats a
 * reduced subset — bold, italic, strikethrough, bullets — and renders
 * everything else (headings, quotes, tasks, rules, link chrome) as literal
 * text; the marks it does support stay hidden even on the cursor's line,
 * since a sticky note has no "source view" to fall back into.
 */
export interface LivePreviewOptions {
  /** Inline mark node names collapsed to zero width (e.g. `EmphasisMark`). */
  inlineMarks: ReadonlySet<string>
  /** Collapse `HeaderMark` (and setext underlines, once handled). */
  headings: boolean
  /** Collapse `QuoteMark` and draw the blockquote border line. */
  quotes: boolean
  /** Replace `TaskMarker` with a ☐/☑ glyph. */
  taskMarkers: boolean
  /** Replace a horizontal rule with a rendered `<hr>`. */
  horizontalRules: boolean
  /** Collapse `[label](url)` down to just the label. */
  linkChrome: boolean
  /** Replace a bullet `ListMark` with a `•` glyph. */
  bullets: boolean
  /** Re-reveal markup on the line holding the cursor while editable. */
  revealOnCursor: boolean
  /** Mark URLs as click/Cmd+click-openable targets (`data-md-url`). Off
   *  means links are just text — no color, no pointer, no opening. */
  linkTargets: boolean
}

export const FULL_LIVE_PREVIEW: LivePreviewOptions = {
  inlineMarks: INLINE_MARKS,
  headings: true,
  quotes: true,
  taskMarkers: true,
  horizontalRules: true,
  linkChrome: true,
  bullets: true,
  revealOnCursor: true,
  linkTargets: true,
}

export const STICKY_LIVE_PREVIEW: LivePreviewOptions = {
  inlineMarks: STICKY_INLINE_MARKS,
  headings: false,
  quotes: false,
  taskMarkers: false,
  horizontalRules: false,
  linkChrome: false,
  bullets: true,
  revealOnCursor: false,
  linkTargets: false,
}

const HIDDEN = Decoration.replace({})
const HIDDEN_LINE = Decoration.line({ class: 'cm-md-hidden-line' })
const QUOTE_LINE = Decoration.line({ class: 'cm-md-quote' })

class GlyphWidget extends WidgetType {
  constructor(
    private readonly text: string,
    private readonly cls: string,
  ) {
    super()
  }
  eq(other: GlyphWidget): boolean {
    return other.text === this.text && other.cls === this.cls
  }
  toDOM(): HTMLElement {
    const span = document.createElement('span')
    span.className = this.cls
    span.textContent = this.text
    return span
  }
  ignoreEvent(): boolean {
    return true
  }
}

class RuleWidget extends WidgetType {
  eq(): boolean {
    return true
  }
  toDOM(): HTMLElement {
    return document.createElement('hr')
  }
  ignoreEvent(): boolean {
    return true
  }
}

/** Line numbers whose markup should stay visible because the user is editing there. */
function revealedLines(state: EditorState, options: LivePreviewOptions): Set<number> {
  if (!options.revealOnCursor) return new Set()
  if (!state.facet(EditorView.editable)) return new Set()
  return selectedLineNumbers(state)
}

/** `# ` and `> ` own their trailing space; leaving it behind indents the line. */
function eatTrailingSpace(state: EditorState, to: number): number {
  let end = to
  while (state.sliceDoc(end, end + 1) === ' ') end += 1
  return end
}

function hideMarkup(
  decorations: Range<Decoration>[],
  state: EditorState,
  from: number,
  to: number,
): void {
  if (to <= from) return
  const line = state.doc.lineAt(from)
  if (from === line.from && to >= line.to) {
    decorations.push(HIDDEN_LINE.range(line.from))
    return
  }
  decorations.push(HIDDEN.range(from, to))
}

/**
 * `[label](url)` renders as `label`: hide the leading `[` and everything from
 * the closing `]` to the end of the node. The label keeps its link styling
 * from the highlight style.
 */
function collapseLinkChrome(
  decorations: Range<Decoration>[],
  node: SyntaxNodeRef,
  state: EditorState,
): void {
  const marks: Array<{ from: number; to: number }> = []
  for (let child = node.node.firstChild; child; child = child.nextSibling) {
    if (child.name === 'LinkMark') marks.push({ from: child.from, to: child.to })
  }
  // Bare `<autolink>` / reference forms without a `[…](…)` shape: hide every
  // mark individually rather than guessing at a label span.
  if (marks.length < 2) {
    for (const mark of marks) hideMarkup(decorations, state, mark.from, mark.to)
    return
  }
  hideMarkup(decorations, state, node.from, marks[0].to)
  hideMarkup(decorations, state, marks[1].from, node.to)
}

/** One collapsed-node renderer per markdown node name. Each returns what the
 *  tree visitor should: `undefined` to keep descending (feature disabled),
 *  `false` to stop at this node. */
type NodeDecorator = (
  decorations: Range<Decoration>[],
  node: SyntaxNodeRef,
  state: EditorState,
  options: LivePreviewOptions,
) => boolean | undefined

const collapseLink: NodeDecorator = (decorations, node, state, options) => {
  if (!options.linkChrome) return undefined
  collapseLinkChrome(decorations, node, state)
  return false
}

const collapseRule: NodeDecorator = (decorations, node, _state, options) => {
  if (!options.horizontalRules) return undefined
  decorations.push(Decoration.replace({ widget: new RuleWidget() }).range(node.from, node.to))
  return false
}

const collapseListMark: NodeDecorator = (decorations, node, state, options) => {
  if (!options.bullets) return undefined
  // Ordered markers ("1.") already read as a list; only bullets lose
  // their meaning when the `-` is hidden.
  if (!/^[-*+]$/.test(state.sliceDoc(node.from, node.to))) return false
  decorations.push(
    Decoration.replace({ widget: new GlyphWidget('•', 'cm-md-bullet') }).range(node.from, node.to),
  )
  return false
}

const collapseTaskMarker: NodeDecorator = (decorations, node, state, options) => {
  if (!options.taskMarkers) return undefined
  const done = /x/i.test(state.sliceDoc(node.from, node.to))
  decorations.push(
    Decoration.replace({ widget: new GlyphWidget(done ? '☑' : '☐', 'cm-md-task') }).range(
      node.from,
      node.to,
    ),
  )
  return false
}

const collapseQuoteMark: NodeDecorator = (decorations, node, state, options) => {
  if (!options.quotes) return undefined
  hideMarkup(decorations, state, node.from, eatTrailingSpace(state, node.to))
  return false
}

const collapseHeaderMark: NodeDecorator = (decorations, node, state, options) => {
  if (!options.headings) return undefined
  hideMarkup(decorations, state, node.from, eatTrailingSpace(state, node.to))
  return false
}

const NODE_DECORATORS: Record<string, NodeDecorator> = {
  Link: collapseLink,
  Image: collapseLink,
  HorizontalRule: collapseRule,
  ListMark: collapseListMark,
  TaskMarker: collapseTaskMarker,
  QuoteMark: collapseQuoteMark,
  HeaderMark: collapseHeaderMark,
}

/**
 * Pure over `state` + the ranges to cover, so it can be exercised without a
 * live EditorView (and therefore without a DOM).
 */
export function buildMarkdownDecorations(
  state: EditorState,
  ranges: readonly { from: number; to: number }[],
  options: LivePreviewOptions = FULL_LIVE_PREVIEW,
): DecorationSet {
  const decorations: Range<Decoration>[] = []
  const doc = state.doc
  const revealed = revealedLines(state, options)

  for (const { from, to } of ranges) {
    syntaxTree(state).iterate({
      from,
      to,
      enter: (node) => {
        // The quote border replaces the `>` glyph, so it is drawn whether or
        // not the line is revealed — otherwise the quote loses its outline
        // the moment the cursor enters it.
        if (node.name === 'Blockquote') {
          if (!options.quotes) return undefined
          const first = doc.lineAt(node.from).number
          const last = doc.lineAt(node.to).number
          for (let n = first; n <= last; n += 1) {
            decorations.push(QUOTE_LINE.range(doc.line(n).from))
          }
          return undefined
        }
        if (revealed.has(doc.lineAt(node.from).number)) return undefined

        const decorate = NODE_DECORATORS[node.name]
        if (decorate) return decorate(decorations, node, state, options)
        if (!options.inlineMarks.has(node.name)) return undefined
        hideMarkup(decorations, state, node.from, node.to)
        return false
      },
    })
  }
  return Decoration.set(decorations, true)
}

/** The URL a link-ish node points at: its own text for a bare `URL`, the
 *  `URL` child's text for `Link` / `Autolink`. */
function linkTargetUrl(node: SyntaxNodeRef, state: EditorState): string | null {
  if (node.name === 'URL') return state.sliceDoc(node.from, node.to)
  for (let child = node.node.firstChild; child; child = child.nextSibling) {
    if (child.name === 'URL') return state.sliceDoc(child.from, child.to)
  }
  return null
}

/**
 * `data-md-url` marks over links make the target URL readable from any DOM
 * event (Cmd+click while editing, plain click read-only) without re-parsing
 * the document. Kept out of `livePreviewPlugin` because that plugin's
 * decorations double as atomic ranges — link marks there would make the
 * cursor skip over link text. Pure over `state` + ranges, like
 * `buildMarkdownDecorations`, so it's testable without a DOM.
 */
export function buildLinkMarks(
  state: EditorState,
  ranges: readonly { from: number; to: number }[],
): DecorationSet {
  const decorations: Range<Decoration>[] = []
  for (const { from, to } of ranges) {
    syntaxTree(state).iterate({
      from,
      to,
      enter: (node) => {
        if (node.name !== 'Link' && node.name !== 'URL' && node.name !== 'Autolink') {
          return undefined
        }
        // A `URL` inside a Link/Autolink is covered by its parent's mark.
        if (node.name === 'URL') {
          const parent = node.node.parent?.name
          if (parent === 'Link' || parent === 'Image' || parent === 'Autolink') {
            return undefined
          }
        }
        const url = linkTargetUrl(node, state)
        // Relative paths and non-http(s) schemes can't become pages.
        if (url && looksLikeUrl(url)) {
          decorations.push(
            Decoration.mark({
              class: 'cm-md-link',
              attributes: { 'data-md-url': url },
            }).range(node.from, node.to),
          )
        }
        return false
      },
    })
  }
  return Decoration.set(decorations, true)
}

const linkTargetsPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet
    constructor(view: EditorView) {
      this.decorations = buildLinkMarks(view.state, view.visibleRanges)
    }
    update(update: ViewUpdate) {
      if (!update.docChanged && !update.viewportChanged) return
      this.decorations = buildLinkMarks(update.view.state, update.view.visibleRanges)
    }
  },
  { decorations: (plugin) => plugin.decorations },
)

function sameLineSet(a: Set<number>, b: Set<number>): boolean {
  if (a.size !== b.size) return false
  for (const n of a) if (!b.has(n)) return false
  return true
}

function createLivePreviewPlugin(options: LivePreviewOptions) {
  return ViewPlugin.fromClass(
    class {
      decorations: DecorationSet
      revealed: Set<number>
      constructor(view: EditorView) {
        this.revealed = revealedLines(view.state, options)
        this.decorations = buildMarkdownDecorations(view.state, view.visibleRanges, options)
      }
      update(update: ViewUpdate) {
        // Cursor moves only matter when they can reveal/re-hide markup.
        const cursorMatters = options.revealOnCursor && update.selectionSet
        // The view/edit swap reconfigures `editable`, which gates the
        // cursor-line reveal (see revealedLines) — without this the markup
        // left revealed on entry stays revealed after exit.
        const editableChanged =
          update.startState.facet(EditorView.editable) !== update.state.facet(EditorView.editable)
        if (!update.docChanged && !cursorMatters && !update.viewportChanged && !editableChanged) {
          return
        }
        // A cursor-only move within the same line(s) can't change the output.
        const revealed = revealedLines(update.state, options)
        const cursorOnly = !update.docChanged && !update.viewportChanged && !editableChanged
        if (cursorOnly && sameLineSet(revealed, this.revealed)) return
        this.revealed = revealed
        this.decorations = buildMarkdownDecorations(
          update.view.state,
          update.view.visibleRanges,
          options,
        )
      }
    },
    {
      decorations: (plugin) => plugin.decorations,
      // Without this, arrow keys strand the cursor inside a zero-width marker.
      provide: (plugin) =>
        EditorView.atomicRanges.of((view) => view.plugin(plugin)?.decorations ?? Decoration.none),
    },
  )
}

const livePreviewTheme = EditorView.theme({
  '.cm-md-hidden-line': { display: 'none' },
  '.cm-md-quote': {
    borderLeft: '2px solid currentColor',
    paddingLeft: '0.6em',
    opacity: '0.85',
  },
  '.cm-md-bullet': { opacity: '0.6' },
  '.cm-md-task': { opacity: '0.6' },
  // Read-only editors (contenteditable off) open links on plain click, so
  // links advertise themselves; while editing, the caret stays a text cursor
  // and Cmd+click opens.
  '.cm-content[contenteditable="false"] .cm-md-link': { cursor: 'pointer' },
  '.cm-content[contenteditable="false"] .cm-md-link:hover': {
    textDecoration: 'underline',
  },
  '.cm-line hr': {
    border: 'none',
    borderTop: '1px solid currentColor',
    opacity: '0.25',
    margin: '0.5em 0',
    display: 'inline-block',
    width: '100%',
  },
})

export function markdownLivePreview(options: LivePreviewOptions = FULL_LIVE_PREVIEW): Extension {
  const parts: Extension[] = [createLivePreviewPlugin(options), livePreviewTheme]
  if (options.linkTargets) parts.push(linkTargetsPlugin)
  return parts
}

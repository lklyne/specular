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
function revealedLines(state: EditorState): Set<number> {
  const lines = new Set<number>()
  if (!state.facet(EditorView.editable)) return lines
  const doc = state.doc
  for (const range of state.selection.ranges) {
    const first = doc.lineAt(range.from).number
    const last = doc.lineAt(range.to).number
    for (let n = first; n <= last; n += 1) lines.add(n)
  }
  return lines
}

/** `# ` and `> ` own their trailing space; leaving it behind indents the line. */
function eatTrailingSpace(text: string, to: number): number {
  let end = to
  while (text[end] === ' ') end += 1
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

/**
 * Pure over `state` + the ranges to cover, so it can be exercised without a
 * live EditorView (and therefore without a DOM).
 */
export function buildMarkdownDecorations(
  state: EditorState,
  ranges: readonly { from: number; to: number }[],
): DecorationSet {
  const decorations: Range<Decoration>[] = []
  const doc = state.doc
  const revealed = revealedLines(state)
  const text = doc.toString()

  for (const { from, to } of ranges) {
    syntaxTree(state).iterate({
      from,
      to,
      enter: (node) => {
        // The quote border replaces the `>` glyph, so it is drawn whether or
        // not the line is revealed — otherwise the quote loses its outline
        // the moment the cursor enters it.
        if (node.name === 'Blockquote') {
          const first = doc.lineAt(node.from).number
          const last = doc.lineAt(node.to).number
          for (let n = first; n <= last; n += 1) {
            decorations.push(QUOTE_LINE.range(doc.line(n).from))
          }
          return undefined
        }
        if (revealed.has(doc.lineAt(node.from).number)) return undefined

        switch (node.name) {
          case 'Link':
          case 'Image':
            collapseLinkChrome(decorations, node, state)
            return false
          case 'HorizontalRule':
            decorations.push(
              Decoration.replace({ widget: new RuleWidget() }).range(node.from, node.to),
            )
            return false
          case 'ListMark': {
            // Ordered markers ("1.") already read as a list; only bullets lose
            // their meaning when the `-` is hidden.
            if (!/^[-*+]$/.test(text.slice(node.from, node.to))) return false
            decorations.push(
              Decoration.replace({
                widget: new GlyphWidget('•', 'cm-md-bullet'),
              }).range(node.from, node.to),
            )
            return false
          }
          case 'TaskMarker': {
            const done = /x/i.test(text.slice(node.from, node.to))
            decorations.push(
              Decoration.replace({
                widget: new GlyphWidget(done ? '☑' : '☐', 'cm-md-task'),
              }).range(node.from, node.to),
            )
            return false
          }
          case 'QuoteMark':
            hideMarkup(decorations, state, node.from, eatTrailingSpace(text, node.to))
            return false
          case 'HeaderMark':
            hideMarkup(decorations, state, node.from, eatTrailingSpace(text, node.to))
            return false
          default:
            if (!INLINE_MARKS.has(node.name)) return undefined
            hideMarkup(decorations, state, node.from, node.to)
            return false
        }
      },
    })
  }
  return Decoration.set(decorations, true)
}

const livePreviewPlugin = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet
    constructor(view: EditorView) {
      this.decorations = buildMarkdownDecorations(view.state, view.visibleRanges)
    }
    update(update: ViewUpdate) {
      if (!update.docChanged && !update.selectionSet && !update.viewportChanged) return
      this.decorations = buildMarkdownDecorations(update.view.state, update.view.visibleRanges)
    }
  },
  {
    decorations: (plugin) => plugin.decorations,
    // Without this, arrow keys strand the cursor inside a zero-width marker.
    provide: (plugin) =>
      EditorView.atomicRanges.of((view) => view.plugin(plugin)?.decorations ?? Decoration.none),
  },
)

const livePreviewTheme = EditorView.theme({
  '.cm-md-hidden-line': { display: 'none' },
  '.cm-md-quote': {
    borderLeft: '2px solid currentColor',
    paddingLeft: '0.6em',
    opacity: '0.85',
  },
  '.cm-md-bullet': { opacity: '0.6' },
  '.cm-md-task': { opacity: '0.6' },
  '.cm-line hr': {
    border: 'none',
    borderTop: '1px solid currentColor',
    opacity: '0.25',
    margin: '0.5em 0',
    display: 'inline-block',
    width: '100%',
  },
})

export function markdownLivePreview(): Extension {
  return [livePreviewPlugin, livePreviewTheme]
}

/**
 * Whole-note formatting toggles for stickies that aren't in edit mode — the
 * popover buttons still work there, applying to every line of the note.
 * Pure string transforms over the markdown source: each non-empty line's
 * content (after any bullet prefix) is wrapped/unwrapped as a unit, so the
 * result stays valid markdown (a wrap never spans line breaks or swallows
 * a list marker).
 */

import { BULLET_LINE } from './bullet-line'

const LINE_PARTS = /^(\s*)([-*+] )?(.*)$/

export type WholeNoteMarker = '**' | '~~'

interface PeeledContent {
  bold: boolean
  strikethrough: boolean
  inner: string
}

/** Strips outer `**`/`~~` layers in any nesting order, recording which were present. */
function peelWraps(content: string): PeeledContent {
  let inner = content
  let bold = false
  let strikethrough = false
  for (;;) {
    if (!bold && isWrapped(inner, '**')) {
      bold = true
      inner = inner.slice(2, -2)
    } else if (!strikethrough && isWrapped(inner, '~~')) {
      strikethrough = true
      inner = inner.slice(2, -2)
    } else {
      return { bold, strikethrough, inner }
    }
  }
}

function isWrapped(content: string, marker: WholeNoteMarker): boolean {
  return (
    content.length > marker.length * 2 &&
    content.startsWith(marker) &&
    content.endsWith(marker) &&
    // `**a** b **c**` looks wrapped but the opening marker closes at the
    // first inner occurrence — a true whole-line wrap has no inner markers.
    !innerBreaksWrap(content.slice(marker.length, -marker.length), marker)
  )
}

function innerBreaksWrap(inner: string, marker: WholeNoteMarker): boolean {
  return inner.includes(marker) || inner.startsWith(marker.slice(0, 1))
}

function rebuild(peeled: PeeledContent): string {
  let content = peeled.inner
  if (peeled.strikethrough) content = `~~${content}~~`
  if (peeled.bold) content = `**${content}**`
  return content
}

interface NoteLine {
  indent: string
  bullet: string
  content: string
}

function splitLines(text: string): NoteLine[] {
  return text.split('\n').map((line) => {
    const match = LINE_PARTS.exec(line) as RegExpExecArray
    return { indent: match[1], bullet: match[2] ?? '', content: match[3] }
  })
}

function joinLines(lines: NoteLine[]): string {
  return lines.map((l) => l.indent + l.bullet + l.content).join('\n')
}

function contentLines(lines: NoteLine[]): NoteLine[] {
  return lines.filter((l) => l.content.trim().length > 0)
}

export interface WholeNoteFormatState {
  bold: boolean
  strikethrough: boolean
  bulletList: boolean
}

export function wholeNoteFormatState(text: string): WholeNoteFormatState {
  const targets = contentLines(splitLines(text))
  if (targets.length === 0) {
    return { bold: false, strikethrough: false, bulletList: false }
  }
  const peeled = targets.map((l) => peelWraps(l.content))
  return {
    bold: peeled.every((p) => p.bold),
    strikethrough: peeled.every((p) => p.strikethrough),
    bulletList: text
      .split('\n')
      .filter((line) => line.trim().length > 0)
      .every((line) => BULLET_LINE.test(line)),
  }
}

/**
 * Toggles a wrap across every non-empty line: unwraps when all lines carry
 * the marker, otherwise wraps them all. Stray occurrences of the marker
 * inside a line are absorbed into the whole-line wrap (partial bold becomes
 * all bold rather than producing nested, broken markup).
 */
export function toggleWholeNoteWrap(text: string, marker: WholeNoteMarker): string {
  const lines = splitLines(text)
  const targets = contentLines(lines)
  if (targets.length === 0) return text

  const key = marker === '**' ? 'bold' : 'strikethrough'
  const allWrapped = targets.every((l) => peelWraps(l.content)[key])

  for (const line of targets) {
    const peeled = peelWraps(line.content)
    if (allWrapped) {
      peeled[key] = false
    } else {
      peeled[key] = true
      peeled.inner = peeled.inner.split(marker).join('')
    }
    line.content = rebuild(peeled)
  }
  return joinLines(lines)
}

/** Bullets every non-empty line, or un-bullets all when every line already is. */
export function toggleWholeNoteBullets(text: string): string {
  const lines = splitLines(text)
  const targets = lines.filter((l) => (l.bullet + l.content).trim().length > 0)
  if (targets.length === 0) return text

  const allBulleted = targets.every((l) => l.bullet !== '')
  for (const line of targets) {
    line.bullet = allBulleted ? '' : '- '
  }
  return joinLines(lines)
}

import type {
  Annotation,
  AnnotationStatus,
  CanvasEntityKind,
  TextEntityStyle,
  WorkspaceBounds,
} from '../../shared/types'
import { truncate } from '../../shared/annotation-utils'
import { regionCanvasRect } from '../runtime/page-anchor-state'
import type { FixTarget } from './fix-target'

/** One selected canvas item, flattened to what the prompt says about it. */
export interface SelectionMemberSummary {
  id: string
  kind: CanvasEntityKind
  bounds?: WorkspaceBounds
  /** Text content for text/sticky entities, inner label for shapes. */
  text?: string
  textStyle?: TextEntityStyle
  shapeKind?: string
  label?: string
  url?: string
  pageName?: string
  filePath?: string
}

/** An unresolved comment already sitting on one of the selected items. */
export interface PriorFeedbackSummary {
  text: string
  status: AnnotationStatus
  /** Element the prior comment was left on, as selector / name / component. */
  element?: string
  pageName?: string
}

export interface SelectionPromptContext {
  members: SelectionMemberSummary[]
  priorFeedback: PriorFeedbackSummary[]
}

/**
 * Runtime facts the prompt needs but cannot look up itself — resolved by the
 * orchestrator so prompt shaping stays a pure function of its inputs.
 */
export interface FixPromptContext {
  selection?: SelectionPromptContext | null
  target?: FixTarget | null
}

const REPLY_FORMAT = [
  'Reply format — REQUIRED:',
  '- Your entire final message is the only thing the user sees, so keep it brief and self-contained — no references to your steps, tool output, or anything "above" they cannot see.',
  '- End the message with one of:',
  '  <<RESOLVE>>   if you have addressed the comment (made the change or answered it)',
  '  <<WAITING>>   if you need more information from the user',
  'Do not write anything after the marker.',
]

export function buildFixPrompt(annotation: Annotation, context?: FixPromptContext): string {
  const lines: string[] = []
  const selectionIds = annotation.metadata?.selectionEntityIds ?? []
  const isSelectionComment = selectionIds.length > 0

  lines.push(
    isSelectionComment
      ? 'You are responding to a comment left over a group of items selected on a Specular canvas.'
      : 'You are responding to a comment left on a live web page.',
  )
  lines.push('')

  const pageUrl = annotation.pageAnchor?.pageUrl
  if (pageUrl) {
    lines.push(`Page URL: ${pageUrl}`)
  }

  const pageName = annotation.metadata?.pageName
  if (pageName) {
    lines.push(`Page: ${pageName}`)
  }

  const inspect = annotation.metadata?.inspectContext
  if (inspect) {
    if (inspect.sourceLocation) {
      const { file, line, column } = inspect.sourceLocation
      const ref = line != null
        ? column != null ? `${file}:${line}:${column}` : `${file}:${line}`
        : file
      lines.push(`Element source: ${ref}`)
    }
    if (inspect.reactComponents?.length) {
      lines.push(`React components (inner to outer): ${inspect.reactComponents.join(' > ')}`)
    }
    if (inspect.name) {
      lines.push(`Element name: ${inspect.name}`)
    }
    if (inspect.tagName) {
      lines.push(`Tag: <${inspect.tagName.toLowerCase()}>`)
    }
    if (inspect.textPreview) {
      lines.push(`Text preview: ${truncate(inspect.textPreview, 160)}`)
    }
    if (inspect.elementPath) {
      lines.push(`Element path: ${inspect.elementPath}`)
    }
    if (inspect.boundingBox) {
      const { x, y, width, height } = inspect.boundingBox as {
        x: number; y: number; width: number; height: number
      }
      lines.push(`Bounding box: x=${Math.round(x)} y=${Math.round(y)} w=${Math.round(width)} h=${Math.round(height)}`)
    }
  }

  if (annotation.anchor.type === 'element' && !inspect?.elementPath) {
    lines.push(`Selector: ${annotation.anchor.selector}`)
  }

  // A page-anchored region stores a document rect; report its current canvas
  // position so the coordinates read the same for both region variants.
  const canvasRect = annotation.anchor.type === 'region' ? regionCanvasRect(annotation) : null

  if (annotation.anchor.type === 'region') {
    if (canvasRect) {
      lines.push(
        `Region: x=${Math.round(canvasRect.x)} y=${Math.round(canvasRect.y)} w=${Math.round(canvasRect.width)} h=${Math.round(canvasRect.height)} (canvas coords)`,
      )
    }
    const components = annotation.metadata?.regionComponents ?? []
    for (const group of components) {
      if (!group.components.length) continue
      lines.push(`Components in region (${group.pageName}):`)
      for (const c of group.components) {
        const loc = c.sourceLocation
          ? c.sourceLocation.line != null
            ? `${c.sourceLocation.file}:${c.sourceLocation.line}`
            : c.sourceLocation.file
          : 'no source'
        lines.push(`  - ${c.name} ×${c.count} (${loc})`)
      }
    }
  }

  if (isSelectionComment) {
    lines.push(...selectionLines(annotation, context?.selection ?? null, canvasRect, selectionIds.length))
  }

  lines.push('')
  lines.push('Thread:')
  lines.push(`[${labelForAuthor(annotation.author)}] ${annotation.text}`)
  for (const reply of annotation.replies) {
    lines.push(`[${labelForAuthor(reply.author)}] ${reply.text}`)
  }

  lines.push('')
  const isSpaceFolderFix = context?.target?.kind === 'space-folder'
  lines.push('The comment may request a change or simply ask a question.')
  lines.push(
    isSpaceFolderFix
      ? '- If it calls for a change, make the minimal change to address it.'
      : '- If it calls for a change, make the minimal code change in this repo to address it, then verify typecheck when reasonable.',
  )
  lines.push('- If it is a question or needs no change, just answer it. Do not edit code only to have made a change.')

  if (isSelectionComment || isSpaceFolderFix) {
    lines.push(...editingPolicyLines(annotation, context?.target ?? null))
  }

  // Nothing to inspect when the request is about a file on disk and no page is
  // in play.
  if (pageUrl || !isSpaceFolderFix) {
    lines.push('')
    lines.push('Inspecting the live page: the specular skill already has it open. Prefer:')
    lines.push('  specular snapshot -i -f <pageId>     # element refs + accessibility tree')
    lines.push('  specular get styles @<ref>            # computed CSS for an element')
    lines.push('  specular get text @<ref>              # text content')
    lines.push('  specular get box @<ref>               # bounding box')
    lines.push(`  specular eval '<js>'                  # run JS in the page, returns result`)
    lines.push('  specular screenshot -f <pageId>      # then Read the printed path to view')
    lines.push('Do not use chrome-devtools or any other browser automation tool — specular')
    lines.push('covers every case above and has the right page already focused.')
  }
  lines.push('')
  lines.push(...REPLY_FORMAT)

  return lines.join('\n')
}

/**
 * Prompt for a resumed session. The session already holds the page context and
 * prior thread, so we only send the new user message plus the reply contract.
 */
export function buildFollowUpPrompt(replyText: string): string {
  const message = replyText.trim() || 'Continue addressing the latest feedback in this thread.'
  return [
    'The user replied on the same comment thread:',
    `[User] ${message}`,
    '',
    'Continue the thread — make a change if it calls for one, or just answer if it is a question.',
    'Use the specular skill to inspect the live page as before.',
    '',
    ...REPLY_FORMAT,
  ].join('\n')
}

/**
 * Where a selected item sits inside the annotated region, in thirds. Sticky
 * notes and drawings are placed by hand relative to what they point at, so the
 * coarse position is the only pointer the agent gets to which part of the
 * artifact a note is about.
 */
function positionInRegion(
  region: WorkspaceBounds | null,
  bounds: WorkspaceBounds | undefined,
): string | null {
  if (!region || !bounds || region.width <= 0 || region.height <= 0) return null
  const fx = (bounds.x + bounds.width / 2 - region.x) / region.width
  const fy = (bounds.y + bounds.height / 2 - region.y) / region.height
  const horizontal = fx < 1 / 3 ? 'left' : fx > 2 / 3 ? 'right' : 'center'
  const vertical = fy < 1 / 3 ? 'top' : fy > 2 / 3 ? 'bottom' : 'middle'
  if (vertical === 'middle' && horizontal === 'center') return 'the center of the region'
  if (vertical === 'middle') return `the ${horizontal} of the region`
  if (horizontal === 'center') return `the ${vertical} of the region`
  return `the ${vertical}-${horizontal} of the region`
}

function quoted(text: string): string {
  return `"${truncate(text.replace(/\s+/g, ' ').trim(), 240)}"`
}

function memberLine(
  member: SelectionMemberSummary,
  region: WorkspaceBounds | null,
): string {
  const at = positionInRegion(region, member.bounds)
  const place = at ? ` at ${at}` : ''
  switch (member.kind) {
    case 'text': {
      const noun = member.textStyle === 'sticky' ? 'sticky' : 'text note'
      return `${noun}${place}: ${member.text ? quoted(member.text) : '(empty)'}`
    }
    case 'drawing': {
      const size = member.bounds
        ? ` (${Math.round(member.bounds.width)}×${Math.round(member.bounds.height)})`
        : ''
      return `freehand drawing${at ? ` overlays ${at}` : ''}${size} — see the screenshot`
    }
    case 'shape': {
      const kind = member.shapeKind ?? 'shape'
      const label = member.text ? `: ${quoted(member.text)}` : ''
      return `${kind} shape${place}${label}`
    }
    case 'page':
      return `page${place}: ${member.url ?? '(no url)'}${member.pageName ? ` — ${member.pageName}` : ''}`
    case 'file':
      return `file${place}: ${member.filePath ?? '(no path)'}`
    case 'group':
      return `group${place}${member.label ? `: ${quoted(member.label)}` : ''}`
    default:
      return `${member.kind}${place}`
  }
}

function targetLine(annotation: Annotation): string {
  const target = annotation.metadata?.selectionTarget
  if (target?.kind === 'page') {
    return `The artifact this request is about: the page ${target.url ?? target.entityId}`
  }
  if (target?.kind === 'file') {
    return `The artifact this request is about: the file ${target.filePath ?? target.entityId}`
  }
  return 'The selection names no single artifact — read the comment to decide which of the items below the request is about.'
}

function selectionLines(
  annotation: Annotation,
  selection: SelectionPromptContext | null,
  region: WorkspaceBounds | null,
  selectedCount: number,
): string[] {
  const lines: string[] = ['']
  lines.push(
    selectedCount === 1
      ? 'Selection: 1 canvas item was selected when this comment was made.'
      : `Selection: ${selectedCount} canvas items were selected when this comment was made.`,
  )
  lines.push(targetLine(annotation))
  const members = selection?.members ?? []
  if (members.length) {
    lines.push('Selected items:')
    for (const member of members) {
      lines.push(`  - ${memberLine(member, region)}`)
    }
  }
  const priorFeedback = selection?.priorFeedback ?? []
  if (priorFeedback.length) {
    lines.push('Prior feedback in scope (unresolved comments already on these items):')
    for (const item of priorFeedback) {
      const where = item.element ? ` (on ${item.element})` : ''
      const page = item.pageName ? ` [${item.pageName}]` : ''
      lines.push(`  - [${item.status}]${page} ${quoted(item.text)}${where}`)
    }
  }
  return lines
}

/**
 * The facts that decide where the change lands, plus the default the user
 * expects when the comment does not say. Source is edited in place; an artifact
 * sitting in the user's space folder is copied first, because the original is
 * the user's own file and overwriting it destroys the "before".
 */
function editingPolicyLines(annotation: Annotation, target: FixTarget | null): string[] {
  const lines: string[] = ['', 'Where the change goes:']
  const selectionTarget = annotation.metadata?.selectionTarget
  if (target?.kind === 'repo') {
    lines.push(`- ${target.origin} is served from the repo at ${target.cwd} (your working directory).`)
    if (selectionTarget?.kind === 'page' && selectionTarget.url) {
      lines.push(`- Target page: ${selectionTarget.url}`)
    }
    lines.push('- Edit the source in place. Do not duplicate anything on the canvas.')
  } else if (target?.kind === 'space-folder') {
    lines.push(`- Target file: ${target.filePath}`)
    lines.push(`- It lives in the user's space folder (${target.cwd}, your working directory). No repo is bound to it.`)
    lines.push('- Do not overwrite the original: write your edit to a new file beside it (e.g. `name-2.ext`),')
    lines.push('  then place the copy on the canvas next to the original:')
    lines.push('    specular find-placement --width W --height H   # a free spot')
    lines.push('    specular add file <copy-path> --at x,y         # put the copy there')
  } else {
    lines.push('- No repo is bound to this artifact and no file target was recorded; say what you would change instead of guessing where to write it.')
  }
  lines.push('- An explicit instruction in the comment overrides all of the above.')
  return lines
}

function labelForAuthor(author: 'user' | 'agent'): string {
  return author === 'agent' ? 'Agent' : 'User'
}

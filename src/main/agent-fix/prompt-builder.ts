import type { Annotation } from '../../shared/types'
import { truncate } from '../../shared/annotation-utils'

const REPLY_FORMAT = [
  'Reply format — REQUIRED:',
  '- Your entire final message is the only thing the user sees, so keep it brief and self-contained — no references to your steps, tool output, or anything "above" they cannot see.',
  '- End the message with one of:',
  '  <<RESOLVE>>   if you have addressed the comment (made the change or answered it)',
  '  <<WAITING>>   if you need more information from the user',
  'Do not write anything after the marker.',
]

export function buildFixPrompt(annotation: Annotation): string {
  const lines: string[] = []

  lines.push('You are responding to a comment left on a live web page.')
  lines.push('')

  const pageUrl = annotation.metadata?.pageUrl
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

  if (annotation.anchor.type === 'region') {
    const { canvasRect } = annotation.anchor
    lines.push(
      `Region: x=${Math.round(canvasRect.x)} y=${Math.round(canvasRect.y)} w=${Math.round(canvasRect.width)} h=${Math.round(canvasRect.height)} (canvas coords)`,
    )
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

  lines.push('')
  lines.push('Thread:')
  lines.push(`[${labelForAuthor(annotation.author)}] ${annotation.text}`)
  for (const reply of annotation.replies) {
    lines.push(`[${labelForAuthor(reply.author)}] ${reply.text}`)
  }

  lines.push('')
  lines.push('The comment may request a change or simply ask a question.')
  lines.push('- If it calls for a change, make the minimal code change in this repo to address it, then verify typecheck when reasonable.')
  lines.push('- If it is a question or needs no change, just answer it. Do not edit code only to have made a change.')
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

function labelForAuthor(author: 'user' | 'agent'): string {
  return author === 'agent' ? 'Agent' : 'User'
}

import type { Annotation } from '../../shared/types'
import { truncate } from '../../shared/annotation-utils'

export function buildFixPrompt(annotation: Annotation): string {
  const lines: string[] = []

  lines.push('You are fixing a UI comment left on a live web page.')
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
  lines.push('Make the minimal code change in this repo to address the feedback.')
  lines.push('Verify your change does not break typecheck when reasonable.')
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
  lines.push('Reply format — REQUIRED:')
  lines.push('- Your final output MUST end with one short IM-style summary line (under 280 chars), then a newline, then one of:')
  lines.push('  <<RESOLVE>>   if you believe the issue is now fixed')
  lines.push('  <<WAITING>>   if you need more information from the user')
  lines.push('Do not write anything after the marker.')

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
    'Continue the fix. Use the specular skill to inspect the live page as before.',
    '',
    'Reply format — REQUIRED:',
    '- End with one short IM-style summary line (under 280 chars), then a newline, then one of:',
    '  <<RESOLVE>>   if you believe the issue is now fixed',
    '  <<WAITING>>   if you need more information from the user',
    'Do not write anything after the marker.',
  ].join('\n')
}

function labelForAuthor(author: 'user' | 'agent'): string {
  return author === 'agent' ? 'Agent' : 'User'
}

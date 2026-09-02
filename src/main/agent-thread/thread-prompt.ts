import type { AgentThread, ThreadPill, ThreadWriteTarget } from '../../shared/agent-thread'
import { pillLabel, selectionFocusPrompt } from '../../shared/agent-thread'

const REPLY_FORMAT = [
  'Reply format — REQUIRED:',
  '- Your entire final message is the only thing the user sees, so keep it brief and self-contained — no references to your steps, tool output, or anything "above" they cannot see.',
  '- End the message with one of:',
  '  <<RESOLVE>>   if you have addressed the request (made the change or answered it)',
  '  <<WAITING>>   if you need more information from the user',
  'Do not write anything after the marker.',
]

export function buildThreadPrompt(input: {
  thread: AgentThread
  pill: ThreadPill
  writeTarget: ThreadWriteTarget
  spacePath: string
}): string {
  const { thread, pill, writeTarget, spacePath } = input
  const lines: string[] = []
  lines.push('You are the in-app agent for a Specular canvas — a spatial document of live pages, notes, and files.')
  lines.push('The specular skill talks to the running app. Prefer those verbs for canvas reads and writes.')
  lines.push('')
  lines.push(`Working directory (space folder): ${spacePath}`)
  if (writeTarget.kind === 'repo') {
    lines.push(`This turn should change source for ${writeTarget.origin} in the repo at ${writeTarget.repoPath}.`)
    lines.push('Edit that repo. Pages already on the canvas reload from source.')
    lines.push('Anything new reaches the user only once it is on the canvas: `specular add page <full url> --at x,y`.')
  } else {
    lines.push('This turn is about the canvas / space, not a linked site repo.')
    lines.push('Use `specular add` / `update` / `delete` / `arrange` to change what the user sees.')
    lines.push('Files you write in this folder reach the user once they are on the canvas: `specular add file <path> --at x,y`.')
  }
  lines.push('')
  lines.push(...selectionFocusLines(pill))
  lines.push('Thread:')
  for (const message of thread.messages) {
    const who = message.role === 'agent' ? 'Agent' : 'User'
    lines.push(`[${who}] ${message.text}`)
  }
  lines.push('')
  lines.push('Inspecting a live page (when one is in play):')
  lines.push('  specular snapshot -i -f <pageId>')
  lines.push('  specular get styles @<ref>')
  lines.push('  specular screenshot -f <pageId>')
  lines.push('Do not use chrome-devtools or other browser automation — specular has the right page.')
  lines.push('')
  lines.push(...REPLY_FORMAT)
  return lines.join('\n')
}

export function buildThreadFollowUpPrompt(text: string, pill?: ThreadPill): string {
  const message = text.trim() || 'Continue addressing the latest feedback in this thread.'
  return [
    'The user followed up in the same canvas agent thread:',
    `[User] ${message}`,
    '',
    ...selectionFocusLines(pill ?? { kind: 'empty' }),
    'Continue the thread — make a change if it calls for one, or just answer if it is a question.',
    'Use the specular skill as before.',
    '',
    ...REPLY_FORMAT,
  ].join('\n')
}

function selectionFocusLines(pill: ThreadPill): string[] {
  const focus = selectionFocusPrompt(pill)
  if (!focus) {
    return [`Current selection: ${pillLabel(pill)}`, '']
  }
  return [
    focus,
    'Prefer changing those. Touch other canvas items only if the request clearly needs it.',
    '',
  ]
}

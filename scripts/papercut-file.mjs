// Shared PAPERCUTS.md reading/appending. Used by both `pnpm papercut` and
// `pnpm papercut:review`.

import { existsSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')

export const PAPERCUTS_PATH = resolve(REPO_ROOT, 'PAPERCUTS.md')

const HEADER = `# Papercuts

Small frictions hit while working in this repo — a tool call that missed, a
confusing setup step, a flaky command, a stale cache, a misleading error. None
of them are blocking on their own; logged together they show where the repo
needs sanding down.

Logged with \`pnpm papercut -m <model> "message"\`, or mined from a whole session
with \`pnpm papercut:review\`. Distinct from what an agent accomplished, and from
GitHub issues (real bugs / tracked work).
`

function localDate(now = new Date()) {
  const pad = (n) => String(n).padStart(2, '0')
  return `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())}`
}

export function readPapercuts() {
  return existsSync(PAPERCUTS_PATH) ? readFileSync(PAPERCUTS_PATH, 'utf8') : ''
}

function formatEntry({ model, message, source }) {
  const oneLine = message.replace(/\s*\n\s*/g, ' ').trim()
  const tag = source ? ` _(${source})_` : ''
  return `- \`${model}\`${tag} — ${oneLine}`
}

/**
 * Append entries under today's date heading, creating the file and heading as
 * needed. Entries are `{ model, message, source? }`.
 */
export function appendPapercuts(entries) {
  if (entries.length === 0) return

  const existing = readPapercuts()
  const today = localDate()
  let out = existing || `${HEADER}`

  if (!out.endsWith('\n')) out += '\n'

  // Date headings are only written once per day; subsequent entries land under
  // the heading already at the end of the file.
  const headings = out.match(/^## \d{4}-\d{2}-\d{2}$/gm) ?? []
  if (headings.at(-1) !== `## ${today}`) {
    out += `\n## ${today}\n\n`
  }

  out += `${entries.map(formatEntry).join('\n')}\n`

  writeFileSync(PAPERCUTS_PATH, out)
}

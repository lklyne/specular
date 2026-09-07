#!/usr/bin/env node
// Append a papercut — a small friction hit while working — to PAPERCUTS.md.
//
//   pnpm papercut -m <model> "what you were doing → what got in the way"
//
// Entries are grouped under a date heading and stamped with the model that hit
// them, so the file reads as a chronological log of where the repo needs sanding.

import { appendPapercuts, PAPERCUTS_PATH } from './papercut-file.mjs'

function parseArgs(argv) {
  const rest = []
  let model = ''
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i]
    if (arg === '-m' || arg === '--model') {
      model = argv[i + 1] ?? ''
      i += 1
    } else if (arg.startsWith('--model=')) {
      model = arg.slice('--model='.length)
    } else if (arg === '--') {
      rest.push(...argv.slice(i + 1))
      break
    } else {
      rest.push(arg)
    }
  }
  return { model, message: rest.join(' ').trim() }
}

const { model, message } = parseArgs(process.argv.slice(2))

if (!message) {
  console.error('usage: pnpm papercut -m <model> "what you were doing → what got in the way"')
  process.exit(1)
}

if (!model) {
  console.error('missing -m <model>. Pass the model that hit the friction, e.g. -m claude-opus-5')
  process.exit(1)
}

appendPapercuts([{ model, message }])
console.log(`logged to ${PAPERCUTS_PATH}`)

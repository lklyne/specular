#!/usr/bin/env node
// Mine a whole Claude Code session for papercuts and append what it finds to
// PAPERCUTS.md.
//
//   pnpm papercut:review [path/to/transcript.jsonl]
//
// Without an argument it picks the most recently modified transcript for this
// repo under ~/.claude/projects/. The transcript goes to a cheap model (Gemini
// Flash) via GOOGLE_API_KEY in .env — this is user-triggered, not something an
// agent should run on its own.

import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs'
import { homedir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { appendPapercuts, PAPERCUTS_PATH, readPapercuts } from './papercut-file.mjs'

const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const MODEL = process.env.PAPERCUT_REVIEW_MODEL || 'gemini-flash-latest'
// Flash has a very large window; this bound keeps a long session from turning
// into a multi-megabyte request.
const MAX_TRANSCRIPT_CHARS = 400_000

function loadEnvFile() {
  const envPath = resolve(REPO_ROOT, '.env')
  if (!existsSync(envPath)) return
  for (const line of readFileSync(envPath, 'utf8').split('\n')) {
    const match = /^\s*(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/.exec(line)
    if (!match) continue
    const [, key, rawValue] = match
    if (process.env[key] !== undefined) continue
    const value = rawValue.trim().replace(/^(['"])(.*)\1$/s, '$2')
    process.env[key] = value
  }
}

function transcriptDir() {
  // Claude Code stores transcripts per project, keyed by the cwd with every
  // non-alphanumeric character replaced by a dash.
  return join(homedir(), '.claude', 'projects', REPO_ROOT.replace(/[^a-zA-Z0-9]/g, '-'))
}

function latestTranscript() {
  const dir = transcriptDir()
  if (!existsSync(dir)) return null
  const candidates = readdirSync(dir)
    .filter((name) => name.endsWith('.jsonl'))
    .map((name) => join(dir, name))
    .map((path) => ({ path, mtime: statSync(path).mtimeMs }))
    .sort((a, b) => b.mtime - a.mtime)
  return candidates[0]?.path ?? null
}

function blockToText(block) {
  if (typeof block === 'string') return block
  switch (block?.type) {
    case 'text':
      return block.text
    case 'thinking':
      return ''
    case 'tool_use':
      return `[tool: ${block.name}] ${JSON.stringify(block.input).slice(0, 600)}`
    case 'tool_result': {
      const body = typeof block.content === 'string'
        ? block.content
        : (block.content ?? []).map(blockToText).join('\n')
      const prefix = block.is_error ? '[tool error] ' : '[tool result] '
      return prefix + body.slice(0, 1200)
    }
    default:
      return ''
  }
}

function flattenTranscript(path) {
  const turns = []
  for (const line of readFileSync(path, 'utf8').split('\n')) {
    if (!line.trim()) continue
    let record
    try {
      record = JSON.parse(line)
    } catch {
      continue
    }
    if (record.type !== 'user' && record.type !== 'assistant') continue
    const content = record.message?.content
    const text = (Array.isArray(content) ? content.map(blockToText) : [blockToText(content)])
      .filter(Boolean)
      .join('\n')
      .trim()
    if (text) turns.push(`${record.type.toUpperCase()}: ${text}`)
  }

  const joined = turns.join('\n\n')
  return joined.length > MAX_TRANSCRIPT_CHARS ? joined.slice(-MAX_TRANSCRIPT_CHARS) : joined
}

function buildPrompt(transcript, existingPapercuts) {
  return `You are mining a Claude Code session transcript from the Specular repo for "papercuts".

A papercut is a SMALL friction the agent hit while working: a tool call that missed and had to be
retried, a confusing or undocumented setup step, a flaky command, a stale cache, a misleading error,
a non-obvious gotcha. None of them are blocking — logged together they show where the repo needs
sanding down.

NOT papercuts: what the agent accomplished, product features it built, real bugs in the product
(those are tracked as GitHub issues), one-off mistakes by the agent that the repo could not have
prevented, or anything the user explicitly asked for.

Write each one as one or two sentences: what the agent was doing → what got in the way. A guess at
the cause or fix is a bonus. Be concrete — name the command, file, or error.

Do not repeat anything already logged here:
<existing_papercuts>
${existingPapercuts || '(none)'}
</existing_papercuts>

Transcript:
<transcript>
${transcript}
</transcript>

Respond with ONLY a JSON array of strings, one per papercut, e.g. ["...", "..."]. Return [] if the
session had no real frictions. Do not wrap it in markdown fences.`
}

async function askGemini(prompt, apiKey) {
  const url = `https://generativelanguage.googleapis.com/v1beta/models/${MODEL}:generateContent`
  const response = await fetch(url, {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'x-goog-api-key': apiKey },
    body: JSON.stringify({
      contents: [{ role: 'user', parts: [{ text: prompt }] }],
      generationConfig: { temperature: 0.2, responseMimeType: 'application/json' },
    }),
  })

  if (!response.ok) {
    const body = await response.text()
    throw new Error(`Gemini request failed (${response.status}): ${body.slice(0, 500)}`)
  }

  const payload = await response.json()
  const text = (payload.candidates?.[0]?.content?.parts ?? [])
    .map((part) => part.text ?? '')
    .join('')
    .trim()
  if (!text) throw new Error('Gemini returned an empty response')
  return text
}

function parseFindings(text) {
  const cleaned = text.replace(/^```(?:json)?\s*/i, '').replace(/```\s*$/, '').trim()
  let parsed
  try {
    parsed = JSON.parse(cleaned)
  } catch {
    throw new Error(`could not parse model output as JSON:\n${cleaned.slice(0, 500)}`)
  }
  if (!Array.isArray(parsed)) throw new Error('model output was not a JSON array')
  return parsed.filter((item) => typeof item === 'string' && item.trim()).map((item) => item.trim())
}

async function main() {
  loadEnvFile()

  const apiKey = process.env.GOOGLE_API_KEY
  if (!apiKey) {
    console.error('GOOGLE_API_KEY is not set. Add it to .env (see .env.example).')
    process.exit(1)
  }

  const explicit = process.argv[2]
  const path = explicit ? resolve(explicit) : latestTranscript()
  if (!path || !existsSync(path)) {
    console.error(
      explicit
        ? `no transcript at ${path}`
        : `no session transcript found under ${transcriptDir()}. Pass one explicitly: pnpm papercut:review <path>`,
    )
    process.exit(1)
  }

  const transcript = flattenTranscript(path)
  if (!transcript) {
    console.error(`no usable turns in ${path}`)
    process.exit(1)
  }

  console.log(`reviewing ${path} (${transcript.length} chars) with ${MODEL}…`)
  const findings = parseFindings(await askGemini(buildPrompt(transcript, readPapercuts()), apiKey))

  if (findings.length === 0) {
    console.log('no papercuts found.')
    return
  }

  appendPapercuts(findings.map((message) => ({ model: MODEL, message, source: 'review' })))
  console.log(`appended ${findings.length} papercut(s) to ${PAPERCUTS_PATH}:`)
  for (const finding of findings) console.log(`  - ${finding}`)
}

main().catch((error) => {
  console.error(error.message)
  process.exit(1)
})

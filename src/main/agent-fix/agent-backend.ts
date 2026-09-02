/**
 * Runs a fix through the Claude Agent SDK.
 *
 * The SDK bundles its own copy of the Claude Code runtime, so a fix never
 * depends on a `claude` binary being installed or on the PATH a GUI-launched
 * Electron app inherits. The one environmental dependency left is auth
 * (a Claude Code sign-in under ~/.claude, or an API key), which is checked
 * up front so failures come back as an actionable message instead of a
 * raw process error.
 */

import { homedir } from 'os'
import { join } from 'path'
import { existsSync } from 'fs'
import { query, type Options } from '@anthropic-ai/claude-agent-sdk'
import type { FixConfig, FixProgressEvent } from '../../shared/types'
import { truncate } from '../../shared/annotation-utils'
import { getFixConfig } from '../runtime/preferences'
import { describeAgentMessage } from './progress-events'

export interface FixResult {
  summary: string
  shouldResolve: boolean
  rawOutput: string
  /** Session id reported by this run, for resuming the thread on the next reply. */
  sessionId?: string
}

export interface InvokeOptions {
  onEvent?: (event: FixProgressEvent) => void
  timeout?: number
  /** Resume an existing Claude session instead of starting a fresh one. */
  resumeSessionId?: string
}

const DEFAULT_TIMEOUT_MS = 10 * 60 * 1000

const ALLOWED_TOOLS = [
  'Read', 'Edit', 'Write', 'Grep', 'Glob',
  'Bash(git status:*)', 'Bash(git diff:*)', 'Bash(git log:*)',
  'Bash(pnpm typecheck:*)', 'Bash(pnpm test:unit:*)', 'Bash(pnpm lint:*)',
  'Bash(npm run typecheck:*)', 'Bash(tsc:*)',
  'Bash(specular:*)',
]

/** The SDK query options a fix run starts with. Pure so the permission modes are testable. */
export function fixQueryOptions(
  config: FixConfig,
  cwd: string,
  resumeSessionId?: string,
): Options {
  const options: Options = {
    cwd,
    model: config.model,
    // Match `claude -p` behavior: the full Claude Code system prompt, with the
    // user's settings and skills loaded from disk (the SDK default).
    systemPrompt: { type: 'preset', preset: 'claude_code' },
  }
  if (resumeSessionId) {
    options.resume = resumeSessionId
  }
  if (config.permissions === 'dangerously') {
    options.permissionMode = 'bypassPermissions'
    options.allowDangerouslySkipPermissions = true
  } else if (config.permissions === 'acceptEdits') {
    // Headless: there is no TTY to answer a prompt, so anything outside this
    // set is denied and the agent works around it. Keep the allowlist to the
    // file edits a fix makes plus the read-only commands that verify them.
    options.permissionMode = 'acceptEdits'
    options.allowedTools = [...ALLOWED_TOOLS]
  } else {
    // A model classifier approves or denies each tool call. Pre-approve the
    // standard fix toolkit so routine operations skip the classifier round-trip.
    options.permissionMode = 'auto'
    options.allowedTools = [...ALLOWED_TOOLS]
  }
  return options
}

/**
 * The SDK runs the bundled agent as the current user, so it authenticates the
 * same way the Claude Code CLI does: the sign-in state under ~/.claude, or an
 * API key in the environment. Neither existing means every run would fail —
 * catch that before spawning and say what to do about it.
 */
export function claudeAuthMissingMessage(): string | null {
  if (existsSync(join(homedir(), '.claude'))) return null
  if (process.env.ANTHROPIC_API_KEY || process.env.CLAUDE_CODE_OAUTH_TOKEN || process.env.ANTHROPIC_AUTH_TOKEN) {
    return null
  }
  return 'Claude Code is not signed in on this machine. Install it from https://claude.com/code, run `claude` in a terminal once to sign in, then retry.'
}

type BackendFn = (
  prompt: string,
  repoPath: string,
  options: InvokeOptions,
) => Promise<FixResult>

let override: BackendFn | null = null

export function _setBackendOverride(fn: BackendFn | null): void {
  override = fn
}

interface StreamAcc {
  rawOutput: string
  finalText: string
  sessionId?: string
  resultError: string | null
}

function recordStderr(acc: StreamAcc, data: string, onEvent?: InvokeOptions['onEvent']): void {
  acc.rawOutput += data
  const line = data.trim().split(/\r?\n/).pop() ?? ''
  if (!line || !onEvent) return
  onEvent({
    kind: 'stderr',
    text: truncate(line, 320),
    timestamp: new Date().toISOString(),
  })
}

function recordResultMessage(acc: StreamAcc, message: { type: string; subtype?: string; result?: string; errors?: string[] }): void {
  if (message.type !== 'result') return
  if (message.subtype === 'success') {
    acc.finalText = message.result ?? ''
    acc.rawOutput += acc.finalText
    return
  }
  acc.resultError = message.errors?.length ? message.errors.join('; ') : (message.subtype ?? 'error')
  acc.rawOutput += acc.resultError
}

async function consumeFixStream(
  stream: AsyncIterable<unknown>,
  acc: StreamAcc,
  onEvent?: InvokeOptions['onEvent'],
): Promise<void> {
  for await (const message of stream) {
    recordResultMessage(acc, message as { type: string; subtype?: string; result?: string; errors?: string[] })
    const described = describeAgentMessage(message)
    if (!described) continue
    if (described.sessionId) acc.sessionId = described.sessionId
    if (onEvent) onEvent(described.event)
  }
}

export async function runFixAgent(
  prompt: string,
  repoPath: string,
  options: InvokeOptions = {},
): Promise<FixResult> {
  if (override) return override(prompt, repoPath, options)

  const authError = claudeAuthMissingMessage()
  if (authError) throw new Error(authError)

  const timeout = options.timeout ?? DEFAULT_TIMEOUT_MS
  const abortController = new AbortController()
  const timer = setTimeout(() => abortController.abort(), timeout)
  const acc: StreamAcc = { rawOutput: '', finalText: '', resultError: null }

  try {
    const stream = query({
      prompt,
      options: {
        ...fixQueryOptions(getFixConfig(), repoPath, options.resumeSessionId),
        abortController,
        stderr: (data: string) => recordStderr(acc, data, options.onEvent),
      },
    })
    await consumeFixStream(stream, acc, options.onEvent)
  } catch (err) {
    if (abortController.signal.aborted) {
      throw new Error(`Fix agent timed out after ${timeout}ms`)
    }
    throw err
  } finally {
    clearTimeout(timer)
  }

  if (!acc.finalText && acc.resultError) {
    throw new Error(`Fix agent failed: ${acc.resultError}`)
  }
  const parsed = parseOutput(acc.finalText || acc.rawOutput)
  return { ...parsed, rawOutput: acc.rawOutput, sessionId: acc.sessionId }
}

const RESOLVE_MARKER = '<<RESOLVE>>'
const WAITING_MARKER = '<<WAITING>>'
// The agent's whole final message is shown to the user verbatim; the marker
// only carries the resolve/waiting hint. Cap as a runaway guard — the prompt
// asks for brevity.
const MAX_REPLY_CHARS = 2000

export function parseOutput(stdout: string): { summary: string; shouldResolve: boolean } {
  const text = stdout.trim()
  if (!text) {
    return { summary: '(no output)', shouldResolve: false }
  }
  // Last marker wins; everything before it is the user-facing answer.
  const resolveIdx = text.lastIndexOf(RESOLVE_MARKER)
  const waitingIdx = text.lastIndexOf(WAITING_MARKER)
  const markerIdx = Math.max(resolveIdx, waitingIdx)
  if (markerIdx === -1) {
    return { summary: truncate(text, MAX_REPLY_CHARS), shouldResolve: false }
  }
  const answer = text.slice(0, markerIdx).trim()
  return {
    summary: truncate(answer || '(no summary)', MAX_REPLY_CHARS),
    shouldResolve: resolveIdx > waitingIdx,
  }
}

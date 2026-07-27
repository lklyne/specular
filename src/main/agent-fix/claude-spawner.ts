import { spawn } from 'child_process'
import type { FixProgressEvent } from '../../shared/types'
import { truncate } from '../../shared/annotation-utils'
import { getFixConfig } from '../runtime/preferences'
import { parseStreamLine } from './stream-json-parser'

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
]

type SpawnerFn = (
  prompt: string,
  repoPath: string,
  options: InvokeOptions,
) => Promise<FixResult>

let override: SpawnerFn | null = null

export function _setSpawnerOverride(fn: SpawnerFn | null): void {
  override = fn
}

export function invokeClaude(
  prompt: string,
  repoPath: string,
  options: InvokeOptions = {},
): Promise<FixResult> {
  if (override) return override(prompt, repoPath, options)
  const timeout = options.timeout ?? DEFAULT_TIMEOUT_MS

  return new Promise<FixResult>((resolve, reject) => {
    const config = getFixConfig()
    const args = [
      '-p', prompt,
      '--output-format', 'stream-json',
      '--verbose',
    ]
    if (options.resumeSessionId) {
      args.push('--resume', options.resumeSessionId)
    }
    if (config.model !== 'opus') {
      args.push('--model', `claude-${config.model}-4-6`)
    }
    if (config.permissions === 'dangerously') {
      args.push('--dangerously-skip-permissions')
    } else if (config.permissions === 'acceptEdits') {
      // Headless: there is no TTY to answer a prompt, so anything outside this
      // set is denied and the agent works around it. Keep the allowlist to the
      // read-only/verify commands a fix run actually needs.
      args.push('--permission-mode', 'acceptEdits')
      args.push('--allowedTools', ALLOWED_TOOLS.join(' '))
    }
    const child = spawn(
      'claude',
      args,
      {
        stdio: ['pipe', 'pipe', 'pipe'],
        cwd: repoPath,
        env: { ...process.env, NO_COLOR: '1' },
      },
    )

    let stdoutBuffer = ''
    let rawOutput = ''
    let finalText = ''
    let sessionId: string | undefined

    const timer = setTimeout(() => {
      child.kill('SIGKILL')
      reject(new Error(`claude timed out after ${timeout}ms`))
    }, timeout)

    const handleLine = (line: string): void => {
      const parsed = parseStreamLine(line)
      if (!parsed) return
      if (parsed.finalText != null) finalText = parsed.finalText
      if (parsed.sessionId) sessionId = parsed.sessionId
      if (options.onEvent) options.onEvent(parsed.event)
    }

    child.stdout.on('data', (chunk: Buffer) => {
      const text = chunk.toString('utf-8')
      rawOutput += text
      stdoutBuffer += text
      let newlineIdx = stdoutBuffer.indexOf('\n')
      while (newlineIdx !== -1) {
        handleLine(stdoutBuffer.slice(0, newlineIdx))
        stdoutBuffer = stdoutBuffer.slice(newlineIdx + 1)
        newlineIdx = stdoutBuffer.indexOf('\n')
      }
    })

    child.stderr.on('data', (chunk: Buffer) => {
      const text = chunk.toString('utf-8')
      rawOutput += text
      const line = text.trim().split(/\r?\n/).pop() ?? ''
      if (line && options.onEvent) {
        options.onEvent({
          kind: 'stderr',
          text: truncate(line, 320),
          timestamp: new Date().toISOString(),
        })
      }
    })

    child.on('error', (err) => {
      clearTimeout(timer)
      reject(err)
    })

    child.on('close', (code) => {
      clearTimeout(timer)
      if (stdoutBuffer.trim()) handleLine(stdoutBuffer)
      if (code !== 0 && !finalText) {
        reject(new Error(`claude exited with code ${code}`))
        return
      }
      const parsed = parseOutput(finalText || rawOutput)
      resolve({ ...parsed, rawOutput, sessionId })
    })

    child.stdin.end()
  })
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

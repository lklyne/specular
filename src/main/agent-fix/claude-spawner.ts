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

export function parseOutput(stdout: string): { summary: string; shouldResolve: boolean } {
  const lines = stdout.split(/\r?\n/).map((line) => line.trim()).filter((line) => line.length > 0)
  if (lines.length === 0) {
    return { summary: '(no output)', shouldResolve: false }
  }
  let markerIndex = -1
  let shouldResolve = false
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i]
    if (line === '<<RESOLVE>>' || line.endsWith('<<RESOLVE>>')) {
      markerIndex = i
      shouldResolve = true
      break
    }
    if (line === '<<WAITING>>' || line.endsWith('<<WAITING>>')) {
      markerIndex = i
      shouldResolve = false
      break
    }
  }
  if (markerIndex === -1) {
    return { summary: truncate(lines[lines.length - 1], 280), shouldResolve: false }
  }
  const markerLine = lines[markerIndex]
  const markerToken = shouldResolve ? '<<RESOLVE>>' : '<<WAITING>>'
  const inlineSummary = markerLine === markerToken
    ? ''
    : markerLine.slice(0, markerLine.length - markerToken.length).trim()
  const summary = inlineSummary || (markerIndex > 0 ? lines[markerIndex - 1] : '')
  return {
    summary: truncate(summary || '(no summary)', 280),
    shouldResolve,
  }
}

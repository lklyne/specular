/**
 * The canvas agent, on the Claude Agent SDK. One process is one run.
 *
 * stdin, one JSON value a line:
 *   {"type":"start", ...Start}      first, once
 *   {"type":"interrupt"}            stops the turn in flight
 *   {"type":"set_model","model"}    changes the model for what follows
 *
 * stdout: every SDK message as it arrives, unchanged, one a line. These are
 * the lines `claude --output-format stream-json` writes. The helper's own
 * lines have `"type":"helper"`.
 */

import { existsSync } from 'node:fs'
import { homedir } from 'node:os'
import { join } from 'node:path'
import { query, type Options, type Query, type SDKUserMessage } from '@anthropic-ai/claude-agent-sdk'
import { received, send } from './lines'

type Permissions = 'auto' | 'acceptEdits' | 'dangerously'

interface StartImage {
  mediaType: 'image/png' | 'image/jpeg' | 'image/gif' | 'image/webp'
  /** Base64. */
  data: string
}

interface Start {
  type: 'start'
  /** Empty with `models: true`, to ask what can be chosen and run nothing. */
  prompt: string
  images?: StartImage[]
  cwd: string
  model?: string
  permissions: Permissions
  allowedTools: string[]
  appendSystemPrompt: string
  resume?: string
  /** The Claude Code runtime to run, when it is not the SDK's own. */
  runtime?: string
  /** Answer with the models that can be chosen, then end. */
  models?: boolean
}

function fail(code: string, message: string): never {
  send({ type: 'helper', event: 'error', code, message })
  process.exit(1)
}

/** A sign-in the runtime can use, or what to do about not having one. */
function signedIn(): boolean {
  const env = process.env
  return (
    existsSync(join(homedir(), '.claude')) ||
    Boolean(env.ANTHROPIC_API_KEY || env.CLAUDE_CODE_OAUTH_TOKEN || env.ANTHROPIC_AUTH_TOKEN)
  )
}

function options(start: Start): Options {
  const options: Options = {
    cwd: start.cwd,
    model: start.model,
    systemPrompt: { type: 'preset', preset: 'claude_code', append: start.appendSystemPrompt },
    skills: 'all',
    settings: { outputStyle: 'Concise' },
    includePartialMessages: true,
    stderr: (data) => process.stderr.write(data),
  }
  if (start.runtime) options.pathToClaudeCodeExecutable = start.runtime
  if (start.resume) options.resume = start.resume
  if (start.permissions === 'dangerously') {
    options.permissionMode = 'bypassPermissions'
    options.allowDangerouslySkipPermissions = true
  } else {
    options.permissionMode = start.permissions
    options.allowedTools = start.allowedTools
  }
  return options
}

/** The turn's prompt: text alone, or one user message carrying the images first. */
function prompt(start: Start): string | AsyncIterable<SDKUserMessage> {
  const images = start.images ?? []
  if (images.length === 0) return start.prompt
  const message: SDKUserMessage = {
    type: 'user',
    parent_tool_use_id: null,
    message: {
      role: 'user',
      content: [
        ...images.map((image) => ({
          type: 'image' as const,
          source: { type: 'base64' as const, media_type: image.mediaType, data: image.data },
        })),
        { type: 'text' as const, text: start.prompt },
      ],
    },
  }
  return (async function* () {
    yield message
  })()
}

/** A prompt that never sends, so the runtime starts and can be asked things. */
async function* nothing(): AsyncGenerator<SDKUserMessage> {
  await new Promise(() => {})
}

async function models(start: Start): Promise<void> {
  const run = query({ prompt: nothing(), options: options(start) })
  const [models, account] = await Promise.all([run.supportedModels(), run.accountInfo()])
  send({ type: 'helper', event: 'models', models, account })
  run.close()
}

async function commands(run: Query, input: AsyncGenerator<unknown>): Promise<void> {
  for await (const value of input) {
    const command = value as { type?: string; model?: string }
    if (command.type === 'interrupt') await run.interrupt()
    else if (command.type === 'set_model') await run.setModel(command.model)
  }
}

export async function agent(): Promise<void> {
  const input = received()
  const first = (await input.next()).value as Start | undefined
  if (first?.type !== 'start') fail('protocol', 'the first line was not a start')
  if (!signedIn()) {
    fail(
      'signed_out',
      'Claude is not signed in on this machine. Install Claude Code from https://claude.com/code, run `claude` in a terminal once to sign in, then retry.',
    )
  }
  if (first.models) return models(first)

  const run = query({ prompt: prompt(first), options: options(first) })
  // Stdin closing is the app going away, not the end of the run.
  void commands(run, input).catch((error) => process.stderr.write(`${error}\n`))
  try {
    for await (const message of run) send(message)
  } catch (error) {
    fail('run', error instanceof Error ? error.message : String(error))
  }
}

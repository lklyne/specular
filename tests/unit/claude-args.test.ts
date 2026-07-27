import { describe, expect, it } from 'vitest'
import { claudeArgs } from '../../src/main/agent-fix/claude-spawner'
import type { FixConfig } from '../../src/shared/types'

const config = (over: Partial<FixConfig> = {}): FixConfig => ({
  model: 'opus',
  permissions: 'acceptEdits',
  configured: true,
  ...over,
})

describe('claudeArgs', () => {
  it('gives acceptEdits an allowlist, since a headless run has no TTY to answer a prompt', () => {
    const args = claudeArgs('fix it', config())
    const i = args.indexOf('--permission-mode')
    expect(args[i + 1]).toBe('acceptEdits')
    const allowed = args[args.indexOf('--allowedTools') + 1]!
    expect(allowed).toContain('Edit')
    expect(allowed).toContain('Bash(pnpm typecheck:*)')
    expect(args).not.toContain('--dangerously-skip-permissions')
  })

  it('sends no permission flags at all under the read-only default', () => {
    const args = claudeArgs('fix it', config({ permissions: 'default' }))
    expect(args).not.toContain('--permission-mode')
    expect(args).not.toContain('--allowedTools')
    expect(args).not.toContain('--dangerously-skip-permissions')
  })

  it('skips the allowlist when permissions are bypassed — it would only narrow them', () => {
    const args = claudeArgs('fix it', config({ permissions: 'dangerously' }))
    expect(args).toContain('--dangerously-skip-permissions')
    expect(args).not.toContain('--allowedTools')
  })

  it('names a model only when it is not the default opus', () => {
    expect(claudeArgs('x', config())).not.toContain('--model')
    const args = claudeArgs('x', config({ model: 'haiku' }))
    expect(args[args.indexOf('--model') + 1]).toBe('claude-haiku-4-6')
  })

  it('resumes a session when one is handed in', () => {
    const args = claudeArgs('x', config(), 'sess_1')
    expect(args[args.indexOf('--resume') + 1]).toBe('sess_1')
    expect(claudeArgs('x', config())).not.toContain('--resume')
  })
})

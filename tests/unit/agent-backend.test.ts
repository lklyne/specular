import { describe, expect, it } from 'vitest'
import { fixQueryOptions, parseOutput } from '../../src/main/agent-fix/agent-backend'
import type { FixConfig } from '../../src/shared/types'

const config = (over: Partial<FixConfig> = {}): FixConfig => ({
  model: 'opus',
  permissions: 'acceptEdits',
  configured: true,
  ...over,
})

describe('fixQueryOptions', () => {
  it('gives acceptEdits an allowlist, since a headless run has no TTY to answer a prompt', () => {
    const options = fixQueryOptions(config(), '/repo')
    expect(options.permissionMode).toBe('acceptEdits')
    expect(options.allowedTools).toContain('Edit')
    expect(options.allowedTools).toContain('Bash(pnpm typecheck:*)')
    expect(options.allowDangerouslySkipPermissions).toBeUndefined()
  })

  it('routes auto through the classifier with the fix toolkit pre-approved', () => {
    const options = fixQueryOptions(config({ permissions: 'auto' }), '/repo')
    expect(options.permissionMode).toBe('auto')
    expect(options.allowedTools).toContain('Edit')
    expect(options.allowDangerouslySkipPermissions).toBeUndefined()
  })

  it('skips the allowlist when permissions are bypassed — it would only narrow them', () => {
    const options = fixQueryOptions(config({ permissions: 'dangerously' }), '/repo')
    expect(options.permissionMode).toBe('bypassPermissions')
    expect(options.allowDangerouslySkipPermissions).toBe(true)
    expect(options.allowedTools).toBeUndefined()
  })

  it('always names the model, since the bundled runtime has no user default to lean on', () => {
    expect(fixQueryOptions(config(), '/repo').model).toBe('opus')
    expect(fixQueryOptions(config({ model: 'haiku' }), '/repo').model).toBe('haiku')
  })

  it('resumes a session when one is handed in', () => {
    expect(fixQueryOptions(config(), '/repo', 'sess_1').resume).toBe('sess_1')
    expect(fixQueryOptions(config(), '/repo').resume).toBeUndefined()
  })

  it('runs in the repo with the full Claude Code system prompt', () => {
    const options = fixQueryOptions(config(), '/repo')
    expect(options.cwd).toBe('/repo')
    expect(options.systemPrompt).toEqual({ type: 'preset', preset: 'claude_code' })
  })
})

describe('parseOutput', () => {
  it('keeps the full message before <<RESOLVE>> and marks resolved', () => {
    const stdout = 'Shrunk the header padding to 12px.\n\nAlternatives:\n- 8px for tighter\n<<RESOLVE>>\n'
    expect(parseOutput(stdout)).toEqual({
      summary: 'Shrunk the header padding to 12px.\n\nAlternatives:\n- 8px for tighter',
      shouldResolve: true,
    })
  })

  it('treats <<WAITING>> as not resolving and keeps the message', () => {
    const stdout = 'Looked at the component.\nNeed clarification on spacing.\n<<WAITING>>'
    expect(parseOutput(stdout)).toEqual({
      summary: 'Looked at the component.\nNeed clarification on spacing.',
      shouldResolve: false,
    })
  })

  it('handles the marker inline with the message', () => {
    const stdout = 'Header padding reduced. <<RESOLVE>>'
    expect(parseOutput(stdout)).toEqual({
      summary: 'Header padding reduced.',
      shouldResolve: true,
    })
  })

  it('keeps the whole output when no marker is present', () => {
    const stdout = 'Line one\nLine two\nFinal line without marker'
    expect(parseOutput(stdout)).toEqual({
      summary: 'Line one\nLine two\nFinal line without marker',
      shouldResolve: false,
    })
  })

  it('handles empty output', () => {
    expect(parseOutput('')).toEqual({
      summary: '(no output)',
      shouldResolve: false,
    })
  })

  it('truncates a runaway message', () => {
    const long = 'x'.repeat(2400)
    const result = parseOutput(`${long}\n<<RESOLVE>>`)
    expect(result.shouldResolve).toBe(true)
    expect(result.summary.length).toBeLessThanOrEqual(2000)
    expect(result.summary.endsWith('…')).toBe(true)
  })
})

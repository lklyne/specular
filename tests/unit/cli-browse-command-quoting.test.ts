// Mutation-verified: temporarily reverted buildTargetCommand/buildWaitCommand
// in src/main/cli-commands.ts to their pre-fix form (unquoted `${ref}` /
// `"${text}"` interpolation) and confirmed the round-trip assertions below
// fail — a `text=Sign in` target re-splits into two tokens ("text=Sign" and
// "in"), and --text/--url values are silently dropped from the constructed
// command. Restored afterward.
import { describe, it, expect } from 'vitest'
import { buildTargetCommand, buildWaitCommand } from '../../src/main/cli-commands'
import { splitShellArgs } from '../../src/main/shared/browse-handler'

describe('buildTargetCommand quoting round-trip', () => {
  it('keeps a text= selector containing a space as one token', () => {
    const cmd = buildTargetCommand('click', 'text=Sign in')
    expect(splitShellArgs(cmd)).toEqual(['click', 'text=Sign in'])
  })

  it('keeps a bare @eN ref unquoted (no shell-significant chars)', () => {
    const cmd = buildTargetCommand('click', '@e5')
    expect(cmd).toBe('click @e5')
    expect(splitShellArgs(cmd)).toEqual(['click', '@e5'])
  })

  it('round-trips ref + text for fill, including embedded double quotes', () => {
    const cmd = buildTargetCommand('fill', '@e12', 'say "hi" to them')
    expect(splitShellArgs(cmd)).toEqual(['fill', '@e12', 'say "hi" to them'])
  })

  it('round-trips a CSS selector target with spaces for select', () => {
    const cmd = buildTargetCommand('select', 'div.menu > option', 'Second')
    expect(splitShellArgs(cmd)).toEqual(['select', 'div.menu > option', 'Second'])
  })

  it('round-trips a type target with a single quote in the text', () => {
    const cmd = buildTargetCommand('type', '@e3', "it's here")
    expect(splitShellArgs(cmd)).toEqual(['type', '@e3', "it's here"])
  })
})

describe('buildWaitCommand forwards --text/--url quoted', () => {
  it('forwards a --text value containing a space, quoted', () => {
    const cmd = buildWaitCommand({ text: 'Sign in' })
    expect(cmd).toBe("wait --text 'Sign in'")
    expect(splitShellArgs(cmd)).toEqual(['wait', '--text', 'Sign in'])
  })

  it('forwards a --url value containing shell-significant chars, quoted', () => {
    const cmd = buildWaitCommand({ url: 'https://example.com/a?b=1' })
    expect(splitShellArgs(cmd)).toEqual(['wait', '--url', 'https://example.com/a?b=1'])
  })

  it('leaves a plain --url value unquoted (no shell-significant chars)', () => {
    const cmd = buildWaitCommand({ url: 'https://example.com/a' })
    expect(cmd).toBe('wait --url https://example.com/a')
  })

  it('combines --load, positional, --timeout, --text, and --url', () => {
    const cmd = buildWaitCommand({
      load: 'networkidle',
      positional: '@e1',
      timeout: '5000',
      text: 'Loading complete',
      url: 'https://example.com/done',
    })
    expect(splitShellArgs(cmd)).toEqual([
      'wait', '--load', 'networkidle', '@e1', '--timeout', '5000',
      '--text', 'Loading complete', '--url', 'https://example.com/done',
    ])
  })

  it('omits --text/--url entirely when not provided', () => {
    expect(buildWaitCommand({ timeout: '1000' })).toBe('wait --timeout 1000')
  })
})

import { describe, expect, it } from 'vitest'
import { parseOutput } from '../../src/main/agent-fix/claude-spawner'

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

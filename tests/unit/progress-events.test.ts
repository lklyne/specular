import { describe, expect, it } from 'vitest'
import { describeAgentMessage } from '../../src/main/agent-fix/progress-events'

describe('describeAgentMessage', () => {
  it('returns null for non-message input', () => {
    expect(describeAgentMessage(null)).toBeNull()
    expect(describeAgentMessage('text')).toBeNull()
  })

  it('describes system init messages and carries the session id', () => {
    const described = describeAgentMessage({
      type: 'system',
      subtype: 'init',
      model: 'claude-sonnet-4-6',
      session_id: 'sess_1',
    })
    expect(described?.event.kind).toBe('system')
    expect(described?.event.text).toContain('claude-sonnet-4-6')
    expect(described?.sessionId).toBe('sess_1')
  })

  it('drops non-init system messages so status pings cannot flood the panel', () => {
    expect(describeAgentMessage({ type: 'system', subtype: 'status', status: null })).toBeNull()
  })

  it('extracts assistant text blocks', () => {
    const described = describeAgentMessage({
      type: 'assistant',
      message: {
        content: [{ type: 'text', text: 'Looking at the header component.' }],
      },
    })
    expect(described?.event.kind).toBe('text')
    expect(described?.event.text).toBe('Looking at the header component.')
  })

  it('describes tool_use blocks with the most useful hint', () => {
    const described = describeAgentMessage({
      type: 'assistant',
      message: {
        content: [{
          type: 'tool_use',
          name: 'Read',
          input: { file_path: '/repo/src/Header.tsx' },
        }],
      },
    })
    expect(described?.event.kind).toBe('tool_use')
    expect(described?.event.text).toBe('Read /repo/src/Header.tsx')
  })

  it('describes tool_result user messages', () => {
    const described = describeAgentMessage({
      type: 'user',
      message: {
        content: [{ type: 'tool_result', content: 'first line\nsecond line' }],
      },
    })
    expect(described?.event.kind).toBe('tool_result')
    expect(described?.event.text).toBe('first line')
  })

  it('labels image tool_result content instead of showing empty', () => {
    const described = describeAgentMessage({
      type: 'user',
      message: {
        content: [{
          type: 'tool_result',
          content: [{ type: 'image', source: { media_type: 'image/png' } }],
        }],
      },
    })
    expect(described?.event.kind).toBe('tool_result')
    expect(described?.event.text).toBe('image (image/png)')
  })

  it('emits (empty output) instead of dropping empty string tool_results', () => {
    const described = describeAgentMessage({
      type: 'user',
      message: { content: [{ type: 'tool_result', content: '' }] },
    })
    expect(described?.event.kind).toBe('tool_result')
    expect(described?.event.text).toBe('(empty output)')
  })

  it('surfaces is_error flag in tool_result', () => {
    const described = describeAgentMessage({
      type: 'user',
      message: {
        content: [{ type: 'tool_result', content: 'ENOENT: no such file', is_error: true }],
      },
    })
    expect(described?.event.kind).toBe('tool_result')
    expect(described?.event.text).toContain('tool error')
  })

  it('carries finalText on result messages', () => {
    const described = describeAgentMessage({
      type: 'result',
      subtype: 'success',
      result: 'Shrunk the header padding to 12px.\n<<RESOLVE>>',
    })
    expect(described?.event.kind).toBe('result')
    expect(described?.finalText).toContain('<<RESOLVE>>')
  })

  describe('run bar labels', () => {
    const toolLabel = (name: string, input: unknown) =>
      describeAgentMessage({
        type: 'assistant',
        message: { content: [{ type: 'tool_use', name, input }] },
      })?.event.label

    it('names the file for reads and edits', () => {
      expect(toolLabel('Read', { file_path: '/repo/src/Header.tsx' })).toBe('Reading Header.tsx')
      expect(toolLabel('Edit', { file_path: '/repo/src/Header.tsx' })).toBe('Editing Header.tsx')
    })

    it('prefers the Bash description over the raw command', () => {
      expect(toolLabel('Bash', { command: 'cd /x && grep -n foo', description: 'Find foo usages' })).toBe(
        'Find foo usages',
      )
      expect(toolLabel('Bash', { command: 'pnpm typecheck' })).toBe('Running pnpm')
    })

    it('quotes search patterns and humanizes MCP tool names', () => {
      expect(toolLabel('Grep', { pattern: 'command palette' })).toBe('Searching for “command palette”')
      expect(toolLabel('mcp__specular__create_page', {})).toBe('Using create page')
    })

    it('labels narration by its first sentence and the last block wins', () => {
      const described = describeAgentMessage({
        type: 'assistant',
        message: {
          content: [
            { type: 'text', text: 'Found it. Now fixing.' },
            { type: 'tool_use', name: 'Read', input: { file_path: '/a/b.ts' } },
          ],
        },
      })
      expect(described?.event.label).toBe('Reading b.ts')
      expect(
        describeAgentMessage({ type: 'assistant', message: { content: [{ type: 'text', text: 'Found it. Now fixing.' }] } })
          ?.event.label,
      ).toBe('Found it.')
    })

    it('leaves tool results unlabelled so the bar keeps the running step', () => {
      const described = describeAgentMessage({
        type: 'user',
        message: { content: [{ type: 'tool_result', content: 'ok' }] },
      })
      expect(described?.event.label).toBeUndefined()
    })
  })
})

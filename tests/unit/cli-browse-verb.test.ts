import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'

// `specular browse` is CLI parity with the MCP `browse` tool: a raw command
// string (possibly chained with `&&`) forwarded to handleBrowse exactly as
// the MCP tool does. Mock handleBrowse itself (not spawn/callApp) so these
// tests assert on the args the CLI verb builds, not on a live app.
const handleBrowseMock = vi.hoisted(() =>
  vi.fn(async () => ({ content: [{ type: 'text' as const, text: 'ok' }] })),
)

vi.mock('../../src/main/shared/browse-handler', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../src/main/shared/browse-handler')>()
  return { ...actual, handleBrowse: handleBrowseMock }
})

import { dispatch } from '../../src/main/cli-commands'

describe('specular browse', () => {
  let stderrSpy: ReturnType<typeof vi.spyOn>
  let stdoutSpy: ReturnType<typeof vi.spyOn>

  beforeEach(() => {
    handleBrowseMock.mockClear()
    stderrSpy = vi.spyOn(process.stderr, 'write').mockImplementation(() => true)
    stdoutSpy = vi.spyOn(process.stdout, 'write').mockImplementation(() => true)
  })

  afterEach(() => {
    stderrSpy.mockRestore()
    stdoutSpy.mockRestore()
  })

  function stderrText(): string {
    return stderrSpy.mock.calls.map((call) => String(call[0])).join('')
  }

  it('usage-errors and exits 1 when the command string is missing', async () => {
    const code = await dispatch(['browse', '-f', 'page-1'])
    expect(code).toBe(1)
    expect(stderrText()).toContain('usage: specular browse')
    expect(handleBrowseMock).not.toHaveBeenCalled()
  })

  it('usage-errors and exits 1 when the page is missing', async () => {
    const code = await dispatch(['browse', 'snapshot -i && click @e3'])
    expect(code).toBe(1)
    expect(stderrText()).toContain('usage: specular browse')
    expect(handleBrowseMock).not.toHaveBeenCalled()
  })

  it('forwards the command string and page id to handleBrowse unchanged, like the MCP tool', async () => {
    const code = await dispatch(['browse', 'snapshot -i && click @e3', '-f', 'page-42'])
    expect(code).toBe(0)
    expect(handleBrowseMock).toHaveBeenCalledWith({
      page_id: 'page-42',
      command: 'snapshot -i && click @e3',
      echo: false,
    })
  })

  it('accepts --page as well as -f', async () => {
    await dispatch(['browse', 'get text', '--page', 'page-7'])
    expect(handleBrowseMock).toHaveBeenCalledWith({
      page_id: 'page-7',
      command: 'get text',
      echo: false,
    })
  })
})

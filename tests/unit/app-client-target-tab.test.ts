import { afterEach, beforeEach, describe, expect, it } from 'vitest'
import { mkdtempSync, writeFileSync, rmSync } from 'fs'
import { tmpdir } from 'os'
import { join } from 'path'
import { APP_CONTROL_VERSION } from '../../src/shared/constants'

// The MCP helper is one long-lived process that can service overlapping tool
// calls, so `withTargetTab` scopes a `tab` ref to one call's async chain via
// AsyncLocalStorage rather than the CLI's module-level `setTargetTabRef`.
// This drives two overlapping scopes through real `callApp` requests (fetch
// mocked) and asserts each request carries only its own scope's header.
describe('withTargetTab', () => {
  let discoveryFile: string
  let tmpDir: string
  let requests: Array<{ url: string; tab: string | undefined }>

  beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'specular-mcp-tab-test-'))
    discoveryFile = join(tmpDir, 'discovery.json')
    writeFileSync(
      discoveryFile,
      JSON.stringify({ port: 61234, secret: 'test-secret', version: APP_CONTROL_VERSION }),
    )
    process.env.SPECULAR_DISCOVERY_FILE = discoveryFile
    requests = []
    global.fetch = (async (url: string, init?: RequestInit) => {
      const headers = init?.headers as Record<string, string> | undefined
      requests.push({ url: String(url), tab: headers?.['x-specular-tab'] })
      return {
        ok: true,
        status: 200,
        json: async () => ({}),
      } as Response
    }) as typeof fetch
  })

  afterEach(() => {
    delete process.env.SPECULAR_DISCOVERY_FILE
    rmSync(tmpDir, { recursive: true, force: true })
  })

  it('sends each overlapping scope its own tab header, not the other scope\'s', async () => {
    const { callApp, withTargetTab } = await import('../../src/main/shared/app-client')

    // Both calls are in flight at once (fetch resolves on a microtask), so a
    // module-level target-tab implementation would let one leak into the
    // other; the AsyncLocalStorage scope must keep them apart.
    await Promise.all([
      withTargetTab('canvas-a', () => callApp('/canvas')),
      withTargetTab('canvas-b', () => callApp('/canvas')),
    ])

    expect(requests).toHaveLength(2)
    expect(requests.map((r) => r.tab).sort()).toEqual([
      encodeURIComponent('canvas-a'),
      encodeURIComponent('canvas-b'),
    ])
  })

  it('sends no tab header for a scope with no ref, even while another scope has one', async () => {
    const { callApp, withTargetTab } = await import('../../src/main/shared/app-client')

    await Promise.all([
      withTargetTab('canvas-a', () => callApp('/canvas')),
      withTargetTab(undefined, () => callApp('/canvas')),
    ])

    expect(requests).toHaveLength(2)
    expect(requests.filter((r) => r.tab === encodeURIComponent('canvas-a'))).toHaveLength(1)
    expect(requests.filter((r) => r.tab === undefined)).toHaveLength(1)
  })

  it('falls back to the module-level target (setTargetTabRef) outside any scope', async () => {
    const { callApp, setTargetTabRef } = await import('../../src/main/shared/app-client')

    setTargetTabRef('cli-tab')
    await callApp('/canvas')
    setTargetTabRef(null)

    expect(requests[0].tab).toBe(encodeURIComponent('cli-tab'))
  })
})

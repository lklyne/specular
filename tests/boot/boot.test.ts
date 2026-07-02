/**
 * Boot checks — the only tests that spawn real Electron.
 *
 * Everything data-shaped lives in tests/integration/ (in-process, gated in
 * CI). This suite answers the one question that genuinely needs a window:
 * does the packaged wiring boot — views, HTTP server, IPC registration,
 * workspace restore — and round-trip one real mutation end to end?
 *
 * Run pre-release (`pnpm test:boot`) — requires a built app (`.vite/build`)
 * and the Electron binary. Not part of per-change CI.
 */

import { readFileSync } from 'fs'
import { describe, expect, it } from 'vitest'
import { BOOT_ENV_FILE } from './global-setup'
import type { JsonCanvasDocument } from '../../src/shared/json-canvas-types'

function env(): { port: number; secret: string } {
  return JSON.parse(readFileSync(BOOT_ENV_FILE, 'utf8'))
}

function headers(): Record<string, string> {
  return { 'Content-Type': 'application/json', 'X-Specular-Secret': env().secret }
}

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`http://127.0.0.1:${env().port}${path}`, { headers: headers() })
  if (!res.ok) throw new Error(`GET ${path} → ${res.status}: ${await res.text()}`)
  return res.json() as Promise<T>
}

async function post<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`http://127.0.0.1:${env().port}${path}`, {
    method: 'POST',
    headers: headers(),
    body: JSON.stringify(body),
  })
  if (!res.ok) throw new Error(`POST ${path} → ${res.status}: ${await res.text()}`)
  return res.json() as Promise<T>
}

describe('app boot', () => {
  it('boots and responds to health check', async () => {
    const res = await fetch(`http://127.0.0.1:${env().port}/health`)
    expect(res.ok).toBe(true)
  })

  it('serves a valid workspace document', async () => {
    const doc = await get<JsonCanvasDocument>('/canvas')
    expect(Array.isArray(doc.nodes)).toBe(true)
    expect(Array.isArray(doc.edges)).toBe(true)
  })

  it('round-trips one mutation through the production door', async () => {
    const applied = await post<{ created: string[] }>('/canvas/apply', {
      entities: [{ kind: 'text', text: 'boot check', x: 40, y: 40 }],
    })
    expect(applied.created).toHaveLength(1)

    const doc = await get<JsonCanvasDocument>('/canvas')
    expect(doc.nodes.some((n) => n.id === applied.created[0])).toBe(true)

    const cleanup = await post<{ deleted: string[] }>('/canvas/apply', {
      delete: [applied.created[0]],
    })
    expect(cleanup.deleted).toEqual([applied.created[0]])
  })
})

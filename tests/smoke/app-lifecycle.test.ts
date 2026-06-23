import { describe, it, expect } from 'vitest'
import { getHealth, getWorkspace } from './app-client'

describe('app lifecycle', () => {
  it('responds to health check', async () => {
    const health = await getHealth()
    expect(health.version).toBe('1')
  })

  it('returns a valid workspace graph', async () => {
    // The read shape is GET /canvas now (ADR 0019 slice 4); the graph is the
    // serialized entities + edges. Camera/selection live on their own routes.
    const workspace = await getWorkspace()
    expect(workspace).toHaveProperty('entities')
    expect(workspace).toHaveProperty('edges')
    expect(Array.isArray(workspace.entities)).toBe(true)
    expect(Array.isArray(workspace.edges)).toBe(true)
  })
})

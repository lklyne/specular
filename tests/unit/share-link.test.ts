import { describe, expect, it } from 'vitest'
import { buildShareLink, parseShareLink } from '../../src/main/sync-client/share-link'

describe('buildShareLink', () => {
  it('builds the canonical `<base>/c/<docId>#t=<token>` form', () => {
    expect(buildShareLink({ base: 'http://localhost:8787', docId: 'doc-1', token: 'tok-1' })).toBe(
      'http://localhost:8787/c/doc-1#t=tok-1',
    )
  })

  it('drops a trailing slash on the base so it is idempotent', () => {
    expect(buildShareLink({ base: 'http://localhost:8787/', docId: 'd', token: 't' })).toBe(
      buildShareLink({ base: 'http://localhost:8787', docId: 'd', token: 't' }),
    )
  })

  it('round-trips through parseShareLink', () => {
    const url = buildShareLink({ base: 'https://sync.example.com', docId: 'abc', token: 'xyz' })
    const parsed = parseShareLink(url)
    expect(parsed).toEqual({ base: 'https://sync.example.com', docId: 'abc', token: 'xyz' })
  })
})

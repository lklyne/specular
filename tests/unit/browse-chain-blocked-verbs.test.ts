import { describe, it, expect } from 'vitest'
import { handleBrowse } from '../../src/main/shared/browse-handler'

// Mutation-verified: narrowed handleBrowse's block loop to
// `chainedParts[0]` only (checking just the first step) and confirmed the
// second test below stopped throwing — `close` reached the rest of
// handleBrowse instead of being refused. Restored afterward.
//
// BLOCKED_BROWSE_VERBS exists so a chain can't reach browser-lifecycle verbs
// Specular owns (`close`, `launch`, …) through the `browse` door — see
// handleBrowse's block loop and its comment. It must check every step of a
// chain, not only the first: `snapshot -i && close` would otherwise close
// the page out from under the CLI/MCP `browse` tool by hiding the blocked
// verb behind a harmless first command.
describe('handleBrowse — blocked verbs inside a chain', () => {
  it('refuses a chain whose first step is blocked', async () => {
    await expect(
      handleBrowse({ page_id: 'page-1', command: 'close && get text' }),
    ).rejects.toThrow(/close: blocked/)
  })

  it('refuses a chain whose blocked verb is a later step, not the first', async () => {
    await expect(
      handleBrowse({ page_id: 'page-1', command: 'snapshot -i && close' }),
    ).rejects.toThrow(/close: blocked/)
  })

  it('refuses a chain whose blocked verb is the last of three steps', async () => {
    await expect(
      handleBrowse({ page_id: 'page-1', command: 'snapshot -i && get text && launch' }),
    ).rejects.toThrow(/launch: blocked/)
  })
})

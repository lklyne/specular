import { describe, it, expect } from 'vitest'
import { spawnAsync } from '../../src/main/shared/browse-handler'

// Regression guard for the chained-command (batch) path: agent-browser exits
// non-zero the moment a --bail command fails but still writes its per-command
// JSON to stdout. browse-handler reads that stdout via allowNonZeroExit; if
// spawnAsync went back to rejecting on any non-zero exit, every chained browse
// call containing a failure (stale ref, missing element) would surface a raw
// process error instead of the formatted per-command output + stale-ref hints.
describe('spawnAsync exit-code handling', () => {
  it('resolves with stdout on a non-zero exit when allowNonZeroExit is set', async () => {
    const { stdout } = await spawnAsync('sh', ['-c', 'printf batch-json; exit 1'], {
      timeout: 5000,
      allowNonZeroExit: true,
    })
    expect(stdout).toBe('batch-json')
  })

  it('rejects on a non-zero exit by default', async () => {
    await expect(
      spawnAsync('sh', ['-c', 'printf oops 1>&2; exit 1'], { timeout: 5000 }),
    ).rejects.toThrow('oops')
  })
})

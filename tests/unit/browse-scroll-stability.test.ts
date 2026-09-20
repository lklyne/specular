import { describe, it, expect } from 'vitest'
import { buildScrollSettleExpression, decideScrollStability } from '../../src/main/shared/browse-handler'

// A page with `scroll-behavior: smooth` (or scroll-snap / an animated
// carousel) is still moving when `scrollintoview`'s own promise resolves —
// it reports the scroll as *requested*, not settled. This is the pure
// decision behind the poll loop that closes that gap before a mutation
// dispatches against the target's coordinates (see
// scrollTargetIntoViewAndSettle in shared/browse-handler.ts).
const CAP_MS = 600

describe('decideScrollStability', () => {
  it('gives up immediately when the target has no rect — nothing to stabilize', () => {
    expect(decideScrollStability({ x: 10, y: 10 }, null, 0, CAP_MS)).toBe('give-up')
    expect(decideScrollStability(null, null, 0, CAP_MS)).toBe('give-up')
  })

  it('keeps waiting on the very first read — one read alone can\'t prove stability', () => {
    expect(decideScrollStability(null, { x: 100, y: 50 }, 0, CAP_MS)).toBe('keep-waiting')
  })

  it('settles when two consecutive reads are within 1px — the already-in-view common case', () => {
    expect(decideScrollStability({ x: 100, y: 50 }, { x: 100, y: 50 }, 5, CAP_MS)).toBe('settled')
    expect(decideScrollStability({ x: 100.4, y: 50 }, { x: 100.9, y: 50 }, 5, CAP_MS)).toBe('settled')
  })

  it('keeps waiting while the target is still visibly moving and under the cap', () => {
    expect(decideScrollStability({ x: 100, y: 50 }, { x: 250, y: 50 }, 75, CAP_MS)).toBe('keep-waiting')
  })

  it('gives up once the cap elapses, even if the target is still moving', () => {
    expect(decideScrollStability({ x: 100, y: 50 }, { x: 250, y: 50 }, CAP_MS, CAP_MS)).toBe('give-up')
    expect(decideScrollStability({ x: 100, y: 50 }, { x: 250, y: 50 }, CAP_MS + 50, CAP_MS)).toBe('give-up')
  })

  it('settling wins over the cap on the same read', () => {
    expect(decideScrollStability({ x: 100, y: 50 }, { x: 100, y: 50 }, CAP_MS, CAP_MS)).toBe('settled')
  })
})

// The chained path can't poll from outside the batch, so its settle wait is a
// page-side expression agent-browser polls until truthy. Evaluated here against
// stub globals, since what matters is when it turns truthy.
describe('buildScrollSettleExpression', () => {
  function fakePage() {
    let now = 0
    let onScroll: (() => void) | null = null
    const window: Record<string, unknown> = {}
    const document = { addEventListener: (_type: string, listener: () => void) => { onScroll = listener } }
    const performance = { now: () => now }
    return {
      poll: (expression: string) =>
        new Function('window', 'document', 'performance', `return ${expression}`)(window, document, performance) as boolean,
      scroll: () => onScroll?.(),
      advance: (ms: number) => { now += ms },
    }
  }

  it('stays falsy while scroll events keep arriving and turns truthy once they stop', () => {
    const page = fakePage()
    const wait = buildScrollSettleExpression('batch:0')
    expect(page.poll(wait)).toBe(false)
    for (let i = 0; i < 5; i++) {
      page.advance(100)
      page.scroll()
      expect(page.poll(wait)).toBe(false)
    }
    page.advance(160)
    expect(page.poll(wait)).toBe(true)
  })

  it('turns truthy at the cap, so a scroll that never stops cannot fail the batch', () => {
    const page = fakePage()
    const wait = buildScrollSettleExpression('batch:0')
    page.poll(wait)
    for (let elapsed = 0; elapsed < 1200; elapsed += 100) {
      page.advance(100)
      page.scroll()
    }
    expect(page.poll(wait)).toBe(true)
  })

  it('starts a fresh clock for a later wait on the same page', () => {
    const page = fakePage()
    page.poll(buildScrollSettleExpression('batch:0'))
    page.advance(5000)
    expect(page.poll(buildScrollSettleExpression('batch:0'))).toBe(true)
    expect(page.poll(buildScrollSettleExpression('batch:1'))).toBe(false)
  })
})

import { describe, expect, it } from 'vitest'
import { shouldRunOnReply } from '../../src/main/agent-fix/fix-orchestrator'
import type { Annotation } from '../../src/shared/types'

function annotation(over: Partial<Annotation> = {}): Annotation {
  return {
    id: 'a1',
    anchor: { type: 'canvas', canvasX: 0, canvasY: 0 } as Annotation['anchor'],
    author: 'user',
    text: 'make this red',
    status: 'pending',
    replies: [],
    createdAt: new Date().toISOString(),
    pageAnchor: { pageId: 'p1', pageUrl: 'http://localhost:4321/' },
    ...over,
  }
}

const bindingOn = () => ({ autoFix: true })
const bindingOff = () => ({ autoFix: false })
const noBinding = () => null

describe('shouldRunOnReply', () => {
  it('a thread with a fix session always continues, auto-fix or not', () => {
    const withSession = annotation({ metadata: { fixSessionId: 'sess_1' } })
    expect(shouldRunOnReply(withSession, bindingOff)).toBe(true)
    expect(shouldRunOnReply(withSession, noBinding)).toBe(true)
  })

  it('a fresh thread runs only when the origin binding opted into auto-fix', () => {
    expect(shouldRunOnReply(annotation(), bindingOn)).toBe(true)
    expect(shouldRunOnReply(annotation(), bindingOff)).toBe(false)
    expect(shouldRunOnReply(annotation(), noBinding)).toBe(false)
  })

  it('a canvas-bound thread has no origin to opt in with', () => {
    expect(shouldRunOnReply(annotation({ pageAnchor: undefined }), bindingOn)).toBe(false)
  })

  it('a dismissed thread never fires, session or not', () => {
    expect(
      shouldRunOnReply(
        annotation({ status: 'dismissed', metadata: { fixSessionId: 'sess_1' } }),
        bindingOn,
      ),
    ).toBe(false)
  })
})

import { beforeEach, describe, expect, it, vi } from 'vitest'

const electron = vi.hoisted(() => ({
  startRecording: vi.fn(),
  stopRecording: vi.fn(),
}))

vi.mock('electron', () => ({
  app: { getPath: () => '/tmp' },
  contentTracing: electron,
  shell: { showItemInFolder: vi.fn() },
}))

vi.mock('../../src/main/debug-window', () => ({
  getDebugWebContents: () => null,
}))

describe('performance trace lifecycle', () => {
  beforeEach(() => {
    vi.resetModules()
    electron.startRecording.mockReset().mockResolvedValue(undefined)
    electron.stopRecording.mockReset()
  })

  it('stays in stopping state and cannot restart while Chromium flushes', async () => {
    let finishStop: ((path: string) => void) | undefined
    electron.stopRecording.mockImplementation(
      () => new Promise<string>((resolve) => {
        finishStop = resolve
      }),
    )
    const trace = await import('../../src/main/perf-trace')

    await trace.startPerfTrace({ revealOnAutoStop: false })
    const stopping = trace.stopPerfTrace({ reveal: false })

    expect(trace.getPerfTraceState().status).toBe('stopping')
    await trace.togglePerfTrace()
    expect(electron.startRecording).toHaveBeenCalledTimes(1)

    finishStop?.('/tmp/specular-trace-test.json')
    await stopping
    expect(trace.getPerfTraceState()).toMatchObject({
      recording: false,
      status: 'idle',
      startedAt: null,
    })
  })

  it('returns to idle when Chromium rejects the trace flush', async () => {
    electron.stopRecording.mockRejectedValue(new Error('flush failed'))
    const trace = await import('../../src/main/perf-trace')

    await trace.startPerfTrace({ revealOnAutoStop: false })
    await expect(trace.stopPerfTrace({ reveal: false })).rejects.toThrow('flush failed')

    expect(trace.getPerfTraceState().status).toBe('idle')
  })
})

import { afterEach, expect, it, vi } from 'vitest'
import { WebContentsView } from 'electron'
import * as pageHosts from '../../src/main/runtime/page-host'

const hosts: pageHosts.PageHost[] = []
afterEach(() => {
  hosts.splice(0).forEach((host) => host.destroy())
  pageHosts.setPageFrameTarget(null)
})

it('requests a fresh static-page frame after the canvas subscribes without changing culling or idle state', () => {
  const target = new WebContentsView().webContents
  pageHosts.setPageFrameTarget(target)
  const visible = pageHosts.createPageHost({ id: 'visible', width: 800, height: 600 })
  const culled = pageHosts.createPageHost({ id: 'culled', width: 800, height: 600 })
  const idle = pageHosts.createPageHost({ id: 'idle', width: 800, height: 600 })
  hosts.push(visible, culled, idle)
  culled.setPainting(false)
  idle.setIdle(true)
  const visiblePaint = vi.spyOn(visible.webContents, 'invalidate')
  const culledPaint = vi.spyOn(culled.webContents, 'invalidate')
  const idlePaint = vi.spyOn(idle.webContents, 'invalidate')

  // Initial paint was delivered before the renderer subscribed. Changing the
  // camera alone does not make a static document dirty; the surface asks again
  // once it has both a frame listener and this page in its draw list.
  pageHosts.requestPageFrames(target, ['visible', 'culled', 'idle', 'deleted'])
  expect(visiblePaint).toHaveBeenCalledTimes(1)
  expect(culledPaint).not.toHaveBeenCalled()
  expect(idlePaint).not.toHaveBeenCalled()
  expect(culled.painting).toBe(false)
  expect(idle.idle).toBe(true)

  // aboveView shares the preload, but only canvas-bg may request frames.
  pageHosts.requestPageFrames(new WebContentsView().webContents, ['visible'])
  expect(visiblePaint).toHaveBeenCalledTimes(1)
})

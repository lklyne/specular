import assert from 'node:assert/strict'
import { join } from 'node:path'
import { app, BrowserWindow, ipcMain } from 'electron'
import { ipcChannels } from '../../../src/shared/ipc-contract'
import { createPageHost, setPageFrameTarget, pageHostStats, requestPageFrames } from '../../../src/main/runtime/page-host'

app.setPath('userData', join(__dirname, 'user-data'))
const pause = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
const deadline = setTimeout(() => { console.error('GPU startup scenario timed out'); app.exit(3) }, 15_000)

app.whenReady().then(async () => {
  const surface = new BrowserWindow({
    show: false, width: 400, height: 300,
    webPreferences: { preload: join(__dirname, 'canvas-bg.js'), sandbox: false, backgroundThrottling: false },
  })
  await surface.loadFile(join(__dirname, 'index.html'))
  setPageFrameTarget(surface.webContents)
  const host = createPageHost({ id: 'static', width: 1000, height: 750 })
  let requests = 0
  ipcMain.on(ipcChannels.canvasRequestPageFrames, (event, ids) => {
    requests++
    requestPageFrames(event.sender, ids)
  })
  await host.webContents.loadURL('data:text/html,<style>html{background:rgb(0,200,40)}</style>')
  // Deliberately let the static document finish before React subscribes.
  await pause(1500)
  assert.ok(pageHostStats()[0].framesReceived > 0, 'fixture must paint before mount')
  assert.equal(pageHostStats()[0].outstandingTextures, 0, 'early texture must be released')

  const render = (visible: boolean, x = 0) => surface.webContents.executeJavaScript(`window.renderSurface(${visible}, ${x})`)
  const pixel = () => surface.webContents.executeJavaScript(
    "Array.from(document.querySelector('canvas').getContext('2d').getImageData(40,40,1,1).data)",
  )
  await render(true)
  await pause(800)
  assert.deepEqual(await pixel(), [0, 200, 40, 255], 'static page must appear after late mount')
  const startupRequests = requests
  await render(true, 1)
  await pause(200)
  assert.equal(requests, startupRequests, 'camera updates must not restart frame capture')
  await render(false)
  await pause(200)
  assert.deepEqual(await pixel(), [0, 0, 0, 0])
  await render(true)
  await pause(800)
  assert.deepEqual(await pixel(), [0, 200, 40, 255], 'page must reappear after its held frame was pruned')
  assert.equal(pageHostStats()[0].outstandingTextures, 1, 'the drawn frame is the one texture a static page holds')
  await render(false)
  await pause(200)
  assert.equal(pageHostStats()[0].outstandingTextures, 0, 'pruning the page must release its held frame')
  console.log('PASS: late mount at 20%, camera update, scene re-entry, and texture release')
  clearTimeout(deadline)
  host.destroy()
  surface.destroy()
  app.exit(0)
}).catch((error) => { console.error(error); app.exit(1) })

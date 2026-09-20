/**
 * In-process stand-in for the `electron` module, aliased in
 * vitest.integration.config.ts. Lets the real main-process runtime
 * (workspace model, Y.Doc, undo, persistence, entity mutators) run in plain
 * Node: `app.getPath('userData')` resolves to a per-run temp dir, views are
 * inert fakes, and every renderer-bound `webContents.send` is recorded so
 * tests can assert on broadcasts.
 *
 * The fakes are deliberately shallow. The layout engine stays dormant because
 * the fake shell window reports `isDestroyed() === true`, so nothing ever
 * needs real bounds, attachment, or a display. A page's offscreen host window
 * is the exception: `FakeBrowserWindow` reports itself alive so page
 * creation, resize, and teardown run. Unknown methods fall through to a
 * no-op via Proxy so incidental calls (setBorderRadius, focus, …) don't need
 * enumerating.
 */

import { EventEmitter } from 'events'
import Module from 'module'
import { join, resolve } from 'path'
import { tmpdir } from 'os'

let userDataPath = join(tmpdir(), 'specular-integration-userdata')

/** Harness hook: point app.getPath('userData') at a per-run temp dir. */
export function __setUserDataPath(path: string): void {
  userDataPath = path
}

export interface BroadcastRecord {
  channel: string
  args: unknown[]
  webContentsId: number
}

/** Every webContents.send() lands here; harness exposes and clears it. */
export const __broadcasts: BroadcastRecord[] = []

const noop = (): undefined => undefined

/**
 * Wrap a fake so any method not explicitly implemented resolves to a no-op
 * function instead of `undefined is not a function`. Non-function property
 * reads still return undefined (via the `fields` allowlist on each class).
 */
function withNoopFallback<T extends object>(target: T): T {
  return new Proxy(target, {
    get(obj, prop, receiver) {
      if (prop in obj) return Reflect.get(obj, prop, receiver)
      if (typeof prop === 'symbol') return undefined
      return noop
    },
  })
}

let nextWebContentsId = 1

class FakeWebContents extends EventEmitter {
  id = nextWebContentsId++
  ipc = new EventEmitter()

  // Every loadURL() call, in order — the navigation-sync suite counts these to
  // assert "exactly one navigation per peer" through the real relay machinery.
  loadedUrls: string[] = []

  // Every debugger.sendCommand() call, in order — the interaction-sync suite
  // reads Input.dispatchMouseEvent entries to assert a confident peer replay
  // actually dispatched trusted input at the resolved point.
  debuggerCommands: Array<{ method: string; params: unknown }> = []

  // Every setContentSize() on the offscreen window hosting these contents,
  // with how many debugger commands had been sent by then.
  contentSizes: Array<{ width: number; height: number; afterCommands: number }> = []

  // Every sendInputEvent() call, in order.
  inputEvents: Array<Record<string, unknown>> = []

  sendInputEvent(event: Record<string, unknown>): void {
    this.inputEvents.push(event)
  }

  send(channel: string, ...args: unknown[]): void {
    __broadcasts.push({ channel, args, webContentsId: this.id })
  }

  // Remembers the last loaded URL: the workspace snapshot reads it back via
  // getURL() (snapshotCurrentWorkspaceState in runtime/workspace-tabs.ts).
  private currentUrl = 'about:blank'

  loadURL(url?: string): Promise<void> {
    if (typeof url === 'string') {
      this.currentUrl = url
      this.loadedUrls.push(url)
    }
    return Promise.resolve()
  }

  getURL(): string {
    return this.currentUrl
  }

  getTitle(): string {
    return ''
  }

  isDestroyed(): boolean {
    return false
  }

  isCrashed(): boolean {
    return false
  }

  isLoading(): boolean {
    return false
  }

  executeJavaScript(): Promise<unknown> {
    return Promise.resolve(undefined)
  }

  // Offscreen-rendering levers the page host drives. `painting` is readable so
  // a test can assert the layout pass's painting policy.
  painting = true

  startPainting(): void {
    this.painting = true
  }

  stopPainting(): void {
    this.painting = false
  }

  isPainting(): boolean {
    return this.painting
  }

  invalidate(): void {
    // Nothing paints in-process; the real call asks for one frame.
  }

  /** Readable so a test can assert the idle policy's throttle. */
  frameRate = 60

  setFrameRate(fps: number): void {
    this.frameRate = fps
  }

  getFrameRate(): number {
    return this.frameRate
  }

  // Chained with .catch() by installScrollbarCss's dom-ready listener
  // (src/main/runtime/page-scrollbar-css.ts) — needs a real promise, not the
  // bare `undefined` the noop Proxy fallback would return.
  insertCSS(): Promise<void> {
    return Promise.resolve(undefined)
  }

  // A capture is as large as the view is: `captureFullResolution` polls for
  // a full-width one after resizing a shrunken page.
  capturePage(): Promise<{ toPNG(): Buffer; toDataURL(): string; getSize(): { width: number; height: number } }> {
    const size = this.contentSizes.at(-1) ?? { width: 0, height: 0 }
    return Promise.resolve({
      toPNG: () => Buffer.alloc(0),
      toDataURL: () => '',
      getSize: () => ({ width: size.width, height: size.height }),
    })
  }

  // `page-chrome-state.ts` samples back/forward availability whenever a
  // navigation settles. The noop fallback can't cover this one: it hands back a
  // bare function for the unknown `navigationHistory` property, and the call
  // sites reach through it for a method.
  navigationHistory = withNoopFallback({
    canGoBack: () => false,
    canGoForward: () => false,
  })

  session = withNoopFallback({})
  debugger = withNoopFallback({
    sendCommand: (method: string, params: unknown) => {
      this.debuggerCommands.push({ method, params })
      return Promise.resolve({})
    },
    isAttached: () => false,
  })

  constructor() {
    super()
    return withNoopFallback(this)
  }
}

class FakeWebContentsView {
  webContents = new FakeWebContents()

  getBounds() {
    return { x: 0, y: 0, width: 0, height: 0 }
  }

  constructor(_options?: unknown) {
    return withNoopFallback(this)
  }
}

/**
 * A page's offscreen host window. Separate from FakeBaseWindow because a page
 * host must report itself alive (the shell window reports destroyed so the
 * layout pass stays dormant).
 */
class FakeBrowserWindow extends EventEmitter {
  webContents = new FakeWebContents()

  private destroyed = false

  /**
   * Every setContentSize() call, in order, mirrored onto the webContents so a
   * test holding only `host.webContents` can assert the texture-scale policy
   * (and its ordering against `debuggerCommands`).
   */
  setContentSize(width: number, height: number): void {
    this.webContents.contentSizes.push({ width, height, afterCommands: this.webContents.debuggerCommands.length })
  }

  constructor() {
    super()
    return withNoopFallback(this)
  }

  isDestroyed(): boolean {
    return this.destroyed
  }

  destroy(): void {
    this.destroyed = true
    this.webContents.emit('destroyed')
  }
}

class FakeBaseWindow {
  contentView = withNoopFallback({
    children: [] as unknown[],
    getBounds: () => ({ x: 0, y: 0, width: 1440, height: 900 }),
  })

  // Reporting destroyed keeps layoutAllViews() (layout-engine.ts) an early
  // return, which is what makes the whole harness display-free.
  isDestroyed(): boolean {
    return true
  }

  getBounds() {
    return { x: 0, y: 0, width: 1440, height: 900 }
  }

  getContentBounds() {
    return { x: 0, y: 0, width: 1440, height: 900 }
  }

  constructor(_options?: unknown) {
    return withNoopFallback(this)
  }
}

export const WebContentsView = FakeWebContentsView
export const BaseWindow = FakeBaseWindow
export const BrowserWindow = FakeBrowserWindow

export const sharedTexture = withNoopFallback({
  importSharedTexture: () => withNoopFallback({ release: noop }),
  sendSharedTexture: () => Promise.resolve(),
  setSharedTextureReceiver: noop,
})

export const webContents = withNoopFallback({
  fromDevToolsTargetId: () => undefined,
  getAllWebContents: () => [] as unknown[],
})

export const app = withNoopFallback({
  getPath: (_name: string) => userDataPath,
  // Repo root, so code resolving bundled resources (e.g. the starter space)
  // reads the real files rather than a stub path.
  getAppPath: () => resolve(__dirname, '..', '..'),
  getName: () => 'Specular',
  getVersion: () => '0.0.0-test',
  isPackaged: false,
  whenReady: () => Promise.resolve(),
  requestSingleInstanceLock: () => true,
  getAppMetrics: () => [],
})

export const screen = withNoopFallback({
  getPrimaryDisplay: () => ({
    scaleFactor: 1,
    bounds: { x: 0, y: 0, width: 1440, height: 900 },
    workArea: { x: 0, y: 0, width: 1440, height: 900 },
    workAreaSize: { width: 1440, height: 900 },
  }),
  getCursorScreenPoint: () => ({ x: 0, y: 0 }),
})

export const nativeTheme = withNoopFallback({
  shouldUseDarkColors: false,
})

let clipboardText = ''
const clipboardBuffers = new Map<string, Buffer>()
export const clipboard = withNoopFallback({
  writeText: (text: string) => {
    clipboardText = text
    clipboardBuffers.clear()
  },
  readText: () => clipboardText,
  readImage: () => ({ isEmpty: () => true }),
  writeBuffer: (format: string, buffer: Buffer) => {
    clipboardText = ''
    clipboardBuffers.clear()
    clipboardBuffers.set(format, buffer)
  },
  readBuffer: (format: string) => clipboardBuffers.get(format) ?? Buffer.alloc(0),
  availableFormats: () => Array.from(clipboardBuffers.keys()),
  clear: () => {
    clipboardText = ''
    clipboardBuffers.clear()
  },
})

export const ipcMain = withNoopFallback(new EventEmitter())
export const shell = withNoopFallback({})
export const dialog = withNoopFallback({})
export const session = withNoopFallback({ defaultSession: withNoopFallback({}) })
export const Menu = withNoopFallback({ buildFromTemplate: () => withNoopFallback({}), setApplicationMenu: noop })
export const nativeImage = withNoopFallback({
  createFromBuffer: (buffer: Buffer) => withNoopFallback({
    isEmpty: () => buffer.length === 0,
    toPNG: () => buffer,
    getSize: () => ({ width: 1, height: 1 }),
  }),
  createEmpty: () => withNoopFallback({ isEmpty: () => true }),
})
export const crashReporter = withNoopFallback({})
export const protocol = withNoopFallback({})
export const net = withNoopFallback({})

const stubModule = withNoopFallback({
  app,
  screen,
  nativeTheme,
  clipboard,
  ipcMain,
  shell,
  dialog,
  session,
  Menu,
  nativeImage,
  WebContentsView,
  BaseWindow,
  BrowserWindow,
  sharedTexture,
  webContents,
})

// The vite alias only rewrites ESM imports; bare CJS `require('electron')`
// calls resolve through Node's loader to the real
// package, which throws in headless Node. Route them to the stub too.
const nodeModuleLoad = (Module as unknown as { _load: (...args: unknown[]) => unknown })._load
;(Module as unknown as { _load: (...args: unknown[]) => unknown })._load = function (
  request: unknown,
  ...rest: unknown[]
) {
  if (request === 'electron') return stubModule
  return nodeModuleLoad.call(this, request, ...rest)
}

export default stubModule

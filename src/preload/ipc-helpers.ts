import { ipcRenderer } from 'electron'
import type {
  IpcChannel,
  IpcPayload,
  RendererToMainChannel,
} from '../shared/ipc-contract'

/**
 * Subscribe to an IPC channel, forwarding the payload (event arg stripped) to
 * `callback`. Returns an unsubscribe fn. Collapses the copy-pasted
 * on/removeListener closure every preload bridge otherwise repeats per channel.
 *
 * When the channel is in the contract the payload type is inferred from it with
 * no explicit type argument; unmigrated channels keep the `on<T>(channel)` form.
 */
export function on<C extends IpcChannel>(
  channel: C,
): (callback: (payload: IpcPayload<C>) => void) => () => void
export function on<T = void>(
  channel: string,
): (callback: (payload: T) => void) => () => void
export function on(channel: string) {
  return (callback: (payload: unknown) => void) => {
    const handler = (_event: Electron.IpcRendererEvent, payload: unknown) =>
      callback(payload)
    ipcRenderer.on(channel, handler)
    return () => ipcRenderer.removeListener(channel, handler)
  }
}

/** State broadcasts can precede React mount; retain the latest at the bridge. */
export function onLatest<T>(channel: string): (callback: (payload: T) => void) => () => void {
  const listeners = new Set<(payload: T) => void>()
  let latest: { value: T } | undefined
  ipcRenderer.on(channel, (_event, value: T) => {
    latest = { value }
    for (const listener of listeners) listener(value)
  })
  return (callback) => {
    listeners.add(callback)
    if (latest) callback(latest.value)
    return () => { listeners.delete(callback) }
  }
}

/**
 * Send a payload up to main on a contract renderer→main channel. Keyed by the
 * contract, so the channel name and its payload type are checked together.
 */
export function send<C extends RendererToMainChannel>(
  channel: C,
  payload: IpcPayload<C>,
): void {
  ipcRenderer.send(channel, payload)
}

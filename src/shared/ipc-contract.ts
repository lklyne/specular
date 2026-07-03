import type { LayoutUpdateData, ThemeData } from './types'

/**
 * The single source of truth for every IPC channel: its payload type and which
 * way it travels. Preload helpers (`on` / `send`) and main-side senders key off
 * this map, so a channel rename or payload drift becomes a compile error rather
 * than a runtime silence.
 *
 * Payload types stay in `./types`; this file only owns the
 * channel↔payload↔direction wiring. A channel's string literal lives here and
 * nowhere else in the codebase.
 */
export interface IpcContract {
  'theme-changed': { dir: 'main→renderer'; payload: ThemeData }
  'layout-update': { dir: 'main→renderer'; payload: LayoutUpdateData }
}

export type IpcChannel = keyof IpcContract

export type IpcPayload<C extends IpcChannel> = IpcContract[C]['payload']

/** Channels a renderer sends up to main (`ipcRenderer.send`). */
export type RendererToMainChannel = {
  [C in IpcChannel]: IpcContract[C]['dir'] extends 'renderer→main' ? C : never
}[IpcChannel]

/**
 * Channel-name constants so senders reference the contract instead of restating
 * a raw literal. The `satisfies` clause keeps every value a declared channel.
 */
export const ipcChannels = {
  themeChanged: 'theme-changed',
  layoutUpdate: 'layout-update',
} as const satisfies Record<string, IpcChannel>

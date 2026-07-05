import { pages } from '../runtime/page-runtime'
import { ipcChannels } from '../../shared/ipc-contract'

export type ComponentPropOverridePayload = {
  pageId: string
  componentId: string
  propPath: string[]
  value: unknown
}

export type ComponentTokenOverridePayload = {
  pageId: string
  componentId?: string
  token: string
  value: string
  selector?: string
}

export function forwardOverrideToPage(
  pageId: string,
  channel: typeof ipcChannels.overrideProps | typeof ipcChannels.overrideToken,
  payload: Record<string, unknown>,
): void {
  const page = pages.find((candidate) => candidate.id === pageId)
  if (!page || page.pageView.webContents.isDestroyed()) return
  page.pageView.webContents.send(channel, payload)
}

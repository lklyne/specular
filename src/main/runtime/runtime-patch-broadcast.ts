import { ipcChannels } from '../../shared/ipc-contract'
import type { RuntimePatch } from '../../shared/runtime-patch'
import { aboveView } from './view-refs'
import { safeSend } from './safe-send'

/**
 * Push one changed runtime slice straight to the canvas overlay, bypassing the
 * debounced layout pass. A pass rebuilds the whole scene and re-serializes it
 * for every consumer, so its cost is set by scene size rather than by what
 * moved; a patch costs what it carries.
 *
 * aboveView is the only target today because it owns every piece of chrome a
 * patch currently describes. `layoutUpdate` still carries the same values, so a
 * renderer that missed a patch heals on the next full snapshot instead of
 * holding stale state.
 */
export function broadcastRuntimePatch(patch: RuntimePatch): void {
  if (!aboveView) return
  safeSend(aboveView.webContents, ipcChannels.runtimePatch, patch)
}

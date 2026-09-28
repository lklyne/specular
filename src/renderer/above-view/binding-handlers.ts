import type { BindingId } from '../../shared/bindings'

export function buildAboveViewHandlers(
  closeThread: () => void,
): Partial<Record<BindingId, () => void>> {
  return {
    'annotation-close-thread': closeThread,
  }
}

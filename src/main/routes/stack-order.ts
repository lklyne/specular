import type { Route } from './types'
import {
  currentEntityIds,
  currentEntityOrder,
  reorderStackOrderIds,
  type StackOrderAction,
} from '../runtime/entity-order-state'
import { writeJson } from './http-helpers'

type StackOrderRoute = {
  path: string
  action: StackOrderAction
}

const stackOrderRoutes: StackOrderRoute[] = [
  { path: '/stack-order/bring-forward', action: 'bring-forward' },
  { path: '/stack-order/send-backward', action: 'send-backward' },
  { path: '/stack-order/bring-to-front', action: 'bring-to-front' },
  { path: '/stack-order/send-to-back', action: 'send-to-back' },
]

function parseIds(body: unknown): string[] | null {
  const payload = body as { id?: unknown; ids?: unknown }
  if (typeof payload.id === 'string') return [payload.id]
  if (Array.isArray(payload.ids) && payload.ids.every((id) => typeof id === 'string')) {
    return payload.ids
  }
  return null
}

export const stackOrderHttpRoutes: Route[] = stackOrderRoutes.map(({ path, action }) => ({
  method: 'POST',
  pattern: path,
  async handler({ response, body }) {
    const ids = parseIds(body)
    if (!ids?.length) {
      writeJson(response, 400, { error: 'id or ids is required' })
      return
    }

    const currentIds = currentEntityIds()
    const unknownIds = ids.filter((id) => !currentIds.has(id))
    if (unknownIds.length) {
      writeJson(response, 404, { error: 'Unknown stack-order id', unknownIds })
      return
    }

    const ok = reorderStackOrderIds(action, ids)
    writeJson(response, 200, { ok, entityOrder: currentEntityOrder() })
  },
}))

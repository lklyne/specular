/**
 * Structural sharing between successive builds of the same value.
 *
 * The canvas scene is rebuilt whole on every layout pass and re-materialized
 * whole on every IPC delivery, so each consumer receives fresh identity for
 * branches that did not change. Identity is exactly what `React.memo`,
 * `useMemo`, and dirty checks compare, which makes a one-field edit cost a
 * full re-render of the scene. Reconciling the new value against the previous
 * one restores identity wherever the two are deep-equal, so only the changed
 * branches — and the objects containing them — carry new identity forward.
 */

import type { LayoutUpdateData } from './types'

type PlainObject = Record<string, unknown>

const NO_VOLATILE_KEYS: ReadonlySet<string> = new Set<string>()

/**
 * `buildMs` times the pass that produced the payload, so it differs on every
 * build by construction. Comparing it would defeat sharing for a number no
 * consumer treats as scene content.
 */
const VOLATILE_LAYOUT_KEYS = ['buildMs']

function isPlainObject(value: unknown): value is PlainObject {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return false
  const proto = Object.getPrototypeOf(value) as unknown
  return proto === Object.prototype || proto === null
}

function idOf(value: unknown): string | null {
  if (!isPlainObject(value)) return null
  return typeof value.id === 'string' ? value.id : null
}

function share(previous: unknown, next: unknown, volatileKeys: ReadonlySet<string>): unknown {
  if (Object.is(previous, next)) return next
  if (Array.isArray(next)) return shareArray(previous, next)
  if (isPlainObject(next)) return shareObject(previous, next, volatileKeys)
  return next
}

function shareArray(previous: unknown, next: readonly unknown[]): unknown {
  if (!Array.isArray(previous)) return next
  // Match by id where the elements carry one, so inserting an entity doesn't
  // hand every sibling after it new identity.
  const byId = new Map<string, unknown>()
  for (const item of previous) {
    const id = idOf(item)
    if (id !== null) byId.set(id, item)
  }
  let unchanged = previous.length === next.length
  const shared = next.map((item, index) => {
    const id = idOf(item)
    const candidate = (id !== null ? byId.get(id) : undefined) ?? previous[index]
    const element = share(candidate, item, NO_VOLATILE_KEYS)
    if (!Object.is(element, previous[index])) unchanged = false
    return element
  })
  return unchanged ? previous : shared
}

function countCompared(keys: readonly string[], volatileKeys: ReadonlySet<string>): number {
  if (volatileKeys.size === 0) return keys.length
  let count = 0
  for (const key of keys) if (!volatileKeys.has(key)) count += 1
  return count
}

function shareObject(
  previous: unknown,
  next: PlainObject,
  volatileKeys: ReadonlySet<string>,
): unknown {
  if (!isPlainObject(previous)) return next
  const nextKeys = Object.keys(next)
  let unchanged = countCompared(nextKeys, volatileKeys) ===
    countCompared(Object.keys(previous), volatileKeys)
  const shared: PlainObject = {}
  for (const key of nextKeys) {
    if (volatileKeys.has(key)) {
      shared[key] = next[key]
      continue
    }
    if (!Object.prototype.hasOwnProperty.call(previous, key)) unchanged = false
    const value = share(previous[key], next[key], NO_VOLATILE_KEYS)
    if (!Object.is(value, previous[key])) unchanged = false
    shared[key] = value
  }
  return unchanged ? previous : shared
}

/**
 * Reconciles `next` against `previous`, returning `previous` itself when the
 * two describe the same value.
 *
 * `volatileKeys` are compared at the top level only, and only for their
 * presence: when everything else matches, `previous` is returned untouched and
 * its volatile values ride along. Callers that need a fresh reading stamp it
 * themselves after the reconcile.
 */
export function shareStructure<T>(
  previous: unknown,
  next: T,
  volatileKeys: readonly string[] = [],
): T {
  return share(previous, next, new Set(volatileKeys)) as T
}

export function shareLayoutData(
  previous: LayoutUpdateData | null,
  next: LayoutUpdateData,
): LayoutUpdateData {
  if (!previous) return next
  return shareStructure(previous, next, VOLATILE_LAYOUT_KEYS)
}

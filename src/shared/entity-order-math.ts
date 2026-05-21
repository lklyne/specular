export type MovePosition = 'before' | 'after'

export interface EntityOrderGroup {
  id: string
  parentGroupId?: string | null
  childIds: string[]
}

function uniqueExisting(order: readonly string[], ids: readonly string[]): string[] {
  const wanted = new Set(ids)
  const seen = new Set<string>()
  const result: string[] = []
  for (const id of order) {
    if (!wanted.has(id) || seen.has(id)) continue
    seen.add(id)
    result.push(id)
  }
  for (const id of ids) {
    if (seen.has(id)) continue
    seen.add(id)
    result.push(id)
  }
  return result
}

export function moveBefore(
  order: readonly string[],
  id: string,
  anchorId: string,
  position: MovePosition,
): string[] {
  if (id === anchorId) return [...order]
  const withoutId = order.filter((candidate) => candidate !== id)
  const anchorIndex = withoutId.indexOf(anchorId)
  if (anchorIndex === -1) return [...order]
  const insertIndex = position === 'before' ? anchorIndex : anchorIndex + 1
  return [
    ...withoutId.slice(0, insertIndex),
    id,
    ...withoutId.slice(insertIndex),
  ]
}

export function moveBlockBefore(
  order: readonly string[],
  ids: readonly string[],
  anchorId: string | null,
  position: MovePosition,
): string[] {
  const block = uniqueExisting(order, ids)
  if (!block.length) return [...order]
  if (anchorId && block.includes(anchorId)) return [...order]

  const blockIds = new Set(block)
  const withoutBlock = order.filter((candidate) => !blockIds.has(candidate))
  const insertIndex = anchorId
    ? withoutBlock.indexOf(anchorId) + (position === 'after' ? 1 : 0)
    : position === 'before'
      ? 0
      : withoutBlock.length
  if (insertIndex < 0) return [...order]

  return [
    ...withoutBlock.slice(0, insertIndex),
    ...block,
    ...withoutBlock.slice(insertIndex),
  ]
}

export function replaceSubsequence(
  order: readonly string[],
  predicate: (id: string) => boolean,
  nextSubsequence: readonly string[],
): string[] {
  const next = [...nextSubsequence]
  let index = 0
  return order.map((id) => {
    if (!predicate(id)) return id
    const replacement = next[index]
    index += 1
    return replacement ?? id
  })
}

export function appendAtTop(order: readonly string[], id: string): string[] {
  return [...order.filter((candidate) => candidate !== id), id]
}

export function bringToFront(order: readonly string[], ids: readonly string[]): string[] {
  const block = uniqueExisting(order, ids)
  if (!block.length) return [...order]
  const blockIds = new Set(block)
  const withoutBlock = order.filter((candidate) => !blockIds.has(candidate))
  return [...withoutBlock, ...block]
}

export function sendToBack(order: readonly string[], ids: readonly string[]): string[] {
  const block = uniqueExisting(order, ids)
  if (!block.length) return [...order]
  const blockIds = new Set(block)
  const withoutBlock = order.filter((candidate) => !blockIds.has(candidate))
  return [...block, ...withoutBlock]
}

export function moveForward(order: readonly string[], ids: readonly string[]): string[] {
  const block = uniqueExisting(order, ids)
  if (!block.length) return [...order]
  const blockIds = new Set(block)
  const lastBlockIndex = Math.max(...block.map((id) => order.indexOf(id)))
  const anchorId = order
    .slice(lastBlockIndex + 1)
    .find((candidate) => !blockIds.has(candidate))
  if (!anchorId) return [...order]
  return moveBlockBefore(order, block, anchorId, 'after')
}

export function moveBackward(order: readonly string[], ids: readonly string[]): string[] {
  const block = uniqueExisting(order, ids)
  if (!block.length) return [...order]
  const blockIds = new Set(block)
  const firstBlockIndex = Math.min(...block.map((id) => order.indexOf(id)))
  const anchorId = order
    .slice(0, firstBlockIndex)
    .reverse()
    .find((candidate) => !blockIds.has(candidate))
  if (!anchorId) return [...order]
  return moveBlockBefore(order, block, anchorId, 'before')
}

export function enforceGroupContiguity(
  order: readonly string[],
  groups: readonly EntityOrderGroup[],
): string[] {
  const childrenByGroup = new Map<string, string[]>()
  for (const group of groups) childrenByGroup.set(group.id, [...group.childIds])

  function descendants(groupId: string): string[] {
    const ids: string[] = []
    for (const childId of childrenByGroup.get(groupId) ?? []) {
      ids.push(childId)
      if (childrenByGroup.has(childId)) ids.push(...descendants(childId))
    }
    return ids
  }

  let nextOrder = [...order]
  for (const group of groups) {
    const run = uniqueExisting(nextOrder, [...descendants(group.id), group.id])
    if (run.length <= 1) continue
    const runIds = new Set(run)
    const frontmostIndex = Math.max(...run.map((id) => nextOrder.indexOf(id)))
    const withoutRun = nextOrder.filter((id) => !runIds.has(id))
    const insertIndex = Math.min(frontmostIndex - run.length + 1, withoutRun.length)
    nextOrder = [
      ...withoutRun.slice(0, Math.max(0, insertIndex)),
      ...run.filter((id) => id !== group.id),
      group.id,
      ...withoutRun.slice(Math.max(0, insertIndex)),
    ]
  }
  return nextOrder
}

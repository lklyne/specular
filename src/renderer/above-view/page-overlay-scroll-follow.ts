/**
 * Pure reconciliation math for ADR 0032's page-scroll follower contract.
 *
 * Layout positions already incorporate the authoritative scroll offset. The
 * overlay transform therefore carries only the portion of the latest live
 * offset that the current layout has not incorporated yet.
 */
export interface PageScrollOffset {
  pageId: string
  scrollX: number
  scrollY: number
}

export function scrollFollowTransform(
  live: PageScrollOffset | null,
  incorporated: PageScrollOffset,
  scale: { x: number; y: number },
): string {
  if (!live || live.pageId !== incorporated.pageId) return ''
  const dx = (live.scrollX - incorporated.scrollX) * scale.x
  const dy = (live.scrollY - incorporated.scrollY) * scale.y
  if (dx === 0 && dy === 0) return ''
  return `translate(${-dx}px, ${-dy}px)`
}

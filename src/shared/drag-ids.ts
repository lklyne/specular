export type DragId = string & { readonly __brand: 'DragId' }

let counter = 0

export function newDragId(): DragId {
  counter = (counter + 1) >>> 0
  const rand = Math.random().toString(36).slice(2, 8)
  return `drag_${Date.now().toString(36)}_${counter.toString(36)}_${rand}` as DragId
}


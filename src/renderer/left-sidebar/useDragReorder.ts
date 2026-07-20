import { useCallback, useRef, useState } from 'react'

export const SIDEBAR_ITEM_DRAG_TYPE = 'application/x-specular-sidebar-item'

interface DragReorderState {
  draggedId: string | null
  dropTargetIndex: number | null
}

interface DragReorderActions {
  containerProps: {
    onDragOver: (e: React.DragEvent<HTMLElement>) => void
    onDragLeave: (e: React.DragEvent<HTMLElement>) => void
    onDrop: (e: React.DragEvent<HTMLElement>) => void
  }
  itemProps: (id: string, index: number, disabled?: boolean) => {
    draggable: boolean
    onDragStart: (e: React.DragEvent<HTMLElement>) => void
    onDragEnd: () => void
    className: string
    style: React.CSSProperties
  }
}

/**
 * Hook for drag-and-drop reordering of a list.
 * `onReorder` receives `(id, toArrayIndex)` — the visual-to-array
 * index conversion is handled internally.
 */
export function useDragReorder(
  itemCount: number,
  onReorder: (id: string, toIndex: number) => void,
): DragReorderState & DragReorderActions {
  const [draggedId, setDraggedId] = useState<string | null>(null)
  const [dropTargetIndex, setDropTargetIndex] = useState<number | null>(null)
  const dropTargetRef = useRef<number | null>(null)
  const draggedIndexRef = useRef<number>(-1)

  const containerProps = {
    onDragOver(e: React.DragEvent<HTMLElement>) {
      if (!draggedId) return
      e.preventDefault()
      e.dataTransfer.dropEffect = 'move'
      const children = Array.from(e.currentTarget.children) as HTMLElement[]
      let targetIdx = itemCount
      for (let i = 0; i < children.length; i++) {
        const rect = children[i].getBoundingClientRect()
        if (e.clientY < rect.top + rect.height / 2) {
          targetIdx = i
          break
        }
      }
      if (targetIdx !== dropTargetRef.current) {
        dropTargetRef.current = targetIdx
        setDropTargetIndex(targetIdx)
      }
    },
    onDragLeave(e: React.DragEvent<HTMLElement>) {
      if (!e.currentTarget.contains(e.relatedTarget as Node)) {
        dropTargetRef.current = null
        setDropTargetIndex(null)
      }
    },
    onDrop(e: React.DragEvent<HTMLElement>) {
      e.preventDefault()
      const id = draggedId
      const visualTarget = dropTargetRef.current
      if (!id || visualTarget === null) return
      const fromIndex = draggedIndexRef.current
      const toIndex = visualTarget > fromIndex ? visualTarget - 1 : visualTarget
      if (fromIndex !== toIndex) onReorder(id, toIndex)
      setDraggedId(null)
      dropTargetRef.current = null
      setDropTargetIndex(null)
    },
  }

  const itemProps = useCallback(
    (id: string, index: number, disabled = false) => {
      const isDragged = draggedId === id
      const showTopIndicator = dropTargetIndex === index && !isDragged
      const showBottomIndicator =
        dropTargetIndex === itemCount && index === itemCount - 1 && !isDragged

      return {
        draggable: !disabled,
        onDragStart: (e: React.DragEvent<HTMLElement>) => {
          setDraggedId(id)
          draggedIndexRef.current = index
          dropTargetRef.current = null
          setDropTargetIndex(null)
          e.dataTransfer.effectAllowed = 'move'
          e.dataTransfer.setData(SIDEBAR_ITEM_DRAG_TYPE, id)
        },
        onDragEnd: () => {
          setDraggedId(null)
          draggedIndexRef.current = -1
          dropTargetRef.current = null
          setDropTargetIndex(null)
        },
        className: isDragged ? 'opacity-40' : '',
        // Drop indicators use inset box-shadow, not border: box-shadow doesn't
        // occupy layout space, so the indicator can't nudge neighboring rows.
        style: {
          boxShadow: showTopIndicator
            ? 'inset 0 2px 0 0 var(--surface-toolbar-border)'
            : showBottomIndicator
              ? 'inset 0 -2px 0 0 var(--surface-toolbar-border)'
              : 'none',
        } as React.CSSProperties,
      }
    },
    [draggedId, dropTargetIndex, itemCount],
  )

  return { draggedId, dropTargetIndex, containerProps, itemProps }
}

import { useEffect, useRef, type RefObject } from 'react'
import { focusAndSelectAll } from '../../../shared/editor-selection'

export function useInlineEditFocus(
  inputRef: RefObject<HTMLInputElement | null>,
  isEditing: boolean,
  value: string,
  setDraft: (value: string) => void,
  onRequestFocus?: () => void,
) {
  const valueRef = useRef(value)
  const requestFocusRef = useRef(onRequestFocus)
  valueRef.current = value
  requestFocusRef.current = onRequestFocus

  useEffect(() => {
    if (!isEditing) return

    const focusInput = () => {
      if (inputRef.current) focusAndSelectAll(inputRef.current)
    }

    // Electron focuses the host WebContentsView separately from the DOM
    // element. Re-assert the editor focus once that outer handoff lands.
    window.addEventListener('focus', focusInput)
    // A renderer-local input.focus() can update activeElement without
    // reclaiming keyboard ownership for this WebContentsView. Give hosts
    // with an external focus arbiter a chance to request that handoff first.
    requestFocusRef.current?.()
    setDraft(valueRef.current)
    focusInput()
    // The controlled value update above can reset the native selection after
    // this effect. Re-apply it after React has committed that update.
    const frame = requestAnimationFrame(focusInput)

    return () => {
      cancelAnimationFrame(frame)
      window.removeEventListener('focus', focusInput)
    }
  }, [inputRef, isEditing, setDraft])
}

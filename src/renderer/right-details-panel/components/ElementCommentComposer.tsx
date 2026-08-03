import type { RefObject } from 'react'
import { CommentInput } from '../../shared/CommentPrimitives'

export function ElementCommentComposer({
  active,
  commentInputRef,
  elementCommentText,
  hasElementComment,
  onChange,
  onSubmit,
}: {
  active: boolean
  commentInputRef: RefObject<HTMLTextAreaElement | null>
  elementCommentText: string
  hasElementComment: boolean
  onChange: (value: string) => void
  onSubmit: () => void
}) {
  return (
    <div className="relative rounded-[8px] border border-[var(--surface-input-border)] bg-[var(--surface-input)] pl-3 pr-2 py-1.5">
      <CommentInput
        inputRef={commentInputRef}
        value={elementCommentText}
        onChange={onChange}
        onSubmit={onSubmit}
        placeholder={active ? 'Add a comment...' : 'Select an element to comment'}
        disabled={!active}
        buttonClassName="bg-[var(--surface-interactive)] text-[var(--surface-foreground-muted)] hover:bg-[var(--surface-interactive)] dark:hover:bg-[var(--surface-interactive)] dark:hover:text-[var(--surface-foreground)]"
      />
    </div>
  )
}

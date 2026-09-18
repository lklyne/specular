import type { RefObject, KeyboardEvent } from 'react'
import { PRIMARY_BUTTON_CLASS } from './primaryButton'

/**
 * Shared textarea + submit button for comment composers.
 * Callers wrap this in their own styled container div.
 */
export function CommentInput({
  inputRef,
  autoFocus,
  value,
  onChange,
  onSubmit,
  onKeyDown,
  placeholder = 'Add a comment...',
  disabled,
  submitLabel = 'Submit comment',
  buttonClassName,
  canSubmit,
}: {
  inputRef?: RefObject<HTMLTextAreaElement | null>
  autoFocus?: boolean
  value: string
  onChange: (value: string) => void
  onSubmit: () => void
  onKeyDown?: (event: KeyboardEvent<HTMLTextAreaElement>) => void
  placeholder?: string
  disabled?: boolean
  submitLabel?: string
  /** Override inactive button style. Active style is always blue. */
  buttonClassName?: string
  /** When set, controls the send button instead of non-empty text. */
  canSubmit?: boolean
}) {
  const hasContent = value.trim().length > 0
  const submitReady = canSubmit ?? hasContent
  const inactiveBtn = buttonClassName ?? 'bg-zinc-100 text-[var(--surface-foreground-muted)] hover:bg-zinc-200 dark:bg-zinc-700 dark:hover:bg-zinc-600 dark:hover:text-[var(--surface-foreground)]'

  return (
    <>
      <textarea
        ref={inputRef}
        autoFocus={autoFocus}
        className="block min-h-[24px] max-h-[120px] w-full resize-none overflow-y-auto bg-transparent py-0.5 pr-9 text-[14px] leading-6 text-[var(--surface-foreground)] outline-none [field-sizing:content] placeholder:text-[var(--surface-foreground-muted)]"
        rows={1}
        placeholder={placeholder}
        value={value}
        disabled={disabled}
        onChange={(event) => onChange(event.currentTarget.value)}
        onKeyDown={(event) => {
          if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault()
            onSubmit()
          }
          onKeyDown?.(event)
        }}
      />
      <button
        type="button"
        aria-label={submitLabel}
        className={`absolute bottom-1.5 right-1.5 flex h-7 w-7 items-center justify-center rounded-full text-[12px] transition disabled:opacity-40 ${
          submitReady
            ? PRIMARY_BUTTON_CLASS
            : inactiveBtn
        }`}
        disabled={disabled || !submitReady}
        onClick={onSubmit}
      >
        ↑
      </button>
    </>
  )
}

/**
 * Author label + message bubble for annotation threads.
 */
export function CommentBubble({
  author,
  text,
  fallback,
}: {
  author: string
  text?: string | null
  fallback?: string
}) {
  return (
    <div>
      <div className="text-xs font-medium text-[var(--surface-foreground)]">
        {author === 'agent' ? 'Agent' : 'You'}
      </div>
      {text ? (
        <div className="mt-1 inline-block max-w-full whitespace-pre-wrap rounded-2xl bg-zinc-100 px-3 py-1.5 text-[12px] text-[var(--surface-foreground)] dark:bg-zinc-700/60">
          {text}
        </div>
      ) : fallback ? (
        <div className="mt-1 text-[12px] italic text-[var(--surface-foreground-muted)]">
          {fallback}
        </div>
      ) : null}
    </div>
  )
}

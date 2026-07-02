import type { RefObject, KeyboardEvent } from 'react'

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
}) {
  const hasContent = value.trim().length > 0
  const inactiveBtn = buttonClassName ?? 'bg-zinc-100 text-zinc-500 hover:bg-zinc-200 hover:text-zinc-700 dark:bg-zinc-700 dark:text-zinc-300 dark:hover:bg-zinc-600 dark:hover:text-zinc-100'

  return (
    <>
      <textarea
        ref={inputRef}
        autoFocus={autoFocus}
        className="min-h-[24px] max-h-[120px] flex-1 resize-none overflow-y-auto bg-transparent py-0.5 text-[14px] leading-6 text-zinc-900 outline-none [field-sizing:content] placeholder:text-zinc-400 dark:text-zinc-100 dark:placeholder:text-zinc-500"
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
        className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-full text-[12px] transition disabled:opacity-40 ${
          hasContent
            ? 'bg-blue-500 text-white hover:bg-blue-600'
            : inactiveBtn
        }`}
        disabled={disabled || !hasContent}
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
      <div className="text-xs font-medium text-zinc-900 dark:text-zinc-100">
        {author === 'agent' ? 'Agent' : 'You'}
      </div>
      {text ? (
        <div className="mt-1 inline-block max-w-full whitespace-pre-wrap rounded-2xl bg-zinc-100 px-3 py-1.5 text-[12px] text-zinc-900 dark:bg-zinc-700/60 dark:text-zinc-100">
          {text}
        </div>
      ) : fallback ? (
        <div className="mt-1 text-[12px] italic text-zinc-500 dark:text-zinc-400">
          {fallback}
        </div>
      ) : null}
    </div>
  )
}

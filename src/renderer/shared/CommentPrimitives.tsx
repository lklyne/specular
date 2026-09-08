import type { RefObject, KeyboardEvent } from 'react'
import { Markdown } from './Markdown'
import { PRIMARY_BUTTON_CLASS } from './primaryButton'

/** Type ramp and reset shared by every comment/message textarea. */
const TEXTAREA_CLASS =
  'block w-full resize-none overflow-y-auto bg-transparent text-[14px] leading-6 text-[var(--surface-foreground)] outline-none [field-sizing:content] placeholder:text-[var(--surface-foreground-muted)]'

/** The message field. Enter sends, Shift+Enter opens a line. */
export function CommentTextarea({
  inputRef,
  autoFocus,
  value,
  onChange,
  onSubmit,
  onKeyDown,
  placeholder = 'Add a comment...',
  disabled,
  rows = 1,
  className = 'min-h-[24px] max-h-[120px] py-0.5 pr-9',
}: {
  inputRef?: RefObject<HTMLTextAreaElement | null>
  autoFocus?: boolean
  value: string
  onChange: (value: string) => void
  onSubmit: () => void
  onKeyDown?: (event: KeyboardEvent<HTMLTextAreaElement>) => void
  placeholder?: string
  disabled?: boolean
  rows?: number
  /** Sizing and padding — the field grows with its content between them. */
  className?: string
}) {
  return (
    <textarea
      ref={inputRef}
      autoFocus={autoFocus}
      className={`${TEXTAREA_CLASS} ${className}`}
      rows={rows}
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
  )
}

/** Round send button. Goes blue the moment there is something to send. */
export function CommentSendButton({
  onSubmit,
  disabled,
  submitReady,
  label = 'Submit comment',
  className = 'absolute bottom-1.5 right-1.5',
  inactiveClassName = 'bg-zinc-100 text-[var(--surface-foreground-muted)] hover:bg-zinc-200 dark:bg-zinc-700 dark:hover:bg-zinc-600 dark:hover:text-[var(--surface-foreground)]',
}: {
  onSubmit: () => void
  disabled?: boolean
  submitReady: boolean
  label?: string
  /** Placement — the button owns its own size and shape. */
  className?: string
  /** Look before there is anything to send. */
  inactiveClassName?: string
}) {
  return (
    <button
      type="button"
      aria-label={label}
      className={`flex h-7 w-7 items-center justify-center rounded-full text-[12px] transition disabled:opacity-40 ${className} ${
        submitReady ? PRIMARY_BUTTON_CLASS : inactiveClassName
      }`}
      disabled={disabled || !submitReady}
      onClick={onSubmit}
    >
      ↑
    </button>
  )
}

/**
 * Textarea with the send button floated in its bottom-right corner.
 * Callers wrap this in their own styled, relatively-positioned container.
 */
export function CommentInput({
  inputRef,
  autoFocus,
  value,
  onChange,
  onSubmit,
  onKeyDown,
  placeholder,
  disabled,
  submitLabel,
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
  return (
    <>
      <CommentTextarea
        inputRef={inputRef}
        autoFocus={autoFocus}
        value={value}
        onChange={onChange}
        onSubmit={onSubmit}
        onKeyDown={onKeyDown}
        placeholder={placeholder}
        disabled={disabled}
      />
      <CommentSendButton
        onSubmit={onSubmit}
        disabled={disabled}
        submitReady={canSubmit ?? value.trim().length > 0}
        label={submitLabel}
        inactiveClassName={buttonClassName}
      />
    </>
  )
}

/**
 * One message in a thread.
 *
 * Only the user gets a bubble. Their turns are short and interruptive, so the
 * pill reads as an aside; the agent's are long-form markdown that a bubble
 * would box in for no gain. The asymmetry is what tells the two apart, which
 * is why neither carries an author label.
 *
 * Agent text is markdown and renders as such; the user's is shown exactly as
 * typed. The composer offers no markdown affordance, so silently italicising
 * someone's `2 * 3 * 4` would be a surprise, not a feature.
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
  if (!text) {
    return fallback ? (
      <div className="text-[12px] italic text-[var(--surface-foreground-muted)]">{fallback}</div>
    ) : null
  }

  if (author === 'agent') {
    return (
      <div className="min-w-0 text-[12px] leading-relaxed text-[var(--surface-foreground)]">
        <Markdown text={text} />
      </div>
    )
  }

  // The input tokens, not a bubble-specific fill: a sent message should look
  // like the composer it came from. They also read against --surface-panel in
  // both themes (lighter in light, darker in dark), which a mid-zinc does not.
  return (
    <div className="min-w-0">
      <div className="inline-block max-w-full whitespace-pre-wrap rounded-2xl border border-[var(--surface-input-border)] bg-[var(--surface-input)] px-3 py-1.5 text-[12px] leading-relaxed text-[var(--surface-foreground)] [overflow-wrap:anywhere]">
        {text}
      </div>
    </div>
  )
}

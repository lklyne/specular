import { memo } from 'react'
import ReactMarkdown, { type Components } from 'react-markdown'
import remarkGfm from 'remark-gfm'

/**
 * Agent messages arrive as markdown. This renders it at the panel's type ramp,
 * with every element sized to the same 12px/relaxed rhythm as the surrounding
 * chrome rather than a generic prose scale.
 *
 * Everything that can be wider than the panel — code blocks, tables, long
 * URLs — is constrained here rather than at the call site: `min-w-0` on the
 * root plus `overflow-wrap: anywhere` breaks unbreakable words, and the two
 * genuinely wide block types scroll inside their own box. Nothing pushes the
 * panel's flex column wider.
 */
const COMPONENTS: Components = {
  p: ({ children }) => <p className="my-1.5">{children}</p>,
  h1: ({ children }) => <h2 className="mb-1 mt-3 text-[13px] font-semibold">{children}</h2>,
  h2: ({ children }) => <h3 className="mb-1 mt-3 text-[13px] font-semibold">{children}</h3>,
  h3: ({ children }) => <h4 className="mb-1 mt-2.5 font-semibold">{children}</h4>,
  h4: ({ children }) => <h5 className="mb-1 mt-2.5 font-semibold">{children}</h5>,
  h5: ({ children }) => <h6 className="mb-1 mt-2.5 font-semibold">{children}</h6>,
  h6: ({ children }) => <h6 className="mb-1 mt-2.5 font-semibold">{children}</h6>,
  ul: ({ children }) => <ul className="my-1.5 list-disc space-y-0.5 pl-4">{children}</ul>,
  ol: ({ children }) => <ol className="my-1.5 list-decimal space-y-0.5 pl-4">{children}</ol>,
  li: ({ children }) => <li className="pl-0.5 marker:text-[var(--surface-foreground-muted)] [&>p]:my-0">{children}</li>,
  strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
  em: ({ children }) => <em className="italic">{children}</em>,
  hr: () => <hr className="my-2.5 border-black/10 dark:border-white/15" />,
  blockquote: ({ children }) => (
    <blockquote className="my-1.5 border-l-2 border-black/15 pl-2 text-[var(--surface-foreground-muted)] dark:border-white/20 [&>:first-child]:mt-0 [&>:last-child]:mb-0">
      {children}
    </blockquote>
  ),
  a: ({ href, children }) => (
    <a
      href={href}
      target="_blank"
      rel="noreferrer noopener"
      className="text-blue-600 underline underline-offset-2 hover:text-blue-500 dark:text-blue-400"
    >
      {children}
    </a>
  ),
  // Fenced blocks arrive as a `code` inside a `pre`. `code` always paints the
  // inline pill; `pre` unpaints its own descendant so a block reads as one box.
  code: ({ children }) => (
    <code className="rounded bg-black/[0.07] px-1 py-px font-mono text-[11px] dark:bg-white/10">
      {children}
    </code>
  ),
  pre: ({ children }) => (
    <pre className="my-1.5 overflow-x-auto rounded-lg bg-black/[0.06] p-2 dark:bg-black/30 [&_code]:bg-transparent [&_code]:p-0">
      {children}
    </pre>
  ),
  table: ({ children }) => (
    <div className="my-1.5 overflow-x-auto">
      <table className="w-full border-collapse text-left">{children}</table>
    </div>
  ),
  th: ({ children }) => (
    <th className="border-b border-black/15 px-1.5 py-1 font-semibold dark:border-white/20">
      {children}
    </th>
  ),
  td: ({ children }) => (
    <td className="border-b border-black/[0.07] px-1.5 py-1 align-top dark:border-white/10">
      {children}
    </td>
  ),
}

export const Markdown = memo(function Markdown({ text }: { text: string }) {
  return (
    <div className="min-w-0 [overflow-wrap:anywhere] [&>:first-child]:mt-0 [&>:last-child]:mb-0">
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={COMPONENTS}>
        {text}
      </ReactMarkdown>
    </div>
  )
})

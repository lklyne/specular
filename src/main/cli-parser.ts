// ---------------------------------------------------------------------------
// CLI argument parser
// ---------------------------------------------------------------------------
// Splits an argv array into a verb, positional args, value flags, and boolean
// flags. Verb handlers in cli-commands.ts consume the resulting ParsedArgs.

export interface ParsedArgs {
  /** First positional token (the verb) */
  verb: string
  /** Remaining positional tokens after the verb */
  positional: string[]
  /** Named flags (--key value → flags.key = "value") */
  flags: Record<string, string>
  /** Boolean flags (--flag with no value) */
  boolFlags: Set<string>
  /** Raw argv after the verb */
  rest: string[]
}

export const CLI_VALUE_FLAGS = new Set([
  '--page', '-f',
  '--preset', '--presets', '--at', '--width', '--height', '--size', '--cols',
  '--anchor', '--label', '--color', '--layout', '--gap',
  '--status', '--url', '--page-id', '--ids',
  '--reason', '--text',
  '--output', '--fps', '--quality',
  '--min-idle', '--speed-factor',
  '--kind',
  // agent-browser passthrough flags
  '-s', '-d', '-p',
  '--selector', '--format', '--depth', '--wait', '--attr',
  '--timeout', '--load', '--viewport', '--device',
  '--baseline', '--screenshot-format', '--screenshot-quality', '--screenshot-dir',
  '--max-output', '--download-path',
  '--color-scheme', '--idle-timeout',
])

export function parseArgs(argv: string[]): ParsedArgs {
  const flags: Record<string, string> = {}
  const boolFlags = new Set<string>()
  const positional: string[] = []
  let i = 0
  while (i < argv.length) {
    const arg = argv[i]
    if (arg === '--') {
      positional.push(...argv.slice(i + 1))
      break
    }
    if (arg.startsWith('--') || (arg.startsWith('-') && arg.length === 2)) {
      const key = arg.replace(/^-+/, '')
      if (CLI_VALUE_FLAGS.has(arg) && i + 1 < argv.length) {
        flags[key] = argv[i + 1]
        i += 2
      } else {
        boolFlags.add(key)
        i++
      }
    } else {
      positional.push(arg)
      i++
    }
  }
  const verb = positional.shift() ?? ''
  return { verb, positional, flags, boolFlags, rest: argv.slice(1) }
}

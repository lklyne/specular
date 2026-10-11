/** JSON lines on stdin and stdout. */

/** Writes one value as one line of stdout. */
export function send(value: unknown): void {
  process.stdout.write(`${JSON.stringify(value)}\n`)
}

/** The values arriving on stdin, one per line. A line that is not JSON is skipped. */
export async function* received(): AsyncGenerator<unknown> {
  let rest = ''
  for await (const chunk of process.stdin) {
    rest += chunk.toString()
    let end = rest.indexOf('\n')
    while (end >= 0) {
      const line = rest.slice(0, end).trim()
      rest = rest.slice(end + 1)
      if (line) {
        try {
          yield JSON.parse(line)
        } catch {
          process.stderr.write('specular-helper: a line of stdin was not JSON\n')
        }
      }
      end = rest.indexOf('\n')
    }
  }
}

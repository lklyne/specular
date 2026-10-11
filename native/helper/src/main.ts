/**
 * The app's helper process: work that needs a JavaScript library, run beside
 * the Rust app and spoken to in JSON lines over stdin and stdout.
 *
 * The first argument names the service. Each service reads its own requests
 * from stdin and writes its own lines to stdout; stderr is for people.
 */

import { agent } from './agent'

const services: Record<string, () => Promise<void>> = { agent }

const name = process.argv[2] ?? ''
const service = services[name]
if (!service) {
  process.stderr.write(`specular-helper: no service "${name}" (${Object.keys(services).join(', ')})\n`)
  process.exit(2)
}
await service()
process.exit(0)

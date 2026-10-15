// Minimal CDP driver for the capture Chrome (port 9333).
//   node cdp.mjs load <url> <init-script.js>   install init script, navigate
//   node cdp.mjs eval <file.js|-e code>        evaluate an async expression body in the page, print JSON
import fs from 'node:fs'
const PORT = process.env.CDP_PORT || 9333
const targets = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json()
const page = targets.find((t) => t.type === 'page')
const ws = new WebSocket(page.webSocketDebuggerUrl, { perMessageDeflate: false })
await new Promise((r) => ws.addEventListener('open', r))
let id = 0
const pending = new Map()
ws.addEventListener('message', (ev) => {
  const m = JSON.parse(ev.data)
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id) }
  else if (m.method === 'Runtime.consoleAPICalled' && process.env.CONSOLE) console.error('[console]', m.params.args.map((a) => a.value ?? a.description).join(' '))
})
const send = (method, params = {}) => new Promise((res, rej) => {
  const i = ++id
  pending.set(i, (m) => (m.error ? rej(new Error(method + ': ' + JSON.stringify(m.error))) : res(m.result)))
  ws.send(JSON.stringify({ id: i, method, params }))
})
const [cmd, a1, a2] = process.argv.slice(2)
if (cmd === 'load') {
  await send('Page.enable')
  if (process.env.PRE) await send('Page.addScriptToEvaluateOnNewDocument', { source: process.env.PRE })
  if (a2) await send('Page.addScriptToEvaluateOnNewDocument', { source: fs.readFileSync(a2, 'utf8') })
  const loaded = new Promise((r) => ws.addEventListener('message', (ev) => { if (JSON.parse(ev.data).method === 'Page.loadEventFired') r() }))
  await send('Page.navigate', { url: a1 })
  await loaded
  console.log('loaded')
} else if (cmd === 'eval') {
  const code = a1 === '-e' ? a2 : fs.readFileSync(a1, 'utf8')
  const r = await send('Runtime.evaluate', { expression: `(async () => { ${code} })()`, awaitPromise: true, returnByValue: true, timeout: 600000 })
  if (r.exceptionDetails) { console.error(JSON.stringify(r.exceptionDetails, null, 1).slice(0, 3000)); process.exit(1) }
  console.log(typeof r.result.value === 'string' ? r.result.value : JSON.stringify(r.result.value, null, 1))
} else if (cmd === 'shot') {
  const r = await send('Page.captureScreenshot', { format: 'png' })
  fs.writeFileSync(a1, Buffer.from(r.data, 'base64'))
  console.log('wrote', a1)
} else if (cmd === 'shotcanvas') {
  // screenshot of the canvas element as Chrome composites it (page bg included), at dpr 1
  await send('Emulation.setDeviceMetricsOverride', { width: 1400, height: 1300, deviceScaleFactor: 1, mobile: false })
  await new Promise((r) => setTimeout(r, 800))
  const { result } = await send('Runtime.evaluate', { expression: 'JSON.stringify(document.querySelector("canvas").getBoundingClientRect())', returnByValue: true })
  const b = JSON.parse(result.value)
  const r = await send('Page.captureScreenshot', { format: 'png', clip: { x: b.x, y: b.y, width: b.width, height: b.height, scale: 1 } })
  fs.writeFileSync(a1, Buffer.from(r.data, 'base64'))
  await send('Emulation.clearDeviceMetricsOverride')
  console.log('wrote', a1, b.width, b.height)
} else if (cmd === 'viewport') {
  await send('Emulation.setDeviceMetricsOverride', { width: +a1, height: +a2, deviceScaleFactor: 1, mobile: false })
  console.log('ok')
}
ws.close()
process.exit(0)

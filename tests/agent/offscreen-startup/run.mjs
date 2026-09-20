// Run with: node tests/agent/offscreen-startup/run.mjs
// Needs a local Electron/GPU session; does not open or modify the user's space.
// Mutation check: remove CanvasItemSurface's requestPageFrames call, or the
// stopPainting/startPainting pair in page-host: the late-mount pixel stays clear.
import { build } from 'esbuild'
import { mkdtemp, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { spawnSync } from 'node:child_process'
import electron from 'electron'

const repo = fileURLToPath(new URL('../../../', import.meta.url))
const output = await mkdtemp(join(tmpdir(), 'specular-gpu-startup-'))
try {
  for (const [entry, name, platform] of [
    ['tests/agent/offscreen-startup/main.ts', 'main', 'node'],
    ['src/preload/canvas-bg.ts', 'canvas-bg', 'node'],
    ['tests/agent/offscreen-startup/renderer.tsx', 'renderer', 'browser'],
  ]) {
    await build({
      entryPoints: [resolve(repo, entry)], outfile: join(output, `${name}.js`),
      bundle: true, platform, jsx: 'automatic', external: platform === 'node' ? ['electron'] : [],
    })
  }
  await writeFile(join(output, 'page-content.js'), '')
  await writeFile(join(output, 'index.html'), '<html><head><style>html,body{margin:0}canvas{position:absolute;width:100%;height:100%}</style></head><body><div id="root"></div><script src="renderer.js"></script></body></html>')
  const result = spawnSync(electron, [join(output, 'main.js')], { stdio: 'inherit', timeout: 20_000 })
  process.exitCode = result.status ?? 1
  if (result.error) console.error(result.error)
} finally {
  await rm(output, { recursive: true, force: true })
}

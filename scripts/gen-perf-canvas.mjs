#!/usr/bin/env node
/**
 * Generates a `.canvas` fixture for the pan/zoom benchmark.
 *
 * The pages are local and pinned, not live websites. ADR 0038's spike compared
 * runs against Wikipedia, GitHub and MDN, which makes a number irreproducible
 * a week later — the content changed. These are served from
 * `tests/perf/fixtures/`, are seeded per page index, and advance their
 * animations by frame count rather than wall-clock, so two runs do the same
 * work.
 *
 * Usage:
 *   node scripts/gen-perf-canvas.mjs --pages 20 --out perf-20.canvas
 *   node scripts/gen-perf-canvas.mjs --pages 40 --mix static
 *
 * Serve the fixtures first (the default base URL points at this):
 *   python3 -m http.server 8931 --directory tests/perf/fixtures
 */

import { writeFileSync } from 'node:fs'
import { resolve } from 'node:path'

/** Fixture archetypes, in the order a mixed canvas cycles through them. */
const ARCHETYPES = {
  static: 'static-text.html',
  css: 'css-animation.html',
  raf: 'raf-canvas.html',
  webgl: 'webgl-shader.html',
  webgpu: 'webgpu-shader.html',
  video: 'video.html',
}

const DEFAULTS = {
  pages: 20,
  mix: Object.keys(ARCHETYPES).join(','),
  baseUrl: 'http://localhost:8931',
  width: 1280,
  height: 800,
  gap: 160,
  out: null,
}

function parseArgs(argv) {
  const options = { ...DEFAULTS }
  for (let i = 0; i < argv.length; i += 2) {
    const key = argv[i]?.replace(/^--/, '')
    const value = argv[i + 1]
    if (!key || value === undefined) continue
    if (key === 'pages' || key === 'width' || key === 'height' || key === 'gap') {
      options[key] = Number.parseInt(value, 10)
    } else if (key === 'base-url') {
      options.baseUrl = value
    } else if (key in options) {
      options[key] = value
    } else {
      throw new Error(`Unknown option --${key}`)
    }
  }
  if (!Number.isFinite(options.pages) || options.pages < 1) {
    throw new Error('--pages must be a positive integer')
  }
  const mix = options.mix.split(',').map((name) => name.trim()).filter(Boolean)
  const unknown = mix.filter((name) => !(name in ARCHETYPES))
  if (unknown.length > 0) {
    throw new Error(`Unknown archetype(s): ${unknown.join(', ')}. Known: ${Object.keys(ARCHETYPES).join(', ')}`)
  }
  if (mix.length === 0) throw new Error('--mix needs at least one archetype')
  return { ...options, mix }
}

/**
 * Lays pages out on a square-ish grid. Square rather than a single row so a
 * zoom-out sees pages in both axes — a row would leave most of the canvas
 * empty at low zoom and under-report what the compositor is doing.
 */
function gridPosition(index, total, width, height, gap) {
  const columns = Math.ceil(Math.sqrt(total))
  return {
    x: (index % columns) * (width + gap),
    y: Math.floor(index / columns) * (height + gap),
  }
}

function buildCanvas(options) {
  const { pages, mix, baseUrl, width, height, gap } = options
  const nodes = []
  for (let i = 0; i < pages; i++) {
    const archetype = mix[i % mix.length]
    const { x, y } = gridPosition(i, pages, width, height, gap)
    nodes.push({
      id: `perf-${archetype}-${i}`,
      type: 'link',
      x,
      y,
      width,
      height,
      // `?i=` seeds the page so instances differ, deterministically. Identical
      // URLs would also let Chromium share more between pages than a real
      // canvas ever would, flattering the result.
      url: `${baseUrl}/${ARCHETYPES[archetype]}?i=${i}`,
      label: `${archetype} #${i}`,
      source: 'perf-fixture',
    })
  }
  return { nodes, edges: [] }
}

function main() {
  const options = parseArgs(process.argv.slice(2))
  const canvas = buildCanvas(options)
  const out = options.out ?? `perf-${options.pages}.canvas`
  const path = resolve(process.cwd(), out)
  writeFileSync(path, `${JSON.stringify(canvas, null, 2)}\n`, 'utf8')

  const counts = new Map()
  for (const node of canvas.nodes) {
    const archetype = node.id.split('-')[1]
    counts.set(archetype, (counts.get(archetype) ?? 0) + 1)
  }
  const breakdown = [...counts.entries()].map(([name, n]) => `${n} ${name}`).join(', ')
  console.log(`Wrote ${path}`)
  console.log(`  ${canvas.nodes.length} pages — ${breakdown}`)
  console.log(`  base URL ${options.baseUrl}`)
  console.log('')
  console.log('Serve the fixtures before opening the canvas:')
  console.log('  python3 -m http.server 8931 --directory tests/perf/fixtures')
}

try {
  main()
} catch (error) {
  console.error(`gen-perf-canvas: ${error.message}`)
  process.exit(1)
}

/**
 * Shared helpers for the benchmark fixture pages.
 *
 * Every animated fixture advances by frame *count*, never by wall-clock. A
 * benchmark run has to do identical work on a fast and a slow machine; a page
 * driving its animation from `performance.now()` would render further along
 * its timeline when the app is slow, changing the workload being measured.
 */

/** Deterministic PRNG (mulberry32), so page N looks the same on every run. */
export function seededRandom(seed) {
  let state = seed >>> 0
  return function next() {
    state = (state + 0x6d2b79f5) >>> 0
    let t = state
    t = Math.imul(t ^ (t >>> 15), t | 1)
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296
  }
}

/** The `?i=` index this page instance was opened with. */
export function fixtureIndex() {
  const raw = new URLSearchParams(location.search).get('i')
  const parsed = Number.parseInt(raw ?? '0', 10)
  return Number.isFinite(parsed) ? parsed : 0
}

/**
 * Names what the page is actually doing, in the page and in its title. A
 * fixture that silently fell back (no WebGPU adapter, no video codec) would
 * otherwise be measured as if it were still under load.
 */
export function reportMode(kind, mode, detail = '') {
  document.title = `${kind} — ${mode}`
  const badge = document.querySelector('[data-mode]')
  if (badge) {
    badge.textContent = detail ? `${mode} · ${detail}` : mode
    badge.dataset.state = mode === 'unavailable' ? 'fallback' : 'ok'
  }
}

/** Runs `draw(frame)` every animation frame, passing a monotonic counter. */
export function driveByFrame(draw) {
  let frame = 0
  function tick() {
    draw(frame++)
    requestAnimationFrame(tick)
  }
  requestAnimationFrame(tick)
}

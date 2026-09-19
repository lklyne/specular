// @ts-nocheck — no three typings installed; follows PresenceParticleTrail's three/webgpu precedent.

import * as THREE from 'three/webgpu'
import type { SpikePageDraw, SpikePageRenderer, SpikePageRendererFactory } from './spikeRenderer'

interface PageSurfaceSpikeGpuStats {
  mode: 'three'
  setFrames: number
  draws: number
  meshes: number
  backend: string
}

declare global {
  interface Window {
    __pageSurfaceSpikeGpu?: PageSurfaceSpikeGpuStats
    __pageSurfaceSpikeGpuError?: string
  }
}

interface PageEntry {
  mesh: THREE.Mesh
  texture: THREE.VideoFrameTexture
  material: THREE.MeshBasicNodeMaterial
  /** Identity of the frame the texture currently points at, so `draw()` can
   *  detect a frame that arrived before the factory resolved (never routed
   *  through `frameArrived`) and treat it as an arrival. */
  lastFrame: VideoFrame | null
}

/**
 * three.js/WebGPU arm of the page-surface spike (arm D). One VideoFrameTexture
 * per page, uploaded through three's normal `copyExternalImageToTexture` path
 * (see WebGPUTextureUtils.js) — this arm exists to measure that copy's cost,
 * not to avoid it.
 */
export const createThreeRenderer: SpikePageRendererFactory = async (canvas) => {
  const stats: PageSurfaceSpikeGpuStats = {
    mode: 'three',
    setFrames: 0,
    draws: 0,
    meshes: 0,
    backend: 'unknown',
  }
  window.__pageSurfaceSpikeGpu = stats

  const renderer = new THREE.WebGPURenderer({ canvas, alpha: true, antialias: false })

  try {
    await renderer.init()
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    window.__pageSurfaceSpikeGpuError = `three renderer.init() failed: ${message}`
    throw new Error(window.__pageSurfaceSpikeGpuError)
  }

  // WebGPURenderer silently falls back to a WebGL2 backend when a WebGPU
  // adapter/device can't be acquired — this arm measures the WebGPU upload
  // path specifically, so a fallback is a hard failure, not a degraded pass.
  if (renderer.backend?.isWebGPUBackend !== true) {
    const message = 'three WebGPURenderer fell back to a non-WebGPU backend'
    window.__pageSurfaceSpikeGpuError = message
    renderer.dispose()
    throw new Error(message)
  }

  stats.backend = 'webgpu'
  renderer.setClearColor(0x000000, 0)

  const scene = new THREE.Scene()
  // z range is arbitrary (no depth testing happens; renderOrder alone
  // decides paint order), just wide enough to hold z=0 planes comfortably.
  const camera = new THREE.OrthographicCamera(0, 1, 0, 1, -1000, 1000)
  camera.position.z = 100

  // One geometry shared by every page mesh in this renderer instance; owned
  // and disposed by this factory call, not module-level (a second spike
  // surface gets its own instance and must not race this one's dispose).
  const geometry = new THREE.PlaneGeometry(1, 1)

  const pages = new Map<string, PageEntry>()
  let lastWidth = 0
  let lastHeight = 0

  function createPage(pageId: string): PageEntry {
    const texture = new THREE.VideoFrameTexture()
    texture.colorSpace = THREE.SRGBColorSpace
    texture.generateMipmaps = false
    texture.minFilter = THREE.LinearFilter
    texture.magFilter = THREE.LinearFilter

    const material = new THREE.MeshBasicNodeMaterial({
      map: texture,
      transparent: false,
      depthTest: false,
      depthWrite: false,
      toneMapped: false,
      // The y-down camera below is built by putting `top` at the smaller
      // device-px value, which flips the usual up=+Y sense; the mesh's own
      // Y scale is negated to match (see draw()), and that negation flips
      // triangle winding. DoubleSide sidesteps getting the winding direction
      // exactly right rather than risking a culled, invisible plane.
      side: THREE.DoubleSide,
    })

    const mesh = new THREE.Mesh(geometry, material)
    mesh.visible = false
    scene.add(mesh)

    const entry: PageEntry = { mesh, texture, material, lastFrame: null }
    pages.set(pageId, entry)
    stats.meshes = pages.size
    return entry
  }

  function pageFor(pageId: string): PageEntry {
    return pages.get(pageId) ?? createPage(pageId)
  }

  // Three uploads lazily at render time (`needsUpdate` just marks the
  // texture dirty); this only points the texture at the newest frame.
  function setFrame(pageId: string, frame: VideoFrame): void {
    const entry = pageFor(pageId)
    entry.texture.setFrame(frame)
    entry.lastFrame = frame
    stats.setFrames += 1
  }

  function removePage(pageId: string): void {
    const entry = pages.get(pageId)
    if (!entry) return
    pages.delete(pageId)
    scene.remove(entry.mesh)
    // Drop the reference before disposing so a closed frame is never left
    // sitting on a texture that might otherwise be revisited.
    entry.texture.image = null
    entry.texture.dispose()
    entry.material.dispose()
    stats.meshes = pages.size
  }

  const api: SpikePageRenderer = {
    frameArrived(pageId, frame) {
      setFrame(pageId, frame)
    },

    pageRemoved(pageId) {
      removePage(pageId)
    },

    draw(pageDraws: readonly SpikePageDraw[], canvasWidth, canvasHeight) {
      if (canvasWidth !== lastWidth || canvasHeight !== lastHeight) {
        renderer.setPixelRatio(1)
        renderer.setSize(canvasWidth, canvasHeight, false)
        camera.left = 0
        camera.right = canvasWidth
        camera.top = 0
        camera.bottom = canvasHeight
        camera.updateProjectionMatrix()
        lastWidth = canvasWidth
        lastHeight = canvasHeight
      }

      const seen = new Set<string>()
      for (let i = 0; i < pageDraws.length; i++) {
        const page = pageDraws[i]
        seen.add(page.pageId)
        const entry = pageFor(page.pageId)
        // A frame that arrived before the factory resolved was never routed
        // through frameArrived — catch it here on first sight.
        if (entry.lastFrame !== page.frame) setFrame(page.pageId, page.frame)

        // PlaneGeometry's local +Y (top of the plane, v=1 in its UVs) must
        // land at the smaller world Y, since the camera above maps world
        // Y=0 to the screen's top row. Scaling Y negative flips that without
        // touching the texture's default flipY (kept true, matching every
        // other upright image in this app).
        entry.mesh.position.set(page.x + page.width / 2, page.y + page.height / 2, 0)
        entry.mesh.scale.set(page.width, -page.height, 1)
        entry.mesh.renderOrder = i
        entry.mesh.visible = true
      }

      for (const [pageId, entry] of pages) {
        if (!seen.has(pageId)) entry.mesh.visible = false
      }

      // Three's WebGPURenderer.render() is synchronous once init() has
      // resolved (renderAsync is deprecated in 0.184) — no promise to await.
      renderer.render(scene, camera)
      stats.draws += 1
    },

    dispose() {
      for (const pageId of Array.from(pages.keys())) removePage(pageId)
      geometry.dispose()
      renderer.dispose()
      if (window.__pageSurfaceSpikeGpu === stats) delete window.__pageSurfaceSpikeGpu
    },
  }

  return api
}

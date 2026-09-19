// @ts-nocheck — no @webgpu/types in this repo (not adding a dependency for a
// throwaway spike); WebGPU globals (GPUShaderStage, GPUBufferUsage,
// GPUTextureUsage) and the whole API surface are untyped here, same as the
// `three/webgpu` TSL precedent in PresenceParticleTrail.tsx.

import type { PageFrameMeta } from '../../../shared/page-frames'
import { EXTERNAL_TEXTURE_SHADER, TEXTURE_2D_SHADER, UNIFORM_STRUCT_SIZE } from './rawWebGpuShaders'
import type { SpikePageDraw, SpikePageRenderer, SpikePageRendererFactory } from './spikeRenderer'

declare global {
  interface Window {
    __pageSurfaceSpikeGpu?: {
      mode: 'import' | 'blit'
      imports: number
      blits: number
      textureRecreates: number
      draws: number
      ownedTextureBytes: number
    }
    __pageSurfaceSpikeGpuError?: string
  }
}

/** How long a page's wanted on-screen size must hold steady before the blit
 *  arm recreates its owned texture at the new size and re-blits. */
const SETTLE_MS = 150
const INITIAL_UNIFORM_CAPACITY = 256

function logGpuError(message: string) {
  const full = `[spike-webgpu] ${message}`
  console.error(full)
  window.__pageSurfaceSpikeGpuError = full
}

/** True once a wanted size has drifted from an owned texture's size by more
 *  than ~2px or ~2% — the threshold that gates a settle-triggered recreate. */
function sizeDiffers(wantedW: number, wantedH: number, texW: number, texH: number): boolean {
  if (texW <= 0 || texH <= 0) return true
  const dw = Math.abs(wantedW - texW)
  const dh = Math.abs(wantedH - texH)
  return dw > 2 || dh > 2 || dw / texW > 0.02 || dh / texH > 0.02
}

interface UniformEntry {
  x: number
  y: number
  w: number
  h: number
  targetW: number
  targetH: number
  radius: number
}

/** Per-page state for the 'blit' arm: the held frame, the frame last blitted
 *  into the owned texture (comparing the two is the arrival signal), the
 *  wanted on-screen size and how long it's held, and the owned GPU texture. */
interface BlitPageState {
  frame: VideoFrame | null
  blittedFrame: VideoFrame | null
  wantedWidth: number
  wantedHeight: number
  sizeStableSince: number
  texture: any | null
  texWidth: number
  texHeight: number
  textureBindGroup: any | null
}

export const createRawWebGpuRenderer =
  (mode: 'import' | 'blit'): SpikePageRendererFactory =>
  async (canvas) => {
    if (!navigator.gpu) throw new Error('navigator.gpu is unavailable (WebGPU not enabled)')
    const adapter = await navigator.gpu.requestAdapter()
    if (!adapter) throw new Error('navigator.gpu.requestAdapter() returned null')
    const device = await adapter.requestDevice()
    if (!device) throw new Error('adapter.requestDevice() returned null')
    const context = canvas.getContext('webgpu')
    if (!context) throw new Error("canvas.getContext('webgpu') returned null")

    const presentationFormat = navigator.gpu.getPreferredCanvasFormat()
    context.configure({ device, format: presentationFormat, alphaMode: 'premultiplied' })

    device.lost.then((info: any) => {
      logGpuError(`device lost (${info?.reason ?? 'unknown'}): ${info?.message ?? ''}`)
    })
    device.addEventListener('uncapturederror', (event: any) => {
      logGpuError(`uncaptured error: ${event?.error?.message ?? String(event?.error)}`)
    })

    const stats = {
      mode,
      imports: 0,
      blits: 0,
      textureRecreates: 0,
      draws: 0,
      ownedTextureBytes: 0,
    }
    window.__pageSurfaceSpikeGpu = stats

    const sampler = device.createSampler({
      magFilter: 'linear',
      minFilter: 'linear',
      addressModeU: 'clamp-to-edge',
      addressModeV: 'clamp-to-edge',
    })

    // Explicit layouts (not `layout: 'auto'`) so group 0 — the dynamic-offset
    // uniform — is identical across every pipeline and its bind group can be
    // created once and reused for every draw and every blit pass.
    const group0Layout = device.createBindGroupLayout({
      entries: [
        {
          binding: 0,
          visibility: GPUShaderStage.VERTEX | GPUShaderStage.FRAGMENT,
          buffer: { type: 'uniform', hasDynamicOffset: true, minBindingSize: UNIFORM_STRUCT_SIZE },
        },
      ],
    })
    const externalGroup1Layout = device.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, externalTexture: {} },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
      ],
    })
    const textureGroup1Layout = device.createBindGroupLayout({
      entries: [
        { binding: 0, visibility: GPUShaderStage.FRAGMENT, texture: { sampleType: 'float' } },
        { binding: 1, visibility: GPUShaderStage.FRAGMENT, sampler: { type: 'filtering' } },
      ],
    })
    const externalPipelineLayout = device.createPipelineLayout({
      bindGroupLayouts: [group0Layout, externalGroup1Layout],
    })
    const texturePipelineLayout = device.createPipelineLayout({
      bindGroupLayouts: [group0Layout, textureGroup1Layout],
    })

    // Premultiplied "over": result = src + dst * (1 - srcAlpha).
    const blendOver = {
      color: { operation: 'add', srcFactor: 'one', dstFactor: 'one-minus-src-alpha' },
      alpha: { operation: 'add', srcFactor: 'one', dstFactor: 'one-minus-src-alpha' },
    }

    const externalModule = device.createShaderModule({ code: EXTERNAL_TEXTURE_SHADER })
    const textureModule = mode === 'blit' ? device.createShaderModule({ code: TEXTURE_2D_SHADER }) : null

    // Arm B (import): one pipeline, texture_external + rounded mask, straight to canvas.
    const externalMaskPipeline =
      mode === 'import'
        ? device.createRenderPipeline({
            layout: externalPipelineLayout,
            vertex: { module: externalModule, entryPoint: 'vs_main' },
            fragment: {
              module: externalModule,
              entryPoint: 'fs_mask',
              targets: [{ format: presentationFormat, blend: blendOver }],
            },
            primitive: { topology: 'triangle-strip' },
          })
        : null

    // Arm C (blit): texture_external plain, fullscreen-into-owned-texture blit pass.
    const externalPlainPipeline =
      mode === 'blit'
        ? device.createRenderPipeline({
            layout: externalPipelineLayout,
            vertex: { module: externalModule, entryPoint: 'vs_main' },
            fragment: { module: externalModule, entryPoint: 'fs_plain', targets: [{ format: 'rgba8unorm' }] },
            primitive: { topology: 'triangle-strip' },
          })
        : null

    // Arm C (blit): texture_2d + rounded mask, owned texture to canvas.
    const textureMaskPipeline =
      mode === 'blit'
        ? device.createRenderPipeline({
            layout: texturePipelineLayout,
            vertex: { module: textureModule, entryPoint: 'vs_main' },
            fragment: {
              module: textureModule,
              entryPoint: 'fs_mask',
              targets: [{ format: presentationFormat, blend: blendOver }],
            },
            primitive: { topology: 'triangle-strip' },
          })
        : null

    const minAlignment = device.limits?.minUniformBufferOffsetAlignment ?? 256
    const uniformStride = Math.ceil(UNIFORM_STRUCT_SIZE / minAlignment) * minAlignment

    let uniformCapacity = INITIAL_UNIFORM_CAPACITY
    let uniformBuffer = device.createBuffer({
      size: uniformStride * uniformCapacity,
      usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
    })
    let uniformBindGroup = device.createBindGroup({
      layout: group0Layout,
      entries: [{ binding: 0, resource: { buffer: uniformBuffer, offset: 0, size: UNIFORM_STRUCT_SIZE } }],
    })

    function ensureUniformCapacity(needed: number) {
      if (needed <= uniformCapacity) return
      uniformCapacity = Math.max(needed, uniformCapacity * 2)
      uniformBuffer.destroy()
      uniformBuffer = device.createBuffer({
        size: uniformStride * uniformCapacity,
        usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
      })
      uniformBindGroup = device.createBindGroup({
        layout: group0Layout,
        entries: [{ binding: 0, resource: { buffer: uniformBuffer, offset: 0, size: UNIFORM_STRUCT_SIZE } }],
      })
    }

    /** Packs every uniform entry needed this draw into one Float32Array and
     *  issues a single queue.writeBuffer, returning each entry's dynamic offset. */
    function writeUniforms(entries: UniformEntry[]): number[] {
      if (entries.length === 0) return []
      ensureUniformCapacity(entries.length)
      const floatsPerSlot = uniformStride / 4
      const data = new Float32Array(entries.length * floatsPerSlot)
      entries.forEach((e, i) => {
        const base = i * floatsPerSlot
        data[base + 0] = e.x
        data[base + 1] = e.y
        data[base + 2] = e.w
        data[base + 3] = e.h
        data[base + 4] = e.targetW
        data[base + 5] = e.targetH
        data[base + 6] = e.radius
        data[base + 7] = 0
      })
      device.queue.writeBuffer(uniformBuffer, 0, data.buffer, data.byteOffset, data.byteLength)
      return entries.map((_, i) => i * uniformStride)
    }

    // ---- arm B (import) state: only enough to satisfy the interface; import
    // mode re-imports and re-draws every page on every draw() regardless. ----
    const lastSeenFrame = new Map<string, VideoFrame>()

    // ---- arm C (blit) state ----
    const pageStates = new Map<string, BlitPageState>()
    let settleTimer: ReturnType<typeof setTimeout> | null = null
    let lastDrawArgs: { pages: readonly SpikePageDraw[]; width: number; height: number } | null = null

    function getOrCreatePageState(pageId: string): BlitPageState {
      let state = pageStates.get(pageId)
      if (!state) {
        state = {
          frame: null,
          blittedFrame: null,
          wantedWidth: 0,
          wantedHeight: 0,
          sizeStableSince: performance.now(),
          texture: null,
          texWidth: 0,
          texHeight: 0,
          textureBindGroup: null,
        }
        pageStates.set(pageId, state)
      }
      return state
    }

    function recomputeOwnedBytes() {
      let bytes = 0
      for (const state of pageStates.values()) {
        if (state.texture) bytes += state.texWidth * state.texHeight * 4
      }
      stats.ownedTextureBytes = bytes
    }

    function ensurePageTexture(state: BlitPageState, width: number, height: number) {
      if (state.texture && state.texWidth === width && state.texHeight === height) return
      state.texture?.destroy()
      state.texture = device.createTexture({
        size: { width, height },
        format: 'rgba8unorm',
        usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
      })
      state.texWidth = width
      state.texHeight = height
      state.textureBindGroup = device.createBindGroup({
        layout: textureGroup1Layout,
        entries: [
          { binding: 0, resource: state.texture.createView() },
          { binding: 1, resource: sampler },
        ],
      })
      stats.textureRecreates += 1
      recomputeOwnedBytes()
    }

    /** Imports `state.frame` and blits it into the (possibly just-recreated)
     *  owned texture, encoded into `encoder`. The external texture and the
     *  bind group referencing it are created and consumed synchronously here,
     *  within the caller's task, as required for a GPUExternalTexture. */
    function blitPageTexture(encoder: any, state: BlitPageState, width: number, height: number, uniformOffset: number) {
      if (!state.frame) return
      ensurePageTexture(state, width, height)
      const externalTexture = device.importExternalTexture({ source: state.frame })
      stats.imports += 1
      const blitBindGroup = device.createBindGroup({
        layout: externalGroup1Layout,
        entries: [
          { binding: 0, resource: externalTexture },
          { binding: 1, resource: sampler },
        ],
      })
      const pass = encoder.beginRenderPass({
        colorAttachments: [
          {
            view: state.texture.createView(),
            loadOp: 'clear',
            storeOp: 'store',
            clearValue: { r: 0, g: 0, b: 0, a: 0 },
          },
        ],
      })
      pass.setPipeline(externalPlainPipeline)
      pass.setBindGroup(0, uniformBindGroup, [uniformOffset])
      pass.setBindGroup(1, blitBindGroup)
      pass.draw(4)
      pass.end()
      state.blittedFrame = state.frame
      stats.blits += 1
    }

    function scheduleSettle() {
      if (settleTimer !== null) clearTimeout(settleTimer)
      settleTimer = setTimeout(onSettle, SETTLE_MS)
    }

    function onSettle() {
      settleTimer = null
      const now = performance.now()
      const toBlit: BlitPageState[] = []
      for (const state of pageStates.values()) {
        if (!state.frame) continue
        if (now - state.sizeStableSince < SETTLE_MS) continue
        if (!sizeDiffers(state.wantedWidth, state.wantedHeight, state.texWidth, state.texHeight)) continue
        toBlit.push(state)
      }
      if (toBlit.length > 0) {
        const entries: UniformEntry[] = toBlit.map((state) => ({
          x: 0,
          y: 0,
          w: state.wantedWidth,
          h: state.wantedHeight,
          targetW: state.wantedWidth,
          targetH: state.wantedHeight,
          radius: 0,
        }))
        const offsets = writeUniforms(entries)
        const encoder = device.createCommandEncoder()
        toBlit.forEach((state, i) => {
          blitPageTexture(encoder, state, state.wantedWidth, state.wantedHeight, offsets[i])
        })
        device.queue.submit([encoder.finish()])
      }
      if (lastDrawArgs) draw(lastDrawArgs.pages, lastDrawArgs.width, lastDrawArgs.height)
    }

    function drawImport(pages: readonly SpikePageDraw[], width: number, height: number) {
      stats.draws += 1
      const entries: UniformEntry[] = pages.map((p) => {
        lastSeenFrame.set(p.pageId, p.frame)
        return { x: p.x, y: p.y, w: p.width, h: p.height, targetW: width, targetH: height, radius: p.radius }
      })
      const offsets = writeUniforms(entries)

      const encoder = device.createCommandEncoder()
      const view = context.getCurrentTexture().createView()
      const pass = encoder.beginRenderPass({
        colorAttachments: [{ view, loadOp: 'clear', storeOp: 'store', clearValue: { r: 0, g: 0, b: 0, a: 0 } }],
      })
      pages.forEach((p, i) => {
        // A GPUExternalTexture is only valid for the task that imported it —
        // nothing is cached across draws, per arm B's design.
        const externalTexture = device.importExternalTexture({ source: p.frame })
        stats.imports += 1
        const bindGroup = device.createBindGroup({
          layout: externalGroup1Layout,
          entries: [
            { binding: 0, resource: externalTexture },
            { binding: 1, resource: sampler },
          ],
        })
        pass.setPipeline(externalMaskPipeline)
        pass.setBindGroup(0, uniformBindGroup, [offsets[i]])
        pass.setBindGroup(1, bindGroup)
        pass.draw(4)
      })
      pass.end()
      device.queue.submit([encoder.finish()])
    }

    function drawBlit(pages: readonly SpikePageDraw[], width: number, height: number) {
      stats.draws += 1
      lastDrawArgs = { pages, width, height }
      const now = performance.now()
      const blitTargets: { state: BlitPageState; width: number; height: number }[] = []

      for (const p of pages) {
        const state = getOrCreatePageState(p.pageId)
        // The held frame is always kept fresh here (covers the quirk: a page
        // whose frame arrived before this factory resolved never went
        // through frameArrived, so this is the first place it's seen).
        state.frame = p.frame

        const rawW = Math.max(1, Math.ceil(p.width))
        const rawH = Math.max(1, Math.ceil(p.height))
        const cappedW = Math.min(rawW, Math.max(1, Math.round(p.meta.width)))
        const cappedH = Math.min(rawH, Math.max(1, Math.round(p.meta.height)))
        if (cappedW !== state.wantedWidth || cappedH !== state.wantedHeight) {
          state.wantedWidth = cappedW
          state.wantedHeight = cappedH
          state.sizeStableSince = now
          scheduleSettle()
        }

        // Arrival: the currently held frame hasn't been blitted yet. Works
        // whether or not frameArrived ever fired for it.
        const isNewFrame = state.blittedFrame !== state.frame
        const stable = now - state.sizeStableSince >= SETTLE_MS
        const settleNeedsBlit =
          !isNewFrame && stable && sizeDiffers(state.wantedWidth, state.wantedHeight, state.texWidth, state.texHeight)
        const neverBlitted = !state.texture

        if (isNewFrame || settleNeedsBlit || neverBlitted) {
          blitTargets.push({ state, width: state.wantedWidth, height: state.wantedHeight })
        }
      }

      const blitEntries: UniformEntry[] = blitTargets.map(({ width: w, height: h }) => ({
        x: 0,
        y: 0,
        w,
        h,
        targetW: w,
        targetH: h,
        radius: 0,
      }))
      const mainEntries: UniformEntry[] = pages.map((p) => ({
        x: p.x,
        y: p.y,
        w: p.width,
        h: p.height,
        targetW: width,
        targetH: height,
        radius: p.radius,
      }))
      const offsets = writeUniforms([...blitEntries, ...mainEntries])
      const blitOffsets = offsets.slice(0, blitTargets.length)
      const mainOffsets = offsets.slice(blitTargets.length)

      const encoder = device.createCommandEncoder()
      // Pending blits are batched into this same encoder, before the draw pass.
      blitTargets.forEach(({ state, width: w, height: h }, i) => {
        blitPageTexture(encoder, state, w, h, blitOffsets[i])
      })

      const view = context.getCurrentTexture().createView()
      const pass = encoder.beginRenderPass({
        colorAttachments: [{ view, loadOp: 'clear', storeOp: 'store', clearValue: { r: 0, g: 0, b: 0, a: 0 } }],
      })
      pages.forEach((p, i) => {
        const state = pageStates.get(p.pageId)
        // No owned texture yet (a same-draw blit above should have created
        // one whenever a frame is held) — nothing to draw for this page.
        if (!state || !state.textureBindGroup) return
        pass.setPipeline(textureMaskPipeline)
        pass.setBindGroup(0, uniformBindGroup, [mainOffsets[i]])
        pass.setBindGroup(1, state.textureBindGroup)
        pass.draw(4)
      })
      pass.end()
      device.queue.submit([encoder.finish()])
    }

    function draw(pages: readonly SpikePageDraw[], width: number, height: number) {
      if (mode === 'import') drawImport(pages, width, height)
      else drawBlit(pages, width, height)
    }

    const renderer: SpikePageRenderer = {
      frameArrived(pageId: string, frame: VideoFrame, meta: PageFrameMeta) {
        if (mode === 'import') {
          lastSeenFrame.set(pageId, frame)
          return
        }
        // Blits happen lazily inside draw() (it can always tell arrival apart
        // from a stale frame by comparing against `blittedFrame`), so this
        // just keeps the held-frame reference current for a page that isn't
        // on screen yet, or between paints.
        getOrCreatePageState(pageId).frame = frame
        void meta
      },
      pageRemoved(pageId: string) {
        if (mode === 'import') {
          lastSeenFrame.delete(pageId)
          return
        }
        const state = pageStates.get(pageId)
        state?.texture?.destroy()
        pageStates.delete(pageId)
        recomputeOwnedBytes()
      },
      draw(pages: readonly SpikePageDraw[], canvasWidth: number, canvasHeight: number) {
        draw(pages, canvasWidth, canvasHeight)
      },
      dispose() {
        if (settleTimer !== null) {
          clearTimeout(settleTimer)
          settleTimer = null
        }
        for (const state of pageStates.values()) state.texture?.destroy()
        pageStates.clear()
        lastSeenFrame.clear()
        uniformBuffer.destroy()
        context.unconfigure()
        device.destroy()
        if (window.__pageSurfaceSpikeGpu === stats) delete window.__pageSurfaceSpikeGpu
      },
    }

    return renderer
  }

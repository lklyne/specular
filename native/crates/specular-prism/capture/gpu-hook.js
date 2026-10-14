// Injected before any page script. Records every WebGPU object the page creates,
// a per-frame command trace on demand, and can snapshot every render pass's
// colour attachments. Also replaces time with a fixed-step virtual clock.
(() => {
  if (!('gpu' in navigator)) return
  const G = (window.__gpu = {
    objs: new Map(), // id -> record
    trace: window.__prismTraceFromStart ? [] : null, // array while recording
    recordFrames: 0,
    snapshot: false, // when true, copy each pass's colour attachments after end()
    snaps: [], // {passIndex, label, texId, w, h, format, buffer, bytesPerRow}
    shadow: new Map(), // bufferId -> Uint8Array copy of latest contents
    texData: new Map(), // texId -> [{origin, layout, size, mip, bytes}]
    passCount: 0,
    frame: 0,
  })
  let nextId = 1
  const ids = new WeakMap()
  const idOf = (o) => {
    if (o == null) return null
    let id = ids.get(o)
    if (!id) {
      id = nextId++
      ids.set(o, id)
    }
    return id
  }
  G.idOf = idOf
  // three.js reuses descriptor objects between calls, so every descriptor is copied at call time
  const GPU_CLASSES = () => [GPUTexture, GPUTextureView, GPUBuffer, GPUSampler, GPUBindGroup, GPUBindGroupLayout, GPUPipelineLayout, GPURenderPipeline, GPUComputePipeline, GPUShaderModule]
  const snap = (o) => {
    if (o == null || typeof o !== 'object') return o
    const C = GPU_CLASSES()
    const walk = (v, depth) => {
      if (v == null || typeof v !== 'object') return v
      if (C.some((K) => v instanceof K)) return { $ref: idOf(v), $kind: v.constructor.name }
      if (ArrayBuffer.isView(v)) return { $bytes: v.byteLength }
      if (depth > 8) return '<deep>'
      if (Array.isArray(v) || (typeof v[Symbol.iterator] === 'function' && !(typeof v === 'string'))) return Array.from(v, (x) => walk(x, depth + 1))
      const out = {}
      for (const k in v) { const x = v[k]; if (typeof x !== 'function') out[k] = walk(x, depth + 1) }
      return out
    }
    return walk(o, 0)
  }
  G.snap = snap
  const reg = (o, rec) => {
    const id = idOf(o)
    G.objs.set(id, { id, ...rec })
    return id
  }
  const rec = (op, args) => {
    if (G.trace) G.trace.push({ op, ...args })
  }

  // ---- virtual clock: fixed 60 Hz step per real frame, pausable ----
  const realNow = performance.now.bind(performance)
  G.vt = 0
  G.paused = !!window.__prismStartPaused
  G.budget = Infinity // frames left to advance before auto-pause
  G.step = 1000 / 60
  G.frameIndex = 0
  let lastRealTs = -1
  performance.now = () => G.vt
  const realRaf = window.requestAnimationFrame.bind(window)
  window.requestAnimationFrame = (cb) =>
    realRaf((ts) => {
      if (ts !== lastRealTs) {
        lastRealTs = ts
        if (!G.paused && G.budget > 0) {
          G.vt += G.step
          G.frameIndex++
          G.budget--
        }
      }
      cb(G.vt)
    })
  G.realNow = realNow
  G.realRaf = realRaf
  // advance exactly n virtual frames, then hold; resolves when held
  G.advance = (n) =>
    new Promise((resolve) => {
      G.budget = n
      G.paused = false
      const poll = () => (G.budget <= 0 ? realRaf(() => realRaf(resolve)) : realRaf(poll))
      poll()
    })
  G.waitReal = (n) => new Promise((r) => { let k = 0; const f = () => (++k >= n ? r() : realRaf(f)); realRaf(f) })

  const fmtBytes = {
    rgba8unorm: 4, 'rgba8unorm-srgb': 4, bgra8unorm: 4, 'bgra8unorm-srgb': 4, rgba16float: 8,
    rgba32float: 16, r8unorm: 1, rg8unorm: 2, r16float: 2, rg16float: 4, r32float: 4, rg32float: 8,
    rgb10a2unorm: 4, rg11b10ufloat: 4,
  }
  G.fmtBytes = fmtBytes

  const P = (proto, name, wrap) => {
    const orig = proto[name]
    if (!orig) return
    proto[name] = function (...a) {
      return wrap.call(this, orig, a)
    }
  }
  const viewOf = (v) => (v ? idOf(v) : null)

  // ---- adapter/device ----
  P(GPUAdapter.prototype, 'requestDevice', async function (orig, a) {
    const d = await orig.apply(this, a)
    G.device = d
    reg(d, { kind: 'device', desc: a[0] ? JSON.parse(JSON.stringify(a[0], (k, v) => (v instanceof Set ? [...v] : v))) : null })
    return d
  })

  P(GPUDevice.prototype, 'createShaderModule', function (orig, a) {
    const m = orig.apply(this, a)
    reg(m, { kind: 'shader', label: a[0].label || '', code: a[0].code, t: G.vt, frame: G.frameIndex })
    return m
  })
  const stageRec = (s) => (s ? { module: idOf(s.module), entryPoint: s.entryPoint, constants: snap(s.constants), buffers: snap(s.buffers), targets: snap(s.targets) } : null)
  const pipeRec = (d) => ({
    label: d.label || '',
    layout: d.layout === 'auto' ? 'auto' : idOf(d.layout),
    vertex: stageRec(d.vertex),
    fragment: stageRec(d.fragment),
    compute: stageRec(d.compute),
    primitive: snap(d.primitive),
    depthStencil: snap(d.depthStencil),
    multisample: snap(d.multisample),
  })
  const wrapPipe = (o) => {
    // getBindGroupLayout on auto layouts
    return o
  }
  P(GPUDevice.prototype, 'createRenderPipeline', function (orig, a) {
    const p = orig.apply(this, a)
    reg(p, { kind: 'renderPipeline', ...pipeRec(a[0]) })
    return wrapPipe(p)
  })
  P(GPUDevice.prototype, 'createRenderPipelineAsync', async function (orig, a) {
    const p = await orig.apply(this, a)
    reg(p, { kind: 'renderPipeline', async: true, ...pipeRec(a[0]) })
    return p
  })
  P(GPUDevice.prototype, 'createComputePipeline', function (orig, a) {
    const p = orig.apply(this, a)
    reg(p, { kind: 'computePipeline', ...pipeRec(a[0]) })
    return p
  })
  P(GPUDevice.prototype, 'createComputePipelineAsync', async function (orig, a) {
    const p = await orig.apply(this, a)
    reg(p, { kind: 'computePipeline', async: true, ...pipeRec(a[0]) })
    return p
  })
  P(GPURenderPipeline.prototype, 'getBindGroupLayout', function (orig, a) {
    const l = orig.apply(this, a)
    if (!G.objs.has(ids.get(l))) reg(l, { kind: 'bindGroupLayout', auto: true, pipeline: idOf(this), index: a[0] })
    return l
  })
  P(GPUComputePipeline.prototype, 'getBindGroupLayout', function (orig, a) {
    const l = orig.apply(this, a)
    if (!G.objs.has(ids.get(l))) reg(l, { kind: 'bindGroupLayout', auto: true, pipeline: idOf(this), index: a[0] })
    return l
  })
  P(GPUDevice.prototype, 'createBindGroupLayout', function (orig, a) {
    const l = orig.apply(this, a)
    reg(l, { kind: 'bindGroupLayout', label: a[0].label || '', entries: snap(a[0].entries) })
    return l
  })
  P(GPUDevice.prototype, 'createPipelineLayout', function (orig, a) {
    const l = orig.apply(this, a)
    reg(l, { kind: 'pipelineLayout', label: a[0].label || '', bindGroupLayouts: a[0].bindGroupLayouts.map(idOf) })
    return l
  })
  P(GPUDevice.prototype, 'createBindGroup', function (orig, a) {
    const g = orig.apply(this, a)
    const entries = a[0].entries.map((e) => {
      const r = e.resource
      if (r instanceof GPUSampler) return { binding: e.binding, sampler: idOf(r) }
      if (r instanceof GPUTextureView) return { binding: e.binding, textureView: idOf(r) }
      if (typeof GPUExternalTexture !== 'undefined' && r instanceof GPUExternalTexture) return { binding: e.binding, external: true }
      if (r instanceof GPUBuffer) return { binding: e.binding, buffer: idOf(r) }
      return { binding: e.binding, buffer: idOf(r.buffer), offset: r.offset || 0, size: r.size }
    })
    reg(g, { kind: 'bindGroup', label: a[0].label || '', layout: idOf(a[0].layout), entries })
    return g
  })
  P(GPUDevice.prototype, 'createBuffer', function (orig, a) {
    const b = orig.apply(this, a)
    reg(b, { kind: 'buffer', label: a[0].label || '', size: a[0].size, usage: a[0].usage, mappedAtCreation: !!a[0].mappedAtCreation })
    return b
  })
  P(GPUBuffer.prototype, 'getMappedRange', function (orig, a) {
    const r = orig.apply(this, a)
    ;(this.__ranges || (this.__ranges = [])).push({ off: a[0] || 0, ab: r })
    return r
  })
  P(GPUBuffer.prototype, 'unmap', function (orig, a) {
    const id = idOf(this)
    const o = G.objs.get(id)
    if (this.__ranges && o && o.mappedAtCreation && !(o.usage & GPUBufferUsage.MAP_READ)) {
      let sh = G.shadow.get(id)
      if (!sh) G.shadow.set(id, (sh = new Uint8Array(this.size)))
      for (const { off, ab } of this.__ranges) {
        try { sh.set(new Uint8Array(ab), off) } catch (e) {}
      }
    }
    this.__ranges = null
    return orig.apply(this, a)
  })
  P(GPUDevice.prototype, 'createTexture', function (orig, a) {
    const d = { ...a[0] }
    const sc = d.sampleCount || 1
    const isDepth = String(d.format).startsWith('depth') || String(d.format).startsWith('stencil')
    if (sc === 1 && !isDepth) d.usage |= GPUTextureUsage.COPY_SRC
    const t = orig.call(this, d)
    const size = Array.isArray(a[0].size) ? a[0].size : [a[0].size.width, a[0].size.height || 1, a[0].size.depthOrArrayLayers || 1]
    reg(t, { kind: 'texture', label: a[0].label || '', size: snap(size), format: a[0].format, usage: a[0].usage, mipLevelCount: a[0].mipLevelCount || 1, sampleCount: sc, dimension: a[0].dimension || '2d', viewFormats: snap(a[0].viewFormats) })
    return t
  })
  P(GPUTexture.prototype, 'createView', function (orig, a) {
    const v = orig.apply(this, a)
    reg(v, { kind: 'textureView', texture: idOf(this), desc: snap(a[0]) || null })
    return v
  })
  P(GPUDevice.prototype, 'createSampler', function (orig, a) {
    const s = orig.apply(this, a)
    reg(s, { kind: 'sampler', desc: snap(a[0]) || {} })
    return s
  })
  P(GPUCanvasContext.prototype, 'configure', function (orig, a) {
    const d = { ...a[0], usage: (a[0].usage ?? GPUTextureUsage.RENDER_ATTACHMENT) | GPUTextureUsage.COPY_SRC }
    G.canvasConfig = { format: a[0].format, alphaMode: a[0].alphaMode, toneMapping: a[0].toneMapping, colorSpace: a[0].colorSpace }
    G.canvasContext = this
    return orig.call(this, d)
  })
  P(GPUCanvasContext.prototype, 'getCurrentTexture', function (orig, a) {
    const t = orig.apply(this, a)
    if (!G.objs.has(ids.get(t))) reg(t, { kind: 'texture', canvas: true, size: [t.width, t.height, 1], format: t.format, usage: t.usage, mipLevelCount: 1, sampleCount: 1 })
    return t
  })

  // ---- queue ----
  P(GPUQueue.prototype, 'writeBuffer', function (orig, a) {
    const [buf, off, data, dataOff = 0, size] = a
    const id = idOf(buf)
    let bytes
    if (data instanceof ArrayBuffer) bytes = new Uint8Array(data)
    else bytes = new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
    const elt = data.BYTES_PER_ELEMENT || 1
    const start = dataOff * elt
    const len = size !== undefined ? size * elt : bytes.byteLength - start
    const slice = bytes.subarray(start, start + len)
    let sh = G.shadow.get(id)
    if (!sh) G.shadow.set(id, (sh = new Uint8Array(buf.size)))
    sh.set(slice, off)
    if (G.trace) rec('writeBuffer', { buffer: id, offset: off, size: len, data: Array.from(new Uint8Array(slice)) })
    return orig.apply(this, a)
  })
  P(GPUQueue.prototype, 'writeTexture', function (orig, a) {
    const [dst, data, layout, size] = a
    const id = idOf(dst.texture)
    let bytes = data instanceof ArrayBuffer ? new Uint8Array(data) : new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
    let list = G.texData.get(id)
    if (!list) G.texData.set(id, (list = []))
    list.push({ origin: snap(dst.origin) || null, mip: dst.mipLevel || 0, layout: snap(layout), size: snap(size), bytes: new Uint8Array(bytes) })
    rec('writeTexture', { texture: id, mip: dst.mipLevel || 0, origin: snap(dst.origin), size: snap(size) })
    return orig.apply(this, a)
  })
  P(GPUQueue.prototype, 'copyExternalImageToTexture', function (orig, a) {
    const [src, dst, size] = a
    const id = idOf(dst.texture)
    let list = G.texData.get(id)
    if (!list) G.texData.set(id, (list = []))
    list.push({ external: true, sourceKind: src.source?.constructor?.name, flipY: src.flipY, premultipliedAlpha: dst.premultipliedAlpha, colorSpace: dst.colorSpace, mip: dst.mipLevel || 0, origin: snap(dst.origin) || null, size: snap(size) })
    rec('copyExternalImageToTexture', { texture: id, size: snap(size), mip: dst.mipLevel || 0 })
    return orig.apply(this, a)
  })
  P(GPUQueue.prototype, 'submit', function (orig, a) {
    rec('submit', { buffers: a[0].map(idOf) })
    const r = orig.apply(this, a)
    if (G.trace) {
      G.recordFrames--
      if (G.recordFrames <= 0 && G.onTraceDone) {
        // finish on the frame's last submit: wait one microtask cycle
      }
    }
    return r
  })

  // ---- encoders ----
  P(GPUDevice.prototype, 'createCommandEncoder', function (orig, a) {
    const e = orig.apply(this, a)
    rec('createCommandEncoder', { encoder: idOf(e), label: a[0]?.label })
    return e
  })
  P(GPUCommandEncoder.prototype, 'finish', function (orig, a) {
    const cb = orig.apply(this, a)
    rec('finish', { encoder: idOf(this), commandBuffer: idOf(cb) })
    return cb
  })
  P(GPUCommandEncoder.prototype, 'beginRenderPass', function (orig, a) {
    const p = orig.apply(this, a)
    const d = a[0]
    const colors = (d.colorAttachments || []).map((c) =>
      c ? { view: viewOf(c.view), resolveTarget: viewOf(c.resolveTarget), loadOp: c.loadOp, storeOp: c.storeOp, clearValue: snap(c.clearValue) } : null,
    )
    const ds = d.depthStencilAttachment
    const depth = ds ? { view: viewOf(ds.view), depthLoadOp: ds.depthLoadOp, depthStoreOp: ds.depthStoreOp, depthClearValue: ds.depthClearValue, stencilLoadOp: ds.stencilLoadOp } : null
    p.__enc = this
    p.__colors = d.colorAttachments
    p.__index = G.passCount++
    rec('beginRenderPass', { pass: idOf(p), passIndex: p.__index, encoder: idOf(this), label: d.label, colors, depth })
    return p
  })
  P(GPUCommandEncoder.prototype, 'beginComputePass', function (orig, a) {
    const p = orig.apply(this, a)
    rec('beginComputePass', { pass: idOf(p), encoder: idOf(this), label: a[0]?.label })
    return p
  })
  P(GPUCommandEncoder.prototype, 'copyTextureToTexture', function (orig, a) {
    rec('copyTextureToTexture', { src: idOf(a[0].texture), srcMip: a[0].mipLevel || 0, srcOrigin: snap(a[0].origin), dst: idOf(a[1].texture), dstMip: a[1].mipLevel || 0, dstOrigin: snap(a[1].origin), size: snap(a[2]) })
    return orig.apply(this, a)
  })
  P(GPUCommandEncoder.prototype, 'copyBufferToBuffer', function (orig, a) {
    rec('copyBufferToBuffer', { src: idOf(a[0]), srcOff: a[1], dst: idOf(a[2]), dstOff: a[3], size: a[4] })
    return orig.apply(this, a)
  })
  P(GPUCommandEncoder.prototype, 'copyBufferToTexture', function (orig, a) {
    rec('copyBufferToTexture', { src: idOf(a[0].buffer), dst: idOf(a[1].texture), size: snap(a[2]) })
    return orig.apply(this, a)
  })
  P(GPUCommandEncoder.prototype, 'clearBuffer', function (orig, a) {
    rec('clearBuffer', { buffer: idOf(a[0]) })
    return orig.apply(this, a)
  })

  const passProtos = [GPURenderPassEncoder.prototype, GPUComputePassEncoder.prototype]
  if (typeof GPURenderBundleEncoder !== 'undefined') passProtos.push(GPURenderBundleEncoder.prototype)
  for (const proto of passProtos) {
    P(proto, 'setPipeline', function (orig, a) { rec('setPipeline', { pass: idOf(this), pipeline: idOf(a[0]) }); return orig.apply(this, a) })
    P(proto, 'setBindGroup', function (orig, a) { rec('setBindGroup', { pass: idOf(this), index: a[0], group: idOf(a[1]), dyn: a[2] ? Array.from(a[2]) : undefined }); return orig.apply(this, a) })
    P(proto, 'setVertexBuffer', function (orig, a) { rec('setVertexBuffer', { pass: idOf(this), slot: a[0], buffer: idOf(a[1]), offset: a[2], size: a[3] }); return orig.apply(this, a) })
    P(proto, 'setIndexBuffer', function (orig, a) { rec('setIndexBuffer', { pass: idOf(this), buffer: idOf(a[0]), format: a[1], offset: a[2], size: a[3] }); return orig.apply(this, a) })
    P(proto, 'draw', function (orig, a) { rec('draw', { pass: idOf(this), args: a }); return orig.apply(this, a) })
    P(proto, 'drawIndexed', function (orig, a) { rec('drawIndexed', { pass: idOf(this), args: a }); return orig.apply(this, a) })
    P(proto, 'drawIndirect', function (orig, a) { rec('drawIndirect', { pass: idOf(this) }); return orig.apply(this, a) })
    P(proto, 'drawIndexedIndirect', function (orig, a) { rec('drawIndexedIndirect', { pass: idOf(this) }); return orig.apply(this, a) })
    P(proto, 'dispatchWorkgroups', function (orig, a) { rec('dispatchWorkgroups', { pass: idOf(this), args: a }); return orig.apply(this, a) })
  }
  P(GPURenderPassEncoder.prototype, 'setViewport', function (orig, a) { rec('setViewport', { pass: idOf(this), args: a }); return orig.apply(this, a) })
  P(GPURenderPassEncoder.prototype, 'setScissorRect', function (orig, a) { rec('setScissorRect', { pass: idOf(this), args: a }); return orig.apply(this, a) })
  P(GPURenderPassEncoder.prototype, 'setBlendConstant', function (orig, a) { rec('setBlendConstant', { pass: idOf(this), args: a }); return orig.apply(this, a) })
  P(GPURenderPassEncoder.prototype, 'setStencilReference', function (orig, a) { rec('setStencilReference', { pass: idOf(this), args: a }); return orig.apply(this, a) })
  P(GPURenderPassEncoder.prototype, 'executeBundles', function (orig, a) { rec('executeBundles', { pass: idOf(this), bundles: a[0].map(idOf) }); return orig.apply(this, a) })
  P(GPUComputePassEncoder.prototype, 'end', function (orig, a) { rec('endComputePass', { pass: idOf(this) }); return orig.apply(this, a) })
  P(GPURenderPassEncoder.prototype, 'end', function (orig, a) {
    rec('endRenderPass', { pass: idOf(this) })
    const r = orig.apply(this, a)
    if (G.snapshot && this.__colors) {
      const enc = this.__enc
      this.__colors.forEach((c, slot) => {
        if (!c) return
        const view = c.resolveTarget || c.view
        const vrec = G.objs.get(ids.get(view))
        const trec = vrec && G.objs.get(vrec.texture)
        let tex = vrec && vrec.__tex
        if (!trec) return
        const mip = vrec.desc?.baseMipLevel || 0
        const layer = vrec.desc?.baseArrayLayer || 0
        const bpp = fmtBytes[trec.format]
        if (!bpp || trec.sampleCount > 1) return
        const w = Math.max(1, trec.size[0] >> mip)
        const h = Math.max(1, trec.size[1] >> mip)
        const bytesPerRow = Math.ceil((w * bpp) / 256) * 256
        const buffer = G.device.createBuffer({ size: bytesPerRow * h, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ })
        const texture = G.texById.get(trec.id)
        if (!texture) return
        enc.copyTextureToBuffer({ texture, mipLevel: mip, origin: { x: 0, y: 0, z: layer } }, { buffer, bytesPerRow }, { width: w, height: h, depthOrArrayLayers: 1 })
        G.snaps.push({ passIndex: this.__index, slot, texId: trec.id, mip, layer, w, h, format: trec.format, bytesPerRow, buffer, label: trec.label, canvas: !!trec.canvas })
      })
    }
    return r
  })
  // keep strong refs to textures by id so snapshots can copy them
  G.texById = new Map()
  const origCreateTexture = GPUDevice.prototype.createTexture
  GPUDevice.prototype.createTexture = function (...a) {
    const t = origCreateTexture.apply(this, a)
    G.texById.set(idOf(t), t)
    return t
  }
  P(GPUTexture.prototype, 'destroy', function (orig, a) {
    const o = G.objs.get(ids.get(this))
    if (o) o.destroyed = G.frameIndex
    if (!G.keepTextures) G.texById.delete(ids.get(this))
    return orig.apply(this, a)
  })
  const origGCT = GPUCanvasContext.prototype.getCurrentTexture
  GPUCanvasContext.prototype.getCurrentTexture = function (...a) {
    const t = origGCT.apply(this, a)
    G.texById.set(idOf(t), t)
    return t
  }
  // store the texture a view came from so snapshot can use it
  const origCV = GPUTexture.prototype.createView
  GPUTexture.prototype.createView = function (...a) {
    const v = origCV.apply(this, a)
    G.texById.set(idOf(this), this)
    return v
  }

  // ---- helpers called from devtools ----
  G.gpuClasses = [GPUTexture, GPUTextureView, GPUBuffer, GPUSampler, GPUBindGroup, GPUBindGroupLayout, GPUPipelineLayout, GPURenderPipeline, GPUComputePipeline, GPUShaderModule, GPUDevice, GPUCommandEncoder, GPUCommandBuffer, GPURenderPassEncoder, GPUComputePassEncoder, GPUQueue]
  G.post = (path, body) => fetch('http://127.0.0.1:8765/' + path, { method: 'POST', body })
  G.json = (o) =>
    JSON.stringify(o, (k, v) => {
      if (v instanceof Set) return [...v]
      if (v && typeof v === 'object' && G.gpuClasses.some((C) => v instanceof C)) return '<' + v.constructor.name + ' ' + idOf(v) + '>'
      return v
    })
  // record N frames of trace (plus optional pass snapshots); resolves after the last frame
  G.recordNext = (n = 1, { snapshot = false } = {}) =>
    new Promise((resolve) => {
      const startFrame = G.frameIndex
      let started = false
      const tick = () => {
        realRaf(() => {
          if (!started) {
            G.trace = []
            G.snaps = []
            G.passCount = 0
            G.snapshot = snapshot
            G.traceStartFrame = G.frameIndex
            started = true
            G.realFrames = 0
          }
          G.realFrames++
          if (G.realFrames > n) {
            const t = G.trace
            G.trace = null
            G.snapshot = false
            resolve(t)
          } else tick()
        })
      }
      tick()
    })
  G.readSnaps = async () => {
    const out = []
    for (const s of G.snaps) {
      await s.buffer.mapAsync(GPUMapMode.READ)
      const data = new Uint8Array(s.buffer.getMappedRange().slice(0))
      s.buffer.unmap()
      s.buffer.destroy()
      out.push({ ...s, buffer: undefined, data })
    }
    G.snaps = []
    return out
  }
  G.readTexture = async (texId, mip = 0, layer = 0) => {
    const trec = G.objs.get(texId)
    const texture = G.texById.get(texId)
    const bpp = fmtBytes[trec.format]
    const w = Math.max(1, trec.size[0] >> mip)
    const h = Math.max(1, trec.size[1] >> mip)
    const bytesPerRow = Math.ceil((w * bpp) / 256) * 256
    const buffer = G.device.createBuffer({ size: bytesPerRow * h, usage: GPUBufferUsage.COPY_DST | GPUBufferUsage.MAP_READ })
    const enc = G.device.createCommandEncoder()
    enc.copyTextureToBuffer({ texture, mipLevel: mip, origin: { x: 0, y: 0, z: layer } }, { buffer, bytesPerRow }, { width: w, height: h, depthOrArrayLayers: 1 })
    G.device.queue.submit([enc.finish()])
    await buffer.mapAsync(GPUMapMode.READ)
    const data = new Uint8Array(buffer.getMappedRange().slice(0))
    buffer.unmap()
    buffer.destroy()
    return { w, h, bytesPerRow, format: trec.format, data }
  }
  G.readBuffer = async (bufId) => G.shadow.get(bufId)
})()

// Minimal React DevTools hook: records every fiber root (react-dom and r3f's reconciler)
// so uniform nodes can be traced back to the component state that owns them.
;(() => {
  if (window.__REACT_DEVTOOLS_GLOBAL_HOOK__) return
  const roots = new Set()
  window.__prismFiberRoots = roots
  let n = 0
  window.__REACT_DEVTOOLS_GLOBAL_HOOK__ = {
    renderers: new Map(),
    supportsFiber: true,
    isDisabled: false,
    inject(r) { const id = ++n; this.renderers.set(id, r); return id },
    onCommitFiberRoot(id, root) { roots.add(root) },
    onCommitFiberUnmount() {},
    onPostCommitFiberRoot() {},
    onScheduleFiberRoot() {},
    checkDCE() {},
  }
})()

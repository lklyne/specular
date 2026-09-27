// One GPUDevice shared by every `brush` stroke in this renderer. Each stroke
// owns a webgpu-brush painting on its own canvas; sharing the device means one
// queue and one set of adapter limits instead of a device per stroke.

import { useEffect, useState } from 'react'
import type { createBrush } from 'webgpu-brush'

type BrushOptions = NonNullable<Parameters<typeof createBrush>[0]>
export type NaturalBrushGpu = {
  device: NonNullable<BrushOptions['device']>
  adapter: NonNullable<BrushOptions['adapter']>
}

type GpuNavigator = Navigator & {
  gpu?: {
    requestAdapter(): Promise<{
      requestDevice(): Promise<NaturalBrushGpu['device']>
    } | null>
  }
}

let gpuPromise: Promise<NaturalBrushGpu | null> | null = null

function requestNaturalBrushGpu(): Promise<NaturalBrushGpu | null> {
  gpuPromise ??= (async () => {
    const gpu = (navigator as GpuNavigator).gpu
    if (!gpu) return null
    try {
      const adapter = await gpu.requestAdapter()
      if (!adapter) return null
      const device = await adapter.requestDevice()
      // A lost device can't be reused; the next mount requests a fresh one.
      void device.lost.then(() => {
        gpuPromise = null
      })
      return { device, adapter }
    } catch (err) {
      console.warn('[natural-brush] WebGPU unavailable', err)
      return null
    }
  })()
  return gpuPromise
}

/**
 * `undefined` while the device is being requested, `null` when WebGPU is
 * unavailable (callers fall back to the pen), the device once it's ready.
 * Requests nothing until `enabled`, so canvases without brush strokes never
 * touch WebGPU.
 */
export function useNaturalBrushGpu(enabled: boolean): NaturalBrushGpu | null | undefined {
  const [gpu, setGpu] = useState<NaturalBrushGpu | null | undefined>(undefined)
  useEffect(() => {
    if (!enabled) return
    let cancelled = false
    void requestNaturalBrushGpu().then((result) => {
      if (!cancelled) setGpu(result)
    })
    return () => {
      cancelled = true
    }
  }, [enabled])
  return gpu
}

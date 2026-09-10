import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import type { OsrLabElectronAPI } from '../../shared/electron-api/osr-lab'
import type { OsrLabBenchmarkResult, OsrLabConfig, OsrLabMainStats, OsrLabPageInfo } from '../../shared/osr-lab'
import { boundsOf, DEFAULT_OSR_LAB_CONFIG, fitCamera, layoutPages } from '../../shared/osr-lab'
import { LabCanvas, type LabCanvasHandle, type LabPageLayout } from './LabCanvas'
import { LabControls } from './LabControls'
import { LabHud } from './LabHud'
import { measureRefreshMs, runLabBenchmark } from './labBenchmark'
import { clearLabFrames, useLabFrames } from './useLabFrames'

export default function App({ api }: { api: OsrLabElectronAPI }) {
  const [config, setConfig] = useState<OsrLabConfig>(DEFAULT_OSR_LAB_CONFIG)
  const [activeConfig, setActiveConfig] = useState<OsrLabConfig | null>(null)
  const [pages, setPages] = useState<OsrLabPageInfo[]>([])
  const [enteredPageId, setEnteredPageId] = useState<string | null>(null)
  const [cursor, setCursor] = useState('default')
  const [stats, setStats] = useState<OsrLabMainStats | null>(null)
  const [busy, setBusy] = useState(false)
  const [status, setStatus] = useState('Idle. Configure pages and press Start.')
  const [result, setResult] = useState('')
  const [capture, setCapture] = useState<string | null>(null)
  const [, forceHud] = useState(0)
  const handleRef = useRef<LabCanvasHandle | null>(null)

  const frames = useLabFrames(() => handleRef.current?.requestDraw())

  const layout = useMemo<LabPageLayout[]>(() => {
    if (!activeConfig) return []
    const rects = layoutPages(pages.length, activeConfig.pageWidth, activeConfig.pageHeight)
    return pages.map((page, index) => ({ id: page.id, rect: rects[index] }))
  }, [pages, activeConfig])

  useEffect(() => api.onPagesChanged(setPages), [api])
  useEffect(
    () =>
      api.onCursorChanged((payload) => {
        if (payload.pageId === enteredPageId) setCursor(payload.type === 'pointer' ? 'default' : payload.type)
      }),
    [api, enteredPageId],
  )

  useEffect(() => {
    if (!activeConfig) return
    const timer = setInterval(() => {
      void api.getStats().then(setStats)
      forceHud((n) => n + 1)
    }, 1000)
    return () => clearInterval(timer)
  }, [api, activeConfig])

  const fit = useCallback(() => {
    const handle = handleRef.current
    if (!handle || layout.length === 0) return
    handle.setCamera(fitCamera(boundsOf(layout.map((page) => page.rect)), handle.viewportSize()))
  }, [layout])

  const enterPage = useCallback(
    (pageId: string | null) => {
      setEnteredPageId(pageId)
      setCursor('default')
      api.setEnteredPage(pageId)
    },
    [api],
  )

  const start = async () => {
    setBusy(true)
    setStatus('Creating offscreen pages…')
    try {
      clearLabFrames(frames)
      setEnteredPageId(null)
      const created = await api.configure(config)
      setActiveConfig(config)
      setPages(created)
      setStatus(`${created.length} offscreen page(s) running in ${config.mode} mode.`)
      setTimeout(fit, 50)
    } catch (error) {
      setStatus(`Start failed: ${String(error)}`)
    } finally {
      setBusy(false)
    }
  }

  const stop = async () => {
    setBusy(true)
    try {
      await api.teardown()
      clearLabFrames(frames)
      setPages([])
      setActiveConfig(null)
      setEnteredPageId(null)
      setStatus('Torn down.')
    } finally {
      setBusy(false)
    }
  }

  const benchmark = async (withTrace: boolean) => {
    const handle = handleRef.current
    if (!handle || !activeConfig) return
    setBusy(true)
    setStatus('Measuring refresh rate…')
    try {
      const refreshMs = await measureRefreshMs()
      if (withTrace) await api.traceStart()
      setStatus('Running benchmark…')
      const phases = await runLabBenchmark({
        getCamera: () => handle.cameraRef.current,
        setCamera: handle.setCamera,
        viewportCenter: () => {
          const size = handle.viewportSize()
          return { x: size.width / 2, y: size.height / 2 }
        },
        framesReceived: () => frames.totalFrames,
        refreshMs,
      })
      const tracePath = withTrace ? await api.traceStop() : null
      const mainStats = await api.getStats()
      const summary: OsrLabBenchmarkResult = {
        config: activeConfig,
        pageCount: pages.length,
        refreshMs,
        phases,
        tracePath,
        mainStats,
      }
      setResult(JSON.stringify(summary, null, 2))
      setStatus(tracePath ? `Benchmark done. Trace: ${tracePath}` : 'Benchmark done.')
    } catch (error) {
      setStatus(`Benchmark failed: ${String(error)}`)
    } finally {
      setBusy(false)
    }
  }

  const captureEntered = async () => {
    if (!enteredPageId) return
    setBusy(true)
    try {
      const started = performance.now()
      const dataUrl = await api.capturePage(enteredPageId)
      setCapture(dataUrl)
      setStatus(
        dataUrl
          ? `capturePage() returned a frame in ${(performance.now() - started).toFixed(0)} ms.`
          : 'capturePage() returned an empty image.',
      )
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="lab">
      <LabControls
        config={config}
        onConfigChange={setConfig}
        running={activeConfig !== null}
        busy={busy}
        onStart={() => void start()}
        onStop={() => void stop()}
        onFit={fit}
        onBenchmark={(withTrace) => void benchmark(withTrace)}
        onCapture={() => void captureEntered()}
        onDevTools={() => {
          if (enteredPageId) void api.openDevTools(enteredPageId)
        }}
        enteredPageId={enteredPageId}
        result={result}
        capture={capture}
        status={status}
      />
      <div className="lab-stage">
        {activeConfig && (
          <LabCanvas
            api={api}
            config={activeConfig}
            pages={layout}
            frames={frames}
            enteredPageId={enteredPageId}
            onEnterPage={enterPage}
            cursor={cursor}
            handleRef={handleRef}
          />
        )}
        <LabHud
          pages={pages}
          frames={frames}
          stats={stats}
          drawStats={handleRef.current?.drawStats ?? null}
          enteredPageId={enteredPageId}
          cursor={cursor}
        />
      </div>
    </div>
  )
}

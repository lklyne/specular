import type { OsrLabConfig } from '../../shared/osr-lab'
import { DEFAULT_OSR_LAB_URLS } from '../../shared/osr-lab'

interface LabControlsProps {
  config: OsrLabConfig
  onConfigChange: (config: OsrLabConfig) => void
  running: boolean
  busy: boolean
  onStart: () => void
  onStop: () => void
  onFit: () => void
  onBenchmark: (withTrace: boolean) => void
  onCapture: () => void
  onDevTools: () => void
  enteredPageId: string | null
  result: string
  capture: string | null
  status: string
}

export function LabControls({
  config,
  onConfigChange,
  running,
  busy,
  onStart,
  onStop,
  onFit,
  onBenchmark,
  onCapture,
  onDevTools,
  enteredPageId,
  result,
  capture,
  status,
}: LabControlsProps) {
  const update = (patch: Partial<OsrLabConfig>) => onConfigChange({ ...config, ...patch })
  const setCount = (count: number) => {
    const urls: string[] = []
    for (let i = 0; i < count; i++) urls.push(DEFAULT_OSR_LAB_URLS[i % DEFAULT_OSR_LAB_URLS.length])
    update({ urls })
  }
  return (
    <div className="lab-side">
      <h1>Offscreen Rendering Lab</h1>
      <div>
        Click a page to enter it; Esc or click empty canvas to leave. Wheel pans, ⌘/Ctrl+wheel zooms, Alt-drag or
        middle-drag pans. Inside an entered page, pointer, wheel and keys go to the page.
      </div>

      <h2>Pages</h2>
      <label>
        Mode
        <select
          value={config.mode}
          onChange={(event) => update({ mode: event.target.value as OsrLabConfig['mode'] })}
        >
          <option value="shared-texture">GPU shared texture</option>
          <option value="bitmap-jpeg">CPU bitmap → JPEG (baseline)</option>
        </select>
      </label>
      <label>
        Count
        <input
          type="number"
          min={1}
          max={40}
          value={config.urls.length}
          onChange={(event) => setCount(Math.max(1, Math.min(40, Number(event.target.value) || 1)))}
        />
      </label>
      <label>
        Page width
        <input type="number" value={config.pageWidth} onChange={(e) => update({ pageWidth: Number(e.target.value) || 1280 })} />
      </label>
      <label>
        Page height
        <input type="number" value={config.pageHeight} onChange={(e) => update({ pageHeight: Number(e.target.value) || 800 })} />
      </label>
      <label>
        Device scale
        <select
          value={config.deviceScaleFactor}
          onChange={(event) => update({ deviceScaleFactor: Number(event.target.value) })}
        >
          <option value={1}>1×</option>
          <option value={2}>2×</option>
        </select>
      </label>
      <label>
        Frame rate cap (0 = none)
        <input type="number" min={0} max={240} value={config.frameRateCap} onChange={(e) => update({ frameRateCap: Number(e.target.value) || 0 })} />
      </label>
      <label>
        Stop painting off-screen pages
        <input
          type="checkbox"
          checked={config.stopPaintingOffscreen}
          onChange={(event) => update({ stopPaintingOffscreen: event.target.checked })}
        />
      </label>
      <textarea
        value={config.urls.join('\n')}
        onChange={(event) => update({ urls: event.target.value.split('\n').map((line) => line.trim()).filter(Boolean) })}
        spellCheck={false}
      />

      <h2>Input</h2>
      <label>
        Pointer / wheel
        <select
          value={config.pointerTransport}
          onChange={(event) => update({ pointerTransport: event.target.value as OsrLabConfig['pointerTransport'] })}
        >
          <option value="send-input-event">sendInputEvent</option>
          <option value="cdp">CDP Input.dispatchMouseEvent</option>
        </select>
      </label>
      <label>
        Keyboard
        <select
          value={config.keyboardTransport}
          onChange={(event) => update({ keyboardTransport: event.target.value as OsrLabConfig['keyboardTransport'] })}
        >
          <option value="cdp">CDP Input.dispatchKeyEvent</option>
          <option value="send-input-event">sendInputEvent</option>
        </select>
      </label>

      <div className="lab-buttons">
        <button className="primary" onClick={() => onStart()} disabled={busy}>
          {running ? 'Restart pages' : 'Start pages'}
        </button>
        <button onClick={() => onStop()} disabled={!running || busy}>
          Tear down
        </button>
        <button onClick={() => onFit()} disabled={!running}>
          Fit all
        </button>
      </div>

      <h2>Measure</h2>
      <div className="lab-buttons">
        <button onClick={() => onBenchmark(false)} disabled={!running || busy}>
          Run benchmark
        </button>
        <button onClick={() => onBenchmark(true)} disabled={!running || busy}>
          Benchmark + Chromium trace
        </button>
        <button onClick={() => onCapture()} disabled={!enteredPageId || busy}>
          capturePage() entered
        </button>
        <button onClick={() => onDevTools()} disabled={!enteredPageId}>
          DevTools for entered
        </button>
      </div>
      <div>{status}</div>
      {capture && <img className="lab-capture" src={capture} alt="capturePage result" />}
      <textarea className="lab-result" readOnly value={result} spellCheck={false} />
    </div>
  )
}

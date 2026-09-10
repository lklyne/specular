import type { OsrLabMainStats, OsrLabPageInfo } from '../../shared/osr-lab'
import type { LabFrameStore } from './useLabFrames'

interface LabHudProps {
  pages: OsrLabPageInfo[]
  frames: LabFrameStore
  stats: OsrLabMainStats | null
  drawStats: { drawMsMean: number | null; drawsPerSecond: number } | null
  enteredPageId: string | null
  cursor: string
}

function fmt(value: number | null | undefined, digits = 1): string {
  if (value === null || value === undefined || Number.isNaN(value)) return '–'
  return value.toFixed(digits)
}

export function LabHud({ pages, frames, stats, drawStats, enteredPageId, cursor }: LabHudProps) {
  const byId = new Map(stats?.pages.map((page) => [page.pageId, page]) ?? [])
  const totalDrops = stats?.pages.reduce((sum, page) => sum + page.framesDroppedForPoolPressure, 0) ?? 0
  const totalFailures = stats?.pages.reduce((sum, page) => sum + page.sendFailures, 0) ?? 0
  const noTexture = stats?.pages.reduce((sum, page) => sum + page.framesWithoutTexture, 0) ?? 0
  return (
    <div className="lab-hud">
      <div>
        draw <b>{drawStats?.drawsPerSecond ?? 0}</b>/s · {fmt(drawStats?.drawMsMean, 2)} ms · frames in{' '}
        <b>{frames.framesPerSecond}</b>/s · copy {fmt(frames.copyMsMean, 2)} ms · total {frames.totalFrames}
      </div>
      <div>
        gpu {fmt(stats?.gpuWorkingSetMb, 0)} MB · page renderers {fmt(stats?.pageRenderersWorkingSetMb, 0)} MB
        · cdp attached {stats?.cdpAttached ?? 0}
      </div>
      <div>
        entered <b>{enteredPageId ?? 'none'}</b> · cursor {cursor}
        {totalDrops > 0 && <span className="warn"> · pool drops {totalDrops}</span>}
        {totalFailures > 0 && <span className="bad"> · send failures {totalFailures}</span>}
        {noTexture > 0 && <span className="bad"> · paints without texture {noTexture}</span>}
      </div>
      <table>
        <thead>
          <tr>
            <th>page</th>
            <th>frames</th>
            <th>popup</th>
            <th>tex</th>
            <th>out/max</th>
            <th>release ms</th>
            <th>enc ms</th>
            <th>cursor</th>
            <th>paint</th>
          </tr>
        </thead>
        <tbody>
          {pages.map((page) => {
            const s = byId.get(page.id)
            return (
              <tr key={page.id}>
                <td>
                  {page.id}
                  {page.loading ? ' …' : ''}
                </td>
                <td>{s?.framesReceived ?? 0}</td>
                <td className={s && s.popupFrames > 0 ? 'ok' : ''}>{s?.popupFrames ?? 0}</td>
                <td>
                  {s?.lastTextureWidth ?? 0}×{s?.lastTextureHeight ?? 0}
                </td>
                <td className={s && s.maxOutstandingTextures >= 5 ? 'warn' : ''}>
                  {s?.outstandingTextures ?? 0}/{s?.maxOutstandingTextures ?? 0}
                </td>
                <td>{fmt(s?.releaseLatencyMs)}</td>
                <td>{fmt(s?.lastEncodeMs)}</td>
                <td>
                  {s?.cursorChanges ?? 0} {s?.lastCursor ?? ''}
                </td>
                <td className={s && !s.painting ? 'warn' : ''}>{s?.painting === false ? 'off' : 'on'}</td>
              </tr>
            )
          })}
        </tbody>
      </table>
    </div>
  )
}

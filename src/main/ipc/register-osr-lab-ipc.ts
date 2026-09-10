import { ipcMain } from 'electron'
import { ipcChannels } from '../../shared/ipc-contract'
import type {
  OsrLabConfig,
  OsrLabKeyPayload,
  OsrLabMainStats,
  OsrLabPageInfo,
  OsrLabPointerPayload,
  OsrLabWheelPayload,
} from '../../shared/osr-lab'
import {
  captureOsrLabPage,
  configureOsrLab,
  forwardOsrLabKey,
  forwardOsrLabPointer,
  forwardOsrLabWheel,
  insertOsrLabText,
  openOsrLabDevTools,
  sampleOsrLabStats,
  setOsrLabEnteredPage,
  setOsrLabVisiblePages,
  teardownOsrLab,
} from '../osr-lab/osr-lab-pages'
import { startPerfTrace, stopPerfTrace } from '../perf-trace'

export function registerOsrLabIpc(): void {
  ipcMain.handle(
    ipcChannels.osrLabConfigure,
    async (_event, config: OsrLabConfig): Promise<OsrLabPageInfo[]> => configureOsrLab(config),
  )
  ipcMain.handle(ipcChannels.osrLabTeardown, async (): Promise<void> => teardownOsrLab())
  ipcMain.handle(ipcChannels.osrLabStats, async (): Promise<OsrLabMainStats> => sampleOsrLabStats())
  ipcMain.handle(
    ipcChannels.osrLabCapture,
    async (_event, pageId: string): Promise<string | null> => captureOsrLabPage(pageId),
  )
  ipcMain.handle(ipcChannels.osrLabOpenDevtools, async (_event, pageId: string): Promise<void> => {
    openOsrLabDevTools(pageId)
  })
  ipcMain.handle(ipcChannels.osrLabTraceStart, async (): Promise<void> => {
    await startPerfTrace({ revealOnAutoStop: false })
  })
  ipcMain.handle(
    ipcChannels.osrLabTraceStop,
    async (): Promise<string | null> => stopPerfTrace({ reveal: false }),
  )
  ipcMain.on(ipcChannels.osrLabPointer, (_event, payload: OsrLabPointerPayload) => {
    forwardOsrLabPointer(payload)
  })
  ipcMain.on(ipcChannels.osrLabWheel, (_event, payload: OsrLabWheelPayload) => {
    forwardOsrLabWheel(payload)
  })
  ipcMain.on(ipcChannels.osrLabKey, (_event, payload: OsrLabKeyPayload) => {
    forwardOsrLabKey(payload)
  })
  ipcMain.on(ipcChannels.osrLabInsertText, (_event, payload: { pageId: string; text: string }) => {
    insertOsrLabText(payload.pageId, payload.text)
  })
  ipcMain.on(ipcChannels.osrLabSetEntered, (_event, payload: { pageId: string | null }) => {
    setOsrLabEnteredPage(payload.pageId)
  })
  ipcMain.on(ipcChannels.osrLabSetVisible, (_event, payload: { pageIds: string[] }) => {
    setOsrLabVisiblePages(payload.pageIds)
  })
}

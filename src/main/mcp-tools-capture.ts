// ---------------------------------------------------------------------------
// MCP tools — recording video and printing pages to disk
// ---------------------------------------------------------------------------

import { asText, callApp, type ToolHandler } from './mcp-server'

export const captureToolHandlers: Record<string, ToolHandler> = {
  start_recording: async (args) =>
    asText(
      await callApp('/recording/start', {
        method: 'POST',
        body: JSON.stringify({
          pageId: args.page_id,
          outputPath: args.output_path,
          fps: args.fps,
          quality: args.quality,
        }),
      }),
    ),

  stop_recording: async () => asText(await callApp('/recording/stop', { method: 'POST' })),

  get_recording_status: async () => asText(await callApp('/recording/status')),

  trim_recording: async (args) =>
    asText(
      await callApp('/recording/trim', {
        method: 'POST',
        body: JSON.stringify({
          inputPath: args.input_path,
          outputPath: args.output_path,
          minIdleMs: args.min_idle_ms,
          idleSpeedFactor: args.idle_speed_factor,
        }),
      }),
    ),

  print_pdf: async (args) => {
    const pageId = args.page_id as string
    const outputPath = (args.output_path as string | undefined) ?? `./${pageId}.pdf`
    return asText(
      await callApp(`/pages/${encodeURIComponent(pageId)}/print-pdf`, {
        method: 'POST',
        body: JSON.stringify({
          outputPath,
          landscape: args.landscape ?? false,
          pageSize: args.page_size,
        }),
      }),
    )
  },
}

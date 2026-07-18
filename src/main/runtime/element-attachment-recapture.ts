let recaptureEntity: ((entityId: string) => void) | null = null

/** Register the derived attachment enricher without making reverse-sync import
 * its implementation (which depends on autosave and therefore observers). */
export function registerElementAttachmentRecapture(
  capture: (entityId: string) => void,
): void {
  recaptureEntity = capture
}

export function recaptureElementAttachments(entityIds: readonly string[]): void {
  if (!recaptureEntity) return
  for (const entityId of entityIds) recaptureEntity(entityId)
}

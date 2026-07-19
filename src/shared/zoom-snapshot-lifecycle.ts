export function snapshotCaptureStillValid({
  captureLeaseAtStart,
  currentCaptureLease,
  signatureAtStart,
  currentSignature,
}: {
  captureLeaseAtStart: number
  currentCaptureLease: number
  signatureAtStart: string
  currentSignature: string
}): boolean {
  return (
    captureLeaseAtStart === currentCaptureLease &&
    signatureAtStart === currentSignature
  )
}

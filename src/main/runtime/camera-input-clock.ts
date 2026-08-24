/**
 * Wall-clock of the last camera change (user input or tween frame). Deferred
 * work that would block the main thread — the snapshot JPEG encodes — polls
 * this to yield while the camera is moving, so a pan tick never queues behind
 * an encode and lands as a visible jump.
 */
let lastCameraInputAt = 0

export function markCameraInput(): void {
  lastCameraInputAt = performance.now()
}

export function msSinceCameraInput(): number {
  return performance.now() - lastCameraInputAt
}

/**
 * Auth seam for the WebSocket upgrade. Step 3 (ADR 0018 §4, capability links)
 * replaces this with a D1 grant-row lookup that maps a connection token to a
 * doc id + scope. Until then it allows every connection so steps 1–2 can prove
 * sync and persistence without the grant layer.
 */
export function verifyConnectionToken(_request: Request): boolean {
  return true;
}

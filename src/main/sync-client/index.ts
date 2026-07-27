/**
 * Headless cloud-sync peer — the pure-node client shared by the `specular
 * connect` CLI verb and the cloud-sync integration test. No electron imports,
 * so it bundles into the standalone CLI and loads outside the app process.
 */

export { SyncClientSession } from './client'
export type { HtmlEntityPlacement, HtmlEntityResult } from './client'
export {
  parseShareLink,
  redeemLink,
  uploadAsset,
  type ParsedShareLink,
  type ConnectionToken,
} from './share-link'

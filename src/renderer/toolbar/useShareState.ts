import { useCallback, useEffect, useState } from 'react'
import type { ShareScope, ShareStateData } from '../../shared/share'
import { toolbarApi } from './toolbarApi'

/**
 * Owns the share popover's data and the async actions behind it. The dev flag
 * is read once on mount (ADR 0018 §4b — the whole surface stays hidden when
 * off); every action refreshes the snapshot so the links list and status stay
 * current without a broadcast channel.
 */
export function useShareState() {
  const [state, setState] = useState<ShareStateData | null>(null)
  const [busy, setBusy] = useState(false)

  const refresh = useCallback(async () => {
    setState(await toolbarApi.shareState())
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const copyLink = useCallback(
    async (scope: ShareScope): Promise<string | null> => {
      setBusy(true)
      try {
        const result = await toolbarApi.shareCopyLink(scope)
        await refresh()
        return result.ok ? result.value.url : null
      } finally {
        setBusy(false)
      }
    },
    [refresh],
  )

  const join = useCallback(
    async (link: string): Promise<string | null> => {
      setBusy(true)
      try {
        const result = await toolbarApi.shareJoin(link)
        await refresh()
        return result.ok ? null : result.error
      } finally {
        setBusy(false)
      }
    },
    [refresh],
  )

  const resetLink = useCallback(
    async (grantId: string) => {
      await toolbarApi.shareResetLink(grantId)
      await refresh()
    },
    [refresh],
  )

  const revokeLink = useCallback(
    async (grantId: string) => {
      await toolbarApi.shareRevokeLink(grantId)
      await refresh()
    },
    [refresh],
  )

  return {
    /** Whether the dev flag is on — the caller renders nothing when false. */
    enabled: state?.enabled ?? false,
    state,
    busy,
    refresh,
    copyLink,
    /** Resolves to an error message, or null on success. */
    join,
    resetLink,
    revokeLink,
  }
}

export interface CallActionsSync {
  lastUpdatedAt: string | null
  ruleCount: number
  elapsedMinutes: number | null
}

export interface SyncStatusResponse {
  ok: boolean
  callActionsSync: CallActionsSync
}

export async function fetchSyncStatus(): Promise<SyncStatusResponse> {
  const response = await fetch("/api/sync-status", {
    method: "GET",
    cache: "no-store",
  })

  if (!response.ok) {
    throw new Error(`Failed to fetch sync status: ${response.statusText}`)
  }

  return (await response.json()) as SyncStatusResponse
}

"use client"

import { useCallback, useEffect, useState } from "react"
import { AlertTriangle, RefreshCw } from "lucide-react"

import { fetchSyncStatus, type CallActionsSync } from "@/lib/api/sync-status"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

const POLL_INTERVAL_MS = 30_000
const DELAY_THRESHOLD_MINUTES = 10

type SyncState = "checking" | "synced" | "delayed" | "unsynced" | "error"

function formatTimestamp(value: string | null): string {
  if (!value) {
    return "データなし"
  }
  const parsed = Date.parse(value)
  if (Number.isNaN(parsed)) {
    return "データなし"
  }
  return new Date(parsed).toLocaleString("ja-JP")
}

export function CallActionsSyncWidget() {
  const [status, setStatus] = useState<CallActionsSync | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const loadStatus = useCallback(async () => {
    setError(null)
    try {
      const response = await fetchSyncStatus()
      setStatus(response.callActionsSync)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load sync status")
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void loadStatus()
    const timer = setInterval(() => {
      void loadStatus()
    }, POLL_INTERVAL_MS)
    return () => clearInterval(timer)
  }, [loadStatus])

  const isDelayed =
    status?.elapsedMinutes !== null &&
    status?.elapsedMinutes !== undefined &&
    status.elapsedMinutes > DELAY_THRESHOLD_MINUTES

  let syncState: SyncState = "checking"
  let syncLabel = "同期確認中"
  let syncClassName = "bg-slate-100 text-slate-700 border-slate-300"

  if (error) {
    syncState = "error"
    syncLabel = "同期確認エラー"
    syncClassName = "bg-red-100 text-red-700 border-red-300"
  } else if (!loading && status?.lastUpdatedAt == null) {
    syncState = "unsynced"
    syncLabel = "未同期"
    syncClassName = "bg-gray-100 text-gray-700 border-gray-300"
  } else if (isDelayed) {
    syncState = "delayed"
    syncLabel = "同期遅延"
    syncClassName = "bg-yellow-100 text-yellow-800 border-yellow-300"
  } else if (!loading && status) {
    syncState = "synced"
    syncLabel = "同期済み"
    syncClassName = "bg-green-100 text-green-700 border-green-300"
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between gap-2 space-y-0">
        <div>
          <CardTitle className="text-base">着信アクション同期状態</CardTitle>
          <CardDescription>Backend / call_action_rules</CardDescription>
          <div
            className={`mt-2 inline-flex items-center rounded-full border px-2 py-0.5 text-xs font-medium ${syncClassName}`}
          >
            {syncLabel}
          </div>
        </div>
        <Button variant="outline" size="sm" onClick={() => void loadStatus()}>
          <RefreshCw className="h-4 w-4" />
          更新
        </Button>
      </CardHeader>
      <CardContent className="space-y-3">
        {loading ? (
          <p className="text-sm text-muted-foreground">読み込み中...</p>
        ) : null}
        {error ? (
          <p className="text-sm text-destructive">{error}</p>
        ) : null}

        {status ? (
          <>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">アクティブルール数</span>
              <span className="font-medium">{status.ruleCount} 件</span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">最終更新</span>
              <span className="font-medium">{formatTimestamp(status.lastUpdatedAt)}</span>
            </div>
            <div className="flex items-center justify-between text-sm">
              <span className="text-muted-foreground">経過時間</span>
              <span className={isDelayed ? "font-medium text-yellow-600" : "font-medium"}>
                {status.elapsedMinutes === null ? "-" : `${status.elapsedMinutes} 分前`}
              </span>
            </div>
            {syncState === "delayed" ? (
              <div className="flex items-center gap-2 rounded-md border border-yellow-200 bg-yellow-50 p-2 text-sm text-yellow-800">
                <AlertTriangle className="h-4 w-4" />
                <span>10分以上更新がありません</span>
              </div>
            ) : null}
          </>
        ) : null}
      </CardContent>
    </Card>
  )
}

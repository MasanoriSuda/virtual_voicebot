"use client"

import { useCallback, useEffect, useState } from "react"
import { RefreshCw } from "lucide-react"

import {
  fetchLocalServicesStatus,
  type LocalServicesStatusResponse,
  type ServiceStatus,
} from "@/lib/api/local-services-status"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

const POLL_INTERVAL_MS = 30_000
const SERVICE_KEYS = ["asr", "llm", "tts"] as const

type ServiceKey = (typeof SERVICE_KEYS)[number]

const SERVICE_LABELS: Record<ServiceKey, string> = {
  asr: "ASR (Whisper)",
  llm: "LLM (Ollama)",
  tts: "TTS (VoiceVox)",
}

const STATUS_STYLE: Record<ServiceStatus, { label: string; className: string }> = {
  ok: { label: "正常", className: "bg-green-100 text-green-700 border-green-300" },
  error: { label: "異常", className: "bg-red-100 text-red-700 border-red-300" },
  disabled: { label: "無効", className: "bg-gray-100 text-gray-500 border-gray-300" },
}

export function LocalServicesStatusWidget() {
  const [data, setData] = useState<LocalServicesStatusResponse | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  const loadStatus = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const response = await fetchLocalServicesStatus()
      setData(response)
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load local services status")
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

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between gap-2 space-y-0">
        <div>
          <CardTitle className="text-base">ローカルサービス状態</CardTitle>
          <CardDescription>ASR / LLM / TTS</CardDescription>
        </div>
        <Button variant="outline" size="sm" onClick={() => void loadStatus()} disabled={loading}>
          <RefreshCw className="h-4 w-4" aria-hidden="true" />
          更新
        </Button>
      </CardHeader>
      <CardContent className="space-y-3">
        {loading ? <p className="text-sm text-muted-foreground">読み込み中...</p> : null}
        {error ? <p className="text-sm text-destructive">{error}</p> : null}

        {data?.localServices
          ? SERVICE_KEYS.map((key) => {
              const entry = data.localServices[key]
              if (!entry) {
                return null
              }
              const style = STATUS_STYLE[entry.status] ?? {
                label: String(entry.status),
                className: "bg-yellow-100 text-yellow-700 border-yellow-300",
              }
              return (
                <div key={key} className="flex items-center justify-between gap-3 text-sm">
                  <div className="min-w-0">
                    <div className="font-medium">{SERVICE_LABELS[key]}</div>
                    {entry.displayUrl ? (
                      <div className="truncate text-xs text-muted-foreground">{entry.displayUrl}</div>
                    ) : null}
                  </div>
                  <span
                    className={`inline-flex shrink-0 items-center rounded-full border px-2 py-0.5 text-xs font-medium ${style.className}`}
                  >
                    {style.label}
                  </span>
                </div>
              )
            })
          : null}
      </CardContent>
    </Card>
  )
}

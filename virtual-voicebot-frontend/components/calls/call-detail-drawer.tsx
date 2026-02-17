"use client"

import { useEffect, useMemo, useState } from "react"
import { Copy } from "lucide-react"

import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { AudioPlayer } from "@/components/calls/audio-player"
import type { CallRecord } from "@/lib/mock-data"
import type { CallDetail, Utterance } from "@/lib/types"
import { cn } from "@/lib/utils"

interface CallDetailDrawerProps {
  call: CallRecord | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function CallDetailDrawer({ call, open, onOpenChange }: CallDetailDrawerProps) {
  const [callDetail, setCallDetail] = useState<CallDetail | null>(null)
  const [isLoading, setIsLoading] = useState(false)

  useEffect(() => {
    if (!open || !call) {
      setCallDetail(null)
      setIsLoading(false)
      return
    }

    const abortController = new AbortController()
    const loadDetail = async () => {
      setIsLoading(true)
      try {
        const response = await fetch(`/api/calls?callId=${encodeURIComponent(call.id)}`, {
          signal: abortController.signal,
          cache: "no-store",
        })
        if (!response.ok) {
          throw new Error(`failed to load call detail: ${response.status}`)
        }
        const detail = (await response.json()) as CallDetail
        if (!abortController.signal.aborted) {
          setCallDetail(detail)
        }
      } catch (error) {
        if (!abortController.signal.aborted) {
          console.error("[call-detail-drawer] failed to load detail", error)
          setCallDetail(null)
        }
      } finally {
        if (!abortController.signal.aborted) {
          setIsLoading(false)
        }
      }
    }

    void loadDetail()
    return () => abortController.abort()
  }, [open, call?.id])

  const startedAt = useMemo(() => (call ? formatDateTime(call.startedAt) : ""), [call])
  const duration = useMemo(
    () => (call ? formatDuration(call.durationSec) : ""),
    [call]
  )
  const utterances = callDetail?.utterances ?? []
  const recordingUrl = callDetail?.recordingUrl ?? call?.recordingUrl ?? null
  const summary = useMemo(() => {
    if (callDetail?.summary && callDetail.summary.trim().length > 0) {
      return callDetail.summary
    }
    if (call?.summary && call.summary.trim().length > 0) {
      return call.summary
    }
    return "準備中"
  }, [callDetail?.summary, call?.summary])
  const transcriptText = useMemo(
    () =>
      utterances
        .map((utterance) => `[${speakerLabel(utterance.speaker)}] ${utterance.text}`)
        .join("\n"),
    [utterances],
  )

  const statusLabel = call ? statusToLabel(call.status) : ""
  const statusClass = call ? statusToClass(call.status) : ""

  const handleCopy = (value: string) => {
    if (typeof navigator === "undefined") return
    navigator.clipboard.writeText(value)
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent side="right" className="w-full sm:max-w-[500px]">
        <SheetHeader className="space-y-3">
          <div className="flex items-start justify-between gap-3">
            <div>
              <SheetTitle className="text-xl font-semibold">
                {call ? `${call.fromName} / ${call.from}` : "通話詳細"}
              </SheetTitle>
              <p className="text-sm text-muted-foreground">
                {call ? `${call.to} ・ ${startedAt}` : ""}
              </p>
              <p className="text-xs text-muted-foreground">通話時間: {duration}</p>
            </div>
            {call ? (
              <Badge className={cn("px-3 py-1 text-xs", statusClass)}>{statusLabel}</Badge>
            ) : null}
          </div>
        </SheetHeader>

        <Tabs defaultValue="recording" className="mt-6">
          <TabsList className="grid w-full grid-cols-3">
            <TabsTrigger value="recording">録音</TabsTrigger>
            <TabsTrigger value="transcript">文字起こし</TabsTrigger>
            <TabsTrigger value="summary">要約</TabsTrigger>
          </TabsList>

          <TabsContent value="recording" className="mt-6">
            <AudioPlayer url={recordingUrl} />
          </TabsContent>

          <TabsContent value="transcript" className="mt-6">
            <div className="flex items-center justify-between">
              <p className="text-sm text-muted-foreground">会話ログ</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleCopy(transcriptText)}
                disabled={transcriptText.length === 0}
              >
                <Copy className="mr-2 h-4 w-4" />
                コピー
              </Button>
            </div>
            {isLoading ? (
              <div className="mt-4 rounded-2xl border bg-muted/20 p-4 text-sm text-muted-foreground">
                文字起こしデータを読み込み中です
              </div>
            ) : utterances.length === 0 ? (
              <div className="mt-4 rounded-2xl border bg-muted/20 p-4 text-sm text-muted-foreground">
                文字起こしデータは準備中です
              </div>
            ) : (
              <div className="mt-4 space-y-3 rounded-2xl border bg-card/60 p-4">
                {utterances.map((utterance) => (
                  <TranscriptRow key={utterance.seq} utterance={utterance} />
                ))}
              </div>
            )}
          </TabsContent>

          <TabsContent value="summary" className="mt-6">
            <div className="flex items-center justify-between">
              <p className="text-sm text-muted-foreground">要約</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleCopy(summary)}
              >
                <Copy className="mr-2 h-4 w-4" />
                コピー
              </Button>
            </div>
            <div className="mt-4 rounded-2xl border bg-card/60 p-4 text-sm leading-relaxed">
              {isLoading ? "要約データを読み込み中です" : summary}
            </div>
          </TabsContent>
        </Tabs>
      </SheetContent>
    </Sheet>
  )
}

function TranscriptRow({ utterance }: { utterance: Utterance }) {
  return (
    <div className="rounded-xl border bg-background/80 p-3">
      <div className="mb-1 text-xs text-muted-foreground">
        {speakerLabel(utterance.speaker)} / {formatTranscriptTime(utterance.timestamp)}
      </div>
      <p className="text-sm leading-relaxed">{utterance.text}</p>
    </div>
  )
}

function speakerLabel(speaker: Utterance["speaker"]): string {
  switch (speaker) {
    case "bot":
      return "Bot"
    case "caller":
      return "Caller"
    default:
      return "System"
  }
}

function formatTranscriptTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return "--:--:--"
  }
  return new Intl.DateTimeFormat("ja-JP", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date)
}

function formatDuration(seconds: number) {
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`
}

function formatDateTime(value: string) {
  const date = new Date(value)
  return new Intl.DateTimeFormat("ja-JP", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date)
}

function statusToLabel(status: CallRecord["status"]) {
  switch (status) {
    case "ended":
      return "完了"
    case "missed":
      return "不在"
    case "in_call":
      return "通話中"
    default:
      return "-"
  }
}

function statusToClass(status: CallRecord["status"]) {
  switch (status) {
    case "ended":
      return "bg-emerald-500/15 text-emerald-600 dark:text-emerald-300"
    case "missed":
      return "bg-rose-500/15 text-rose-600 dark:text-rose-300"
    case "in_call":
      return "bg-sky-500/15 text-sky-600 dark:text-sky-300"
    default:
      return "bg-muted text-muted-foreground"
  }
}

"use client"

import { useMemo } from "react"
import { Copy } from "lucide-react"

import { Sheet, SheetContent, SheetHeader, SheetTitle } from "@/components/ui/sheet"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { AudioPlayer } from "@/components/calls/audio-player"
import type { CallRecord } from "@/lib/mock-data"
import { mockTranscript } from "@/lib/mock-data"
import { cn } from "@/lib/utils"

interface CallDetailDrawerProps {
  call: CallRecord | null
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function CallDetailDrawer({ call, open, onOpenChange }: CallDetailDrawerProps) {
  const startedAt = useMemo(() => (call ? formatDateTime(call.startedAt) : ""), [call])
  const duration = useMemo(
    () => (call ? formatDuration(call.durationSec) : ""),
    [call]
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
            <AudioPlayer url={call?.recordingUrl ?? null} />
          </TabsContent>

          <TabsContent value="transcript" className="mt-6">
            <div className="flex items-center justify-between">
              <p className="text-sm text-muted-foreground">会話ログ</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() =>
                  handleCopy(mockTranscript.map((t) => `${t.time} ${t.speaker}: ${t.text}`).join("\n"))
                }
              >
                <Copy className="mr-2 h-4 w-4" />
                コピー
              </Button>
            </div>
            <div className="mt-4 space-y-3">
              {mockTranscript.map((line) => (
                <div
                  key={`${line.time}-${line.speaker}`}
                  className={cn(
                    "rounded-2xl px-4 py-3 text-sm shadow-sm",
                    line.speaker === "A"
                      ? "bg-primary/10 text-primary"
                      : "bg-muted text-foreground"
                  )}
                >
                  <div className="text-[11px] uppercase tracking-[0.2em] text-muted-foreground">
                    {line.time} ・ 話者{line.speaker}
                  </div>
                  <p className="mt-1 leading-relaxed">{line.text}</p>
                </div>
              ))}
            </div>
          </TabsContent>

          <TabsContent value="summary" className="mt-6">
            <div className="flex items-center justify-between">
              <p className="text-sm text-muted-foreground">要約</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => handleCopy(call?.summary ?? "")}
              >
                <Copy className="mr-2 h-4 w-4" />
                コピー
              </Button>
            </div>
            <div className="mt-4 rounded-2xl border bg-card/60 p-4 text-sm leading-relaxed">
              {call?.summary ?? "準備中"}
            </div>
          </TabsContent>
        </Tabs>
      </SheetContent>
    </Sheet>
  )
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

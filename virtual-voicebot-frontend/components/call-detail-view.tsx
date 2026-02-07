"use client"

import { useEffect, useRef, useState, useCallback } from "react"
import type { CallDetail } from "@/lib/types"
import { ChatBubble } from "./chat-bubble"
import { AudioPlayer } from "./audio-player"
import { useCallStream } from "@/lib/hooks/use-call-stream"
import { Button } from "./ui/button"
import { ArrowLeft, Phone, AlertCircle } from "lucide-react"
import { useRouter } from "next/navigation"
import { Badge } from "./ui/badge"
import { Switch } from "./ui/switch"
import { Label } from "./ui/label"
import { Alert, AlertDescription } from "./ui/alert"

interface CallDetailViewProps {
  call: CallDetail
}

export function CallDetailView({ call }: CallDetailViewProps) {
  const router = useRouter()
  const { utterances, summary } = useCallStream(call.id, call.utterances, {
    enabled: call.status === "in_call",
  })
  const scrollRef = useRef<HTMLDivElement>(null)
  const utteranceRefs = useRef<Map<number, HTMLDivElement>>(new Map())

  const [currentTime, setCurrentTime] = useState(0)
  const [isPlaying, setIsPlaying] = useState(false)
  const [autoScroll, setAutoScroll] = useState(true)
  const [highlightedSeq, setHighlightedSeq] = useState<number | null>(null)
  const audioRef = useRef<HTMLAudioElement | null>(null)

  // Auto-scroll to bottom when new utterances arrive (only if not playing audio)
  useEffect(() => {
    if (scrollRef.current && !isPlaying) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight
    }
  }, [utterances, isPlaying])

  useEffect(() => {
    if (!isPlaying || !autoScroll) return

    // Find the utterance that matches current playback time
    const currentUtterance = utterances.find(
      (u) => u.startSec !== undefined && u.endSec !== undefined && currentTime >= u.startSec && currentTime <= u.endSec,
    )

    if (currentUtterance && currentUtterance.seq !== highlightedSeq) {
      setHighlightedSeq(currentUtterance.seq)

      // Scroll to highlighted utterance if auto-scroll is enabled
      if (autoScroll) {
        const element = utteranceRefs.current.get(currentUtterance.seq)
        if (element) {
          element.scrollIntoView({ behavior: "smooth", block: "center" })
        }
      }
    }
  }, [currentTime, utterances, isPlaying, autoScroll, highlightedSeq])

  const handlePlayUtterance = useCallback((startSec: number) => {
    // Find audio element through the ref
    const audioElement = document.querySelector("audio") as HTMLAudioElement
    if (audioElement) {
      audioElement.currentTime = startSec
      audioElement.play()
      setIsPlaying(true)
    }
  }, [])

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}:${secs.toString().padStart(2, "0")}`
  }

  const getStatusBadge = (status: CallDetail["status"]) => {
    const variants = {
      ringing: "default",
      in_call: "default",
      ended: "secondary",
      error: "destructive",
    } as const

    const labels = {
      ringing: "呼出中",
      in_call: "通話中",
      ended: "完了",
      error: "エラー",
    }

    return <Badge variant={variants[status]}>{labels[status]}</Badge>
  }

  const displaySummary = summary || call.summary || "準備中"
  const hasRecording = !!call.recordingUrl

  return (
    <div className="flex flex-col h-screen bg-background">
      {/* Header */}
      <div className="border-b bg-card">
        <div className="container mx-auto px-4 py-4">
          <div className="flex items-center gap-4 mb-4">
            <Button variant="ghost" size="icon" onClick={() => router.push("/calls")}>
              <ArrowLeft className="h-5 w-5" />
            </Button>
            <div className="flex-1">
              <div className="flex items-center gap-3">
                <div className="p-2 bg-primary/10 rounded-full">
                  <Phone className="h-4 w-4 text-primary" />
                </div>
                <div>
                  <h1 className="text-xl font-semibold">
                    {call.from} → {call.to}
                  </h1>
                  <p className="text-sm text-muted-foreground">通話時間: {formatDuration(call.duration)}</p>
                </div>
              </div>
            </div>
            {getStatusBadge(call.status)}
          </div>

          {/* Summary */}
          <div className="bg-muted/50 rounded-lg p-3 mb-4">
            <p className="text-sm font-medium mb-1">要約</p>
            <p className="text-sm text-muted-foreground leading-relaxed">{displaySummary}</p>
          </div>

          {hasRecording ? (
            <div className="space-y-3">
              <AudioPlayer
                recordingUrl={call.recordingUrl!}
                durationSec={call.durationSec ?? call.duration}
                onTimeUpdate={setCurrentTime}
                onPlayingChange={setIsPlaying}
              />

              <div className="flex items-center gap-2">
                <Switch id="auto-scroll" checked={autoScroll} onCheckedChange={setAutoScroll} />
                <Label htmlFor="auto-scroll" className="text-sm text-muted-foreground cursor-pointer">
                  再生中の発話に自動スクロール
                </Label>
              </div>
            </div>
          ) : (
            <Alert>
              <AlertCircle className="h-4 w-4" />
              <AlertDescription>
                {call.status === "in_call" ? "通話録音は終了後に利用可能になります" : "録音準備中です"}
              </AlertDescription>
            </Alert>
          )}
        </div>
      </div>

      {/* Chat Messages */}
      <div ref={scrollRef} className="flex-1 overflow-y-auto">
        <div className="container mx-auto px-4 py-6">
          <div className="flex flex-col gap-4">
            {utterances.length === 0 ? (
              <div className="text-center text-muted-foreground py-12">発話データがありません</div>
            ) : (
              utterances.map((utterance) => (
                <div
                  key={utterance.seq}
                  ref={(el) => {
                    if (el) utteranceRefs.current.set(utterance.seq, el)
                  }}
                >
                  <ChatBubble
                    utterance={utterance}
                    isHighlighted={highlightedSeq === utterance.seq && isPlaying}
                    onPlayUtterance={hasRecording ? handlePlayUtterance : undefined}
                  />
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  )
}

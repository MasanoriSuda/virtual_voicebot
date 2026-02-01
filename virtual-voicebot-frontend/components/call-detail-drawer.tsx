"use client"

import { useEffect, useState, useRef } from "react"
import type { CallDetail, Utterance } from "@/lib/types"
import { getCallDetail } from "@/lib/api"
import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "./ui/sheet"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./ui/tabs"
import { Button } from "./ui/button"
import { Badge } from "./ui/badge"
import { Skeleton } from "./ui/skeleton"
import { ScrollArea } from "./ui/scroll-area"
import {
  Phone,
  Download,
  Copy,
  Check,
  Play,
  Pause,
  Volume2,
  Mic,
  Bot,
  Clock,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { Slider } from "./ui/slider"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/select"

interface CallDetailDrawerProps {
  callId: string | null
  open: boolean
  onClose: () => void
}

export function CallDetailDrawer({ callId, open, onClose }: CallDetailDrawerProps) {
  const [callDetail, setCallDetail] = useState<CallDetail | null>(null)
  const [loading, setLoading] = useState(false)
  const [activeTab, setActiveTab] = useState("recording")
  const [copied, setCopied] = useState(false)

  // Audio player state
  const audioRef = useRef<HTMLAudioElement>(null)
  const [isPlaying, setIsPlaying] = useState(false)
  const [currentTime, setCurrentTime] = useState(0)
  const [duration, setDuration] = useState(0)
  const [playbackRate, setPlaybackRate] = useState("1")

  useEffect(() => {
    if (callId && open) {
      setLoading(true)
      getCallDetail(callId)
        .then((data) => {
          setCallDetail(data)
          setActiveTab("recording")
        })
        .finally(() => setLoading(false))
    } else {
      setCallDetail(null)
      setIsPlaying(false)
      setCurrentTime(0)
    }
  }, [callId, open])

  useEffect(() => {
    const audio = audioRef.current
    if (!audio) return

    const handleTimeUpdate = () => setCurrentTime(audio.currentTime)
    const handleLoadedMetadata = () => setDuration(audio.duration || callDetail?.durationSec || 0)
    const handleEnded = () => setIsPlaying(false)

    audio.addEventListener("timeupdate", handleTimeUpdate)
    audio.addEventListener("loadedmetadata", handleLoadedMetadata)
    audio.addEventListener("ended", handleEnded)

    return () => {
      audio.removeEventListener("timeupdate", handleTimeUpdate)
      audio.removeEventListener("loadedmetadata", handleLoadedMetadata)
      audio.removeEventListener("ended", handleEnded)
    }
  }, [callDetail])

  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.playbackRate = Number.parseFloat(playbackRate)
    }
  }, [playbackRate])

  const togglePlay = () => {
    const audio = audioRef.current
    if (!audio) return

    if (isPlaying) {
      audio.pause()
      setIsPlaying(false)
    } else {
      audio.play()
      setIsPlaying(true)
    }
  }

  const handleSeek = (value: number[]) => {
    const audio = audioRef.current
    if (!audio) return

    audio.currentTime = value[0]
    setCurrentTime(value[0])
  }

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${mins}:${secs.toString().padStart(2, "0")}`
  }

  const formatDateTime = (isoString: string) => {
    const date = new Date(isoString)
    return new Intl.DateTimeFormat("ja-JP", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).format(date)
  }

  const handleCopyTranscript = () => {
    if (!callDetail) return

    const text = callDetail.utterances
      .map((u) => `[${u.speaker === "bot" ? "Bot" : "Caller"}] ${u.text}`)
      .join("\n")

    navigator.clipboard.writeText(text)
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const getStatusBadge = (status: CallDetail["status"]) => {
    const config = {
      active: {
        label: "通話中",
        className: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300",
      },
      completed: {
        label: "完了",
        className: "bg-secondary text-secondary-foreground",
      },
      failed: {
        label: "不在",
        className: "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300",
      },
    }

    return (
      <Badge variant="outline" className={cn("font-normal", config[status].className)}>
        {config[status].label}
      </Badge>
    )
  }

  return (
    <Sheet open={open} onOpenChange={(o) => !o && onClose()}>
      <SheetContent className="w-full sm:max-w-lg p-0 flex flex-col">
        {loading ? (
          <div className="p-6 space-y-4">
            <Skeleton className="h-8 w-3/4" />
            <Skeleton className="h-4 w-1/2" />
            <Skeleton className="h-32 w-full" />
          </div>
        ) : callDetail ? (
          <>
            <SheetHeader className="p-6 pb-4 border-b">
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-3">
                  <div className="p-2 rounded-full bg-primary/10">
                    <Phone className="h-5 w-5 text-primary" />
                  </div>
                  <div>
                    <SheetTitle className="text-left">{callDetail.from}</SheetTitle>
                    <SheetDescription className="text-left">
                      {formatDateTime(callDetail.startTime)} / {formatTime(callDetail.durationSec)}
                    </SheetDescription>
                  </div>
                </div>
                {getStatusBadge(callDetail.status)}
              </div>
            </SheetHeader>

            <Tabs value={activeTab} onValueChange={setActiveTab} className="flex-1 flex flex-col">
              <TabsList className="mx-6 mt-4 grid grid-cols-3">
                <TabsTrigger value="recording">録音</TabsTrigger>
                <TabsTrigger value="transcript">文字起こし</TabsTrigger>
                <TabsTrigger value="summary">要約</TabsTrigger>
              </TabsList>

              <TabsContent value="recording" className="flex-1 p-6 pt-4 data-[state=inactive]:hidden">
                {callDetail.recordingUrl ? (
                  <div className="space-y-6">
                    <audio ref={audioRef} src={callDetail.recordingUrl} preload="metadata" />

                    {/* Waveform placeholder */}
                    <div className="h-24 bg-muted rounded-lg flex items-center justify-center relative overflow-hidden">
                      <div className="absolute inset-0 flex items-center justify-center gap-0.5">
                        {Array.from({ length: 50 }).map((_, i) => (
                          <div
                            key={i}
                            className={cn(
                              "w-1 rounded-full bg-primary/30 transition-all",
                              currentTime > 0 && i < (currentTime / duration) * 50 && "bg-primary"
                            )}
                            style={{
                              height: `${20 + Math.sin(i * 0.5) * 15 + Math.random() * 30}%`,
                            }}
                          />
                        ))}
                      </div>
                    </div>

                    {/* Controls */}
                    <div className="space-y-4">
                      <Slider
                        value={[currentTime]}
                        max={duration || callDetail.durationSec}
                        step={0.1}
                        onValueChange={handleSeek}
                        className="cursor-pointer"
                      />

                      <div className="flex items-center justify-between text-sm text-muted-foreground">
                        <span>{formatTime(currentTime)}</span>
                        <span>{formatTime(duration || callDetail.durationSec)}</span>
                      </div>

                      <div className="flex items-center justify-between">
                        <Button
                          size="lg"
                          onClick={togglePlay}
                          className="rounded-full h-14 w-14"
                        >
                          {isPlaying ? (
                            <Pause className="h-6 w-6" />
                          ) : (
                            <Play className="h-6 w-6 ml-1" />
                          )}
                        </Button>

                        <div className="flex items-center gap-3">
                          <Volume2 className="h-4 w-4 text-muted-foreground" />
                          <Select value={playbackRate} onValueChange={setPlaybackRate}>
                            <SelectTrigger className="w-20 h-9">
                              <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                              <SelectItem value="0.5">0.5x</SelectItem>
                              <SelectItem value="0.75">0.75x</SelectItem>
                              <SelectItem value="1">1x</SelectItem>
                              <SelectItem value="1.25">1.25x</SelectItem>
                              <SelectItem value="1.5">1.5x</SelectItem>
                              <SelectItem value="2">2x</SelectItem>
                            </SelectContent>
                          </Select>
                        </div>

                        <Button variant="outline" size="sm" className="bg-transparent">
                          <Download className="h-4 w-4 mr-2" />
                          保存
                        </Button>
                      </div>
                    </div>
                  </div>
                ) : (
                  <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
                    <Volume2 className="h-10 w-10 mb-2 opacity-50" />
                    <p>録音データがありません</p>
                  </div>
                )}
              </TabsContent>

              <TabsContent value="transcript" className="flex-1 flex flex-col data-[state=inactive]:hidden">
                <div className="px-6 pt-4 pb-2 flex justify-end">
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={handleCopyTranscript}
                    className="bg-transparent"
                  >
                    {copied ? (
                      <>
                        <Check className="h-4 w-4 mr-2" />
                        コピーしました
                      </>
                    ) : (
                      <>
                        <Copy className="h-4 w-4 mr-2" />
                        コピー
                      </>
                    )}
                  </Button>
                </div>

                <ScrollArea className="flex-1 px-6 pb-6">
                  <div className="space-y-4">
                    {callDetail.utterances.length === 0 ? (
                      <div className="flex flex-col items-center justify-center h-48 text-muted-foreground">
                        <Mic className="h-10 w-10 mb-2 opacity-50" />
                        <p>文字起こしデータがありません</p>
                      </div>
                    ) : (
                      callDetail.utterances.map((utterance) => (
                        <TranscriptItem key={utterance.seq} utterance={utterance} />
                      ))
                    )}
                  </div>
                </ScrollArea>
              </TabsContent>

              <TabsContent value="summary" className="flex-1 p-6 pt-4 data-[state=inactive]:hidden">
                <div className="space-y-6">
                  <div>
                    <h4 className="text-sm font-medium mb-2 flex items-center gap-2">
                      <Bot className="h-4 w-4 text-primary" />
                      AI要約
                    </h4>
                    <p className="text-sm text-muted-foreground leading-relaxed">
                      {callDetail.summary}
                    </p>
                  </div>

                  <div>
                    <h4 className="text-sm font-medium mb-2">主要ポイント</h4>
                    <ul className="space-y-2 text-sm text-muted-foreground">
                      <li className="flex items-start gap-2">
                        <span className="w-1.5 h-1.5 rounded-full bg-primary mt-2 shrink-0" />
                        お客様からの問い合わせを受付
                      </li>
                      <li className="flex items-start gap-2">
                        <span className="w-1.5 h-1.5 rounded-full bg-primary mt-2 shrink-0" />
                        必要な情報を確認・案内
                      </li>
                      <li className="flex items-start gap-2">
                        <span className="w-1.5 h-1.5 rounded-full bg-primary mt-2 shrink-0" />
                        対応完了
                      </li>
                    </ul>
                  </div>

                  <div>
                    <h4 className="text-sm font-medium mb-2">アクションアイテム</h4>
                    <p className="text-sm text-muted-foreground">
                      特になし
                    </p>
                  </div>
                </div>
              </TabsContent>
            </Tabs>
          </>
        ) : (
          <div className="flex items-center justify-center h-full text-muted-foreground">
            通話を選択してください
          </div>
        )}
      </SheetContent>
    </Sheet>
  )
}

function TranscriptItem({ utterance }: { utterance: Utterance }) {
  const isBot = utterance.speaker === "bot"

  const formatTimestamp = (isoString: string) => {
    const date = new Date(isoString)
    return new Intl.DateTimeFormat("ja-JP", {
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    }).format(date)
  }

  return (
    <div className={cn("flex gap-3", isBot ? "flex-row" : "flex-row-reverse")}>
      <div
        className={cn(
          "w-8 h-8 rounded-full flex items-center justify-center shrink-0",
          isBot ? "bg-primary/10" : "bg-muted"
        )}
      >
        {isBot ? (
          <Bot className="h-4 w-4 text-primary" />
        ) : (
          <Mic className="h-4 w-4 text-muted-foreground" />
        )}
      </div>

      <div className={cn("flex-1 max-w-[80%]", !isBot && "flex flex-col items-end")}>
        <div
          className={cn(
            "rounded-2xl px-4 py-2",
            isBot
              ? "bg-muted rounded-tl-sm"
              : "bg-primary text-primary-foreground rounded-tr-sm"
          )}
        >
          <p className="text-sm">{utterance.text}</p>
        </div>
        <span className="text-xs text-muted-foreground mt-1 flex items-center gap-1">
          <Clock className="h-3 w-3" />
          {formatTimestamp(utterance.timestamp)}
        </span>
      </div>
    </div>
  )
}

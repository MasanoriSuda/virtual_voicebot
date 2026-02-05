"use client"

import { useEffect, useRef, useState } from "react"
import { Download, Pause, Play } from "lucide-react"

import { Button } from "@/components/ui/button"
import { Slider } from "@/components/ui/slider"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"

interface AudioPlayerProps {
  url: string | null
}

const speeds = ["0.5", "1", "1.5", "2"]

export function AudioPlayer({ url }: AudioPlayerProps) {
  const audioRef = useRef<HTMLAudioElement>(null)
  const [isPlaying, setIsPlaying] = useState(false)
  const [currentTime, setCurrentTime] = useState(0)
  const [duration, setDuration] = useState(0)
  const [rate, setRate] = useState("1")

  useEffect(() => {
    const audio = audioRef.current
    if (!audio) return

    const handleTimeUpdate = () => setCurrentTime(audio.currentTime)
    const handleLoaded = () => setDuration(audio.duration || 0)
    const handleEnded = () => setIsPlaying(false)

    audio.addEventListener("timeupdate", handleTimeUpdate)
    audio.addEventListener("loadedmetadata", handleLoaded)
    audio.addEventListener("ended", handleEnded)

    return () => {
      audio.removeEventListener("timeupdate", handleTimeUpdate)
      audio.removeEventListener("loadedmetadata", handleLoaded)
      audio.removeEventListener("ended", handleEnded)
    }
  }, [url])

  useEffect(() => {
    if (audioRef.current) {
      audioRef.current.playbackRate = Number.parseFloat(rate)
    }
  }, [rate])

  if (!url) {
    return (
      <div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-dashed border-border/60 bg-muted/30 py-10 text-sm text-muted-foreground">
        録音ファイルを準備中です
      </div>
    )
  }

  const togglePlay = async () => {
    const audio = audioRef.current
    if (!audio) return

    if (isPlaying) {
      audio.pause()
      setIsPlaying(false)
      return
    }

    await audio.play()
    setIsPlaying(true)
  }

  const handleSeek = (value: number[]) => {
    const audio = audioRef.current
    if (!audio) return

    audio.currentTime = value[0]
    setCurrentTime(value[0])
  }

  return (
    <div className="space-y-4">
      <audio ref={audioRef} src={url} preload="metadata" />
      <div className="flex flex-wrap items-center gap-4">
        <Button
          onClick={togglePlay}
          size="icon"
          className="h-10 w-10 rounded-full"
        >
          {isPlaying ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
        </Button>
        <div className="flex-1">
          <Slider
            value={[currentTime]}
            max={duration || 1}
            step={1}
            onValueChange={handleSeek}
          />
        </div>
        <div className="text-xs text-muted-foreground tabular-nums">
          {formatTime(currentTime)} / {formatTime(duration)}
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <Select value={rate} onValueChange={setRate}>
          <SelectTrigger className="w-24">
            <SelectValue placeholder="再生速度" />
          </SelectTrigger>
          <SelectContent>
            {speeds.map((speed) => (
              <SelectItem key={speed} value={speed}>
                {speed}x
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Button variant="outline" asChild>
          <a href={url} download>
            <Download className="mr-2 h-4 w-4" />
            ダウンロード
          </a>
        </Button>
      </div>
    </div>
  )
}

function formatTime(seconds: number) {
  if (!Number.isFinite(seconds)) return "00:00"
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`
}

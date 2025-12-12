"use client"

import { useEffect, useRef, useState } from "react"
import { Button } from "./ui/button"
import { Slider } from "./ui/slider"
import { Play, Pause, Volume2 } from "lucide-react"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select"

interface AudioPlayerProps {
  recordingUrl: string
  durationSec: number
  onTimeUpdate?: (currentTime: number) => void
  onPlayingChange?: (isPlaying: boolean) => void
}

export function AudioPlayer({ recordingUrl, durationSec, onTimeUpdate, onPlayingChange }: AudioPlayerProps) {
  const audioRef = useRef<HTMLAudioElement>(null)
  const [isPlaying, setIsPlaying] = useState(false)
  const [currentTime, setCurrentTime] = useState(0)
  const [duration, setDuration] = useState(durationSec)
  const [playbackRate, setPlaybackRate] = useState("1")

  useEffect(() => {
    const audio = audioRef.current
    if (!audio) return

    const handleLoadedMetadata = () => {
      setDuration(audio.duration || durationSec)
    }

    const handleTimeUpdate = () => {
      setCurrentTime(audio.currentTime)
      onTimeUpdate?.(audio.currentTime)
    }

    const handleEnded = () => {
      setIsPlaying(false)
      onPlayingChange?.(false)
    }

    audio.addEventListener("loadedmetadata", handleLoadedMetadata)
    audio.addEventListener("timeupdate", handleTimeUpdate)
    audio.addEventListener("ended", handleEnded)

    return () => {
      audio.removeEventListener("loadedmetadata", handleLoadedMetadata)
      audio.removeEventListener("timeupdate", handleTimeUpdate)
      audio.removeEventListener("ended", handleEnded)
    }
  }, [durationSec, onTimeUpdate, onPlayingChange])

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
      onPlayingChange?.(false)
    } else {
      audio.play()
      setIsPlaying(true)
      onPlayingChange?.(true)
    }
  }

  const handleSeek = (value: number[]) => {
    const audio = audioRef.current
    if (!audio) return

    const newTime = value[0]
    audio.currentTime = newTime
    setCurrentTime(newTime)
  }

  const formatTime = (seconds: number) => {
    const mins = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${mins}:${secs.toString().padStart(2, "0")}`
  }

  // Public method to seek from parent component
  useEffect(() => {
    const audio = audioRef.current
    if (audio) {
      ;(audio as any).seekTo = (time: number) => {
        audio.currentTime = time
        setCurrentTime(time)
      }
    }
  }, [])

  return (
    <div className="bg-card border rounded-lg p-4">
      <audio ref={audioRef} src={recordingUrl} preload="metadata" />

      <div className="flex items-center gap-4">
        <Button variant="outline" size="icon" onClick={togglePlay} className="shrink-0 bg-transparent">
          {isPlaying ? <Pause className="h-4 w-4" /> : <Play className="h-4 w-4" />}
        </Button>

        <div className="flex-1 space-y-2">
          <Slider
            value={[currentTime]}
            max={duration}
            step={0.1}
            onValueChange={handleSeek}
            className="cursor-pointer"
          />
          <div className="flex items-center justify-between text-xs text-muted-foreground">
            <span>{formatTime(currentTime)}</span>
            <span>{formatTime(duration)}</span>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <Volume2 className="h-4 w-4 text-muted-foreground" />
          <Select value={playbackRate} onValueChange={setPlaybackRate}>
            <SelectTrigger className="w-20 h-8">
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
      </div>
    </div>
  )
}

// Export ref type for parent access
export type AudioPlayerRef = HTMLAudioElement

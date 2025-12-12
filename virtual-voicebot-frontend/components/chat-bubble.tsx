"use client"

import type { Utterance } from "@/lib/types"
import { cn } from "@/lib/utils"
import { Button } from "./ui/button"
import { Play } from "lucide-react"

interface ChatBubbleProps {
  utterance: Utterance
  isHighlighted?: boolean
  onPlayUtterance?: (startSec: number) => void
}

export function ChatBubble({ utterance, isHighlighted = false, onPlayUtterance }: ChatBubbleProps) {
  const isBot = utterance.speaker === "bot"
  const isSystem = utterance.speaker === "system"

  const formatTime = (isoString: string) => {
    const date = new Date(isoString)
    return new Intl.DateTimeFormat("ja-JP", {
      hour: "2-digit",
      minute: "2-digit",
    }).format(date)
  }

  if (isSystem) {
    return (
      <div className="flex justify-center">
        <div className="bg-muted/50 rounded-lg px-3 py-1.5 text-xs text-muted-foreground">{utterance.text}</div>
      </div>
    )
  }

  return (
    <div
      className={cn(
        "flex flex-col gap-1 max-w-[75%] transition-all",
        isBot ? "self-start" : "self-end items-end",
        isHighlighted && "ring-2 ring-primary ring-offset-2 ring-offset-background",
      )}
    >
      <div
        className={cn(
          "rounded-2xl px-4 py-2 break-words",
          isBot ? "bg-muted text-foreground rounded-tl-none" : "bg-primary text-primary-foreground rounded-tr-none",
          !utterance.isFinal && "opacity-60",
        )}
      >
        <p className="text-sm leading-relaxed">{utterance.text}</p>
      </div>
      <div className="flex items-center gap-2 px-2">
        <span className="text-xs text-muted-foreground">{formatTime(utterance.timestamp)}</span>
        {utterance.startSec !== undefined && onPlayUtterance && (
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5"
            onClick={() => onPlayUtterance(utterance.startSec!)}
            title="この発話を再生"
          >
            <Play className="h-3 w-3" />
          </Button>
        )}
      </div>
    </div>
  )
}

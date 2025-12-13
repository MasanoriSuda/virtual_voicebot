"use client"

import { useState, useEffect, useCallback, useRef } from "react"
import type { Utterance, WebSocketMessage } from "../types"

interface UseCallStreamOptions {
  enabled?: boolean
}

export function useCallStream(callId: string, initialUtterances: Utterance[], options: UseCallStreamOptions = {}) {
  const { enabled = true } = options
  const [utterances, setUtterances] = useState<Utterance[]>(initialUtterances)
  const [summary, setSummary] = useState<string | null>(null)
  const wsRef = useRef<WebSocket | null>(null)
  const reconnectTimeoutRef = useRef<NodeJS.Timeout>()

  const handleMessage = useCallback((event: MessageEvent) => {
    try {
      const message: WebSocketMessage = JSON.parse(event.data)

      switch (message.type) {
        case "utterance.partial":
          setUtterances((prev) => {
            const existingIndex = prev.findIndex((u) => u.seq === message.seq)
            const newUtterance: Utterance = {
              seq: message.seq,
              speaker: message.speaker,
              text: message.text,
              timestamp: message.timestamp,
              isFinal: false,
              startSec: message.startSec,
              endSec: message.endSec,
            }

            if (existingIndex >= 0) {
              const updated = [...prev]
              updated[existingIndex] = newUtterance
              return updated
            }
            return [...prev, newUtterance]
          })
          break

        case "utterance.final":
          setUtterances((prev) => {
            const existingIndex = prev.findIndex((u) => u.seq === message.seq)
            const finalUtterance: Utterance = {
              seq: message.seq,
              speaker: message.speaker,
              text: message.text,
              timestamp: message.timestamp,
              isFinal: true,
              startSec: message.startSec,
              endSec: message.endSec,
            }

            if (existingIndex >= 0) {
              const updated = [...prev]
              updated[existingIndex] = finalUtterance
              return updated
            }
            return [...prev, finalUtterance]
          })
          break

        case "summary.updated":
          setSummary(message.summary)
          break
      }
    } catch (error) {
      console.error("[v0] Failed to parse WebSocket message:", error)
    }
  }, [])

  const connect = useCallback(() => {
    if (!enabled || wsRef.current?.readyState === WebSocket.OPEN) {
      return
    }

    try {
      // Mock WebSocket connection - replace with real WebSocket URL
      // const ws = new WebSocket(`wss://your-api.com/calls/${callId}/stream`);
      console.log(`[v0] Mock WebSocket connection for call ${callId}`)

      // In real implementation, uncomment and use:
      // wsRef.current = ws;
      // ws.addEventListener('message', handleMessage);
      // ws.addEventListener('close', () => {
      //   reconnectTimeoutRef.current = setTimeout(connect, 5000);
      // });
    } catch (error) {
      console.error("[v0] WebSocket connection error:", error)
      reconnectTimeoutRef.current = setTimeout(connect, 5000)
    }
  }, [callId, enabled, handleMessage])

  useEffect(() => {
    if (enabled) {
      connect()
    }

    return () => {
      if (reconnectTimeoutRef.current) {
        clearTimeout(reconnectTimeoutRef.current)
      }
      if (wsRef.current) {
        wsRef.current.close()
        wsRef.current = null
      }
    }
  }, [connect, enabled])

  return { utterances, summary }
}

"use client"

import { useCallback, useEffect, useRef, useState } from "react"

const POLL_INTERVAL_MS = 1_000

export interface IncomingCallIvrData {
  dwellTimeSec: number
  dtmfHistory: string[]
}

export interface IncomingCallNotification {
  id: string
  callerNumber: string
  trigger: "direct" | "ivr_transfer"
  receivedAt: string
  ivrData: IncomingCallIvrData | null
}

interface FetchResponse {
  notifications: IncomingCallNotification[]
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function normalizeNotification(value: unknown): IncomingCallNotification | null {
  if (!isRecord(value)) {
    return null
  }
  if (typeof value.id !== "string" || value.id.trim() === "") {
    return null
  }
  const trigger = value.trigger === "ivr_transfer" ? "ivr_transfer" : "direct"
  const callerNumber =
    typeof value.callerNumber === "string" && value.callerNumber.trim() !== ""
      ? value.callerNumber
      : "unknown"
  const receivedAt =
    typeof value.receivedAt === "string" && !Number.isNaN(Date.parse(value.receivedAt))
      ? new Date(value.receivedAt).toISOString()
      : new Date().toISOString()
  const ivrDataValue = value.ivrData
  const ivrData =
    trigger === "ivr_transfer" && isRecord(ivrDataValue)
      ? {
          dwellTimeSec:
            typeof ivrDataValue.dwellTimeSec === "number" && Number.isFinite(ivrDataValue.dwellTimeSec)
              ? Math.max(0, Math.floor(ivrDataValue.dwellTimeSec))
              : 0,
          dtmfHistory: Array.isArray(ivrDataValue.dtmfHistory)
            ? ivrDataValue.dtmfHistory.filter(
                (item): item is string => typeof item === "string",
              )
            : [],
        }
      : null
  return {
    id: value.id,
    callerNumber,
    trigger,
    receivedAt,
    ivrData,
  }
}

function normalizeResponse(payload: unknown): IncomingCallNotification[] {
  if (!isRecord(payload) || !Array.isArray(payload.notifications)) {
    return []
  }
  return payload.notifications
    .map((item) => normalizeNotification(item))
    .filter((item): item is IncomingCallNotification => item !== null)
}

export function useIncomingCallNotifications() {
  const [notifications, setNotifications] = useState<IncomingCallNotification[]>([])
  const [error, setError] = useState<string | null>(null)
  const inFlight = useRef(false)

  const reload = useCallback(async () => {
    if (inFlight.current) {
      return
    }
    inFlight.current = true
    try {
      const response = await fetch("/api/incoming-call-notifications", { cache: "no-store" })
      if (!response.ok) {
        throw new Error(`GET failed: ${response.status}`)
      }
      const body = (await response.json()) as FetchResponse
      setNotifications(normalizeResponse(body))
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : "failed to load incoming call notifications")
    } finally {
      inFlight.current = false
    }
  }, [])

  const dismiss = useCallback(async (id: string) => {
    const response = await fetch(`/api/incoming-call-notifications/${encodeURIComponent(id)}`, {
      method: "DELETE",
    })
    if (!response.ok) {
      throw new Error(`DELETE failed: ${response.status}`)
    }
    setNotifications((prev) => prev.filter((item) => item.id !== id))
  }, [])

  useEffect(() => {
    void reload()
    const timer = window.setInterval(() => {
      void reload()
    }, POLL_INTERVAL_MS)
    return () => window.clearInterval(timer)
  }, [reload])

  return {
    notifications,
    error,
    reload,
    dismiss,
  }
}

import { NextRequest, NextResponse } from "next/server"

import {
  addIncomingCallNotification,
  type IncomingCallIvrData,
  type IncomingCallNotificationInput,
} from "@/lib/db/notifications"

export const runtime = "nodejs"

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function parseIvrData(value: unknown): IncomingCallIvrData {
  if (!isRecord(value)) {
    throw new Error("ivrData is required for ivr_transfer")
  }
  const dwellTimeSec = value.dwellTimeSec
  if (typeof dwellTimeSec !== "number" || !Number.isFinite(dwellTimeSec) || dwellTimeSec < 0) {
    throw new Error("ivrData.dwellTimeSec must be non-negative number")
  }
  const dtmfHistory = value.dtmfHistory
  if (!Array.isArray(dtmfHistory) || !dtmfHistory.every((item) => typeof item === "string")) {
    throw new Error("ivrData.dtmfHistory must be string[]")
  }
  return {
    dwellTimeSec: Math.floor(dwellTimeSec),
    dtmfHistory,
  }
}

function parsePayload(payload: unknown): IncomingCallNotificationInput {
  if (!isRecord(payload)) {
    throw new Error("request body must be object")
  }
  if (typeof payload.callerNumber !== "string" || payload.callerNumber.trim() === "") {
    throw new Error("callerNumber is required")
  }
  if (typeof payload.receivedAt !== "string" || Number.isNaN(Date.parse(payload.receivedAt))) {
    throw new Error("receivedAt must be ISO8601")
  }
  if (payload.trigger !== "direct" && payload.trigger !== "ivr_transfer") {
    throw new Error("trigger must be direct or ivr_transfer")
  }

  if (payload.trigger === "ivr_transfer") {
    return {
      callerNumber: payload.callerNumber,
      trigger: "ivr_transfer",
      receivedAt: new Date(payload.receivedAt).toISOString(),
      ivrData: parseIvrData(payload.ivrData),
    }
  }

  return {
    callerNumber: payload.callerNumber,
    trigger: "direct",
    receivedAt: new Date(payload.receivedAt).toISOString(),
    ivrData: null,
  }
}

export async function POST(req: NextRequest) {
  let body: unknown
  try {
    body = await req.json()
  } catch {
    return NextResponse.json({ ok: false, error: "invalid json body" }, { status: 400 })
  }

  let payload: IncomingCallNotificationInput
  try {
    payload = parsePayload(body)
  } catch (error) {
    return NextResponse.json(
      {
        ok: false,
        error: error instanceof Error ? error.message : "invalid request body",
      },
      { status: 400 },
    )
  }

  try {
    const entry = await addIncomingCallNotification(payload)
    return NextResponse.json({ ok: true, id: entry.id })
  } catch (error) {
    console.error("[ingest/incoming-call] failed to persist notification", error)
    return NextResponse.json(
      { ok: false, error: "failed to persist incoming call notification" },
      { status: 500 },
    )
  }
}

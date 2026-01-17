import { NextResponse } from "next/server"
import { z } from "zod"
import { upsertCall } from "@/lib/db"

const payloadSchema = z.object({
  callId: z.string(),
  from: z.string(),
  to: z.string(),
  startedAt: z.string(),
  endedAt: z.string().nullable().optional(),
  status: z.string().optional(),
  summary: z.string().optional().default(""),
  durationSec: z.number().optional(),
  recording: z
    .object({
      recordingUrl: z.string().url(),
      durationSec: z.number().optional(),
      sampleRate: z.number().optional(),
      channels: z.number().optional(),
    })
    .optional(),
})

export async function POST(req: Request) {
  try {
    const json = await req.json()
    const data = payloadSchema.parse(json)
    const duration =
      data.durationSec ??
      (data.startedAt && data.endedAt ? Math.max(0, Math.floor((Date.parse(data.endedAt) - Date.parse(data.startedAt)) / 1000)) : 0)

    upsertCall({
      id: data.callId,
      from_number: data.from,
      to_number: data.to,
      start_time: data.startedAt,
      end_time: data.endedAt ?? null,
      status: data.status ?? "completed",
      summary: data.summary ?? "",
      duration_sec: duration,
      recording_url: data.recording?.recordingUrl ?? null,
      sample_rate: data.recording?.sampleRate ?? null,
      channels: data.recording?.channels ?? null,
    })

    return NextResponse.json({ ok: true })
  } catch (e) {
    console.error("[ingest][call] failed", e)
    return NextResponse.json({ error: "invalid_payload" }, { status: 400 })
  }
}

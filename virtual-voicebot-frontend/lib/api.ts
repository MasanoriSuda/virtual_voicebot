import type { Call, CallDetail } from "./types"
import { getCallById, listCalls } from "./db"

function mapStatus(status: string): Call["status"] {
  if (status === "failed") return "failed"
  if (status === "active") return "active"
  return "completed"
}

function mapCallRow(row: ReturnType<typeof listCalls>[number]): Call {
  return {
    id: row.id,
    from: row.from_number,
    to: row.to_number,
    callerNumber: row.from_number,
    startTime: row.start_time,
    duration: row.duration_sec,
    durationSec: row.duration_sec,
    status: mapStatus(row.status),
    summary: row.summary,
    recordingUrl: row.recording_url || undefined,
  }
}

export async function getCalls(): Promise<Call[]> {
  const rows = listCalls()
  return rows.map(mapCallRow)
}

export async function getCall(callId: string): Promise<Call | null> {
  const row = getCallById(callId)
  return row ? mapCallRow(row) : null
}

export async function getCallDetail(callId: string): Promise<CallDetail | null> {
  const row = getCallById(callId)
  if (!row) return null
  const call = mapCallRow(row)
  return {
    ...call,
    utterances: [],
  }
}

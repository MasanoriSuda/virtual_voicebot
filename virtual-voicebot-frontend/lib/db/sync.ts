import { promises as fs } from "node:fs"
import * as path from "node:path"

const STORAGE_ROOT = path.join(process.cwd(), "storage")
const DB_DIR = path.join(STORAGE_ROOT, "db")
const SYNC_DB_FILE = path.join(DB_DIR, "sync.json")

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

export interface SyncIngestEntry {
  entityType: string
  entityId: string
  payload: unknown
  createdAt: string
}

interface StoredCallLog {
  id: string
  externalCallId: string
  sipCallId: string | null
  callerNumber: string | null
  callerCategory: string
  actionCode: string
  ivrFlowId: string | null
  status: string
  startedAt: string
  answeredAt: string | null
  endedAt: string | null
  durationSec: number | null
  endReason: string
  createdAt: string
  updatedAt: string
}

interface StoredRecording {
  id: string
  callLogId: string
  recordingType: string
  sequenceNumber: number
  filePath: string | null
  s3Url: string | null
  uploadStatus: string
  durationSec: number | null
  format: string
  fileSizeBytes: number | null
  startedAt: string
  endedAt: string | null
  createdAt: string
  updatedAt: string
}

interface SyncDatabase {
  callLogs: Record<string, StoredCallLog>
  recordings: Record<string, StoredRecording>
  updatedAt: string
}

let writeQueue: Promise<unknown> = Promise.resolve()

function withWriteLock<T>(fn: () => Promise<T>): Promise<T> {
  const run = writeQueue.then(fn, fn)
  writeQueue = run.then(
    () => undefined,
    () => undefined,
  )
  return run
}

function emptyDb(): SyncDatabase {
  return {
    callLogs: {},
    recordings: {},
    updatedAt: new Date(0).toISOString(),
  }
}

async function ensureDbDir() {
  await fs.mkdir(DB_DIR, { recursive: true })
}

async function readDb(): Promise<SyncDatabase> {
  await ensureDbDir()
  try {
    const raw = await fs.readFile(SYNC_DB_FILE, "utf8")
    const parsed = JSON.parse(raw) as Partial<SyncDatabase>
    return {
      callLogs: parsed.callLogs ?? {},
      recordings: parsed.recordings ?? {},
      updatedAt: parsed.updatedAt ?? new Date(0).toISOString(),
    }
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    if (err.code === "ENOENT") {
      return emptyDb()
    }
    throw error
  }
}

async function writeDb(db: SyncDatabase): Promise<void> {
  await ensureDbDir()
  const tempFile = `${SYNC_DB_FILE}.tmp`
  await fs.writeFile(tempFile, JSON.stringify(db, null, 2), "utf8")
  await fs.rename(tempFile, SYNC_DB_FILE)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function asString(
  input: Record<string, unknown>,
  keys: string[],
  fallback: string | null = null,
): string | null {
  for (const key of keys) {
    const value = input[key]
    if (typeof value === "string") {
      return value
    }
  }
  return fallback
}

function asNumber(
  input: Record<string, unknown>,
  keys: string[],
  fallback: number | null = null,
): number | null {
  for (const key of keys) {
    const value = input[key]
    if (typeof value === "number" && Number.isFinite(value)) {
      return value
    }
    if (typeof value === "string" && value.trim() !== "") {
      const parsed = Number(value)
      if (Number.isFinite(parsed)) {
        return parsed
      }
    }
  }
  return fallback
}

function asIsoDate(
  input: Record<string, unknown>,
  keys: string[],
  fallback: string,
): string {
  for (const key of keys) {
    const value = input[key]
    if (typeof value === "string" && !Number.isNaN(Date.parse(value))) {
      return new Date(value).toISOString()
    }
  }
  return fallback
}

function normalizeCallLog(entityId: string, payload: unknown, nowIso: string): StoredCallLog {
  const input = isRecord(payload) ? payload : {}
  const id = asString(input, ["id"], entityId) ?? entityId
  return {
    id,
    externalCallId: asString(input, ["externalCallId", "external_call_id"], id) ?? id,
    sipCallId: asString(input, ["sipCallId", "sip_call_id"], null),
    callerNumber: asString(input, ["callerNumber", "caller_number"], null),
    callerCategory: asString(input, ["callerCategory", "caller_category"], "unknown") ?? "unknown",
    actionCode: asString(input, ["actionCode", "action_code"], "IV") ?? "IV",
    ivrFlowId: asString(input, ["ivrFlowId", "ivr_flow_id"], null),
    status: asString(input, ["status"], "ended") ?? "ended",
    startedAt: asIsoDate(input, ["startedAt", "started_at", "createdAt", "created_at"], nowIso),
    answeredAt: asString(input, ["answeredAt", "answered_at"], null),
    endedAt: asString(input, ["endedAt", "ended_at"], null),
    durationSec: asNumber(input, ["durationSec", "duration_sec"], null),
    endReason: asString(input, ["endReason", "end_reason"], "normal") ?? "normal",
    createdAt: asIsoDate(input, ["createdAt", "created_at"], nowIso),
    updatedAt: nowIso,
  }
}

function normalizeRecording(entityId: string, payload: unknown, nowIso: string): StoredRecording {
  const input = isRecord(payload) ? payload : {}
  const id = asString(input, ["id"], entityId) ?? entityId
  const callLogId = asString(input, ["callLogId", "call_log_id"], id) ?? id
  return {
    id,
    callLogId,
    recordingType:
      asString(input, ["recordingType", "recording_type"], "full_call") ?? "full_call",
    sequenceNumber: asNumber(input, ["sequenceNumber", "sequence_number"], 1) ?? 1,
    filePath: asString(input, ["filePath", "file_path"], null),
    s3Url: asString(input, ["s3Url", "s3_url"], null),
    uploadStatus: asString(input, ["uploadStatus", "upload_status"], "local_only") ?? "local_only",
    durationSec: asNumber(input, ["durationSec", "duration_sec"], null),
    format: asString(input, ["format"], "wav") ?? "wav",
    fileSizeBytes: asNumber(input, ["fileSizeBytes", "file_size_bytes"], null),
    startedAt: asIsoDate(input, ["startedAt", "started_at", "createdAt", "created_at"], nowIso),
    endedAt: asString(input, ["endedAt", "ended_at"], null),
    createdAt: asIsoDate(input, ["createdAt", "created_at"], nowIso),
    updatedAt: nowIso,
  }
}

function assertUuid(value: string, field: string) {
  if (!UUID_RE.test(value)) {
    throw new Error(`${field} must be UUID`)
  }
}

export async function applySyncEntries(entries: SyncIngestEntry[]): Promise<{
  processed: number
  skipped: number
}> {
  return withWriteLock(async () => {
    const nowIso = new Date().toISOString()
    const db = await readDb()
    const next: SyncDatabase = {
      callLogs: { ...db.callLogs },
      recordings: { ...db.recordings },
      updatedAt: nowIso,
    }

    let processed = 0
    let skipped = 0

    for (const entry of entries) {
      assertUuid(entry.entityId, "entityId")
      switch (entry.entityType) {
        case "call_log":
          next.callLogs[entry.entityId] = normalizeCallLog(entry.entityId, entry.payload, nowIso)
          processed += 1
          break
        case "recording":
          next.recordings[entry.entityId] = normalizeRecording(entry.entityId, entry.payload, nowIso)
          processed += 1
          break
        default:
          skipped += 1
          break
      }
    }

    await writeDb(next)
    return { processed, skipped }
  })
}

export async function markRecordingUploaded(params: {
  recordingId: string
  callLogId: string
  filePath: string
  fileUrl: string
}): Promise<void> {
  return withWriteLock(async () => {
    assertUuid(params.recordingId, "recordingId")
    assertUuid(params.callLogId, "callLogId")

    const nowIso = new Date().toISOString()
    const db = await readDb()
    const current = db.recordings[params.recordingId]

    db.recordings[params.recordingId] = {
      id: params.recordingId,
      callLogId: current?.callLogId ?? params.callLogId,
      recordingType: current?.recordingType ?? "full_call",
      sequenceNumber: current?.sequenceNumber ?? 1,
      filePath: params.filePath,
      s3Url: params.fileUrl,
      uploadStatus: "uploaded",
      durationSec: current?.durationSec ?? null,
      format: current?.format ?? "wav",
      fileSizeBytes: current?.fileSizeBytes ?? null,
      startedAt: current?.startedAt ?? nowIso,
      endedAt: current?.endedAt ?? null,
      createdAt: current?.createdAt ?? nowIso,
      updatedAt: nowIso,
    }
    db.updatedAt = nowIso

    await writeDb(db)
  })
}

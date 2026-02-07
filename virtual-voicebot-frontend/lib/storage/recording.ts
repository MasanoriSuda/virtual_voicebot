import { promises as fs } from "node:fs"
import * as path from "node:path"

const STORAGE_RECORDINGS_DIR = path.join(process.cwd(), "storage", "recordings")

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

export class RecordingStorageError extends Error {
  constructor(
    message: string,
    public readonly code: "INVALID_INPUT" | "INVALID_META" | "WRITE_FAILED",
  ) {
    super(message)
    this.name = "RecordingStorageError"
  }
}

export interface SaveRecordingInput {
  callLogId: string
  audioFile: File
  metaRaw: string
  baseUrl: string
}

export interface SavedRecording {
  fileUrl: string
  audioPath: string
  metaPath: string
}

function assertUuid(value: string, field: string) {
  if (!UUID_RE.test(value)) {
    throw new RecordingStorageError(`${field} must be UUID`, "INVALID_INPUT")
  }
}

function sanitizeBaseUrl(baseUrl: string): string {
  const normalized = baseUrl.trim().replace(/\/+$/, "")
  return normalized.length > 0 ? normalized : ""
}

export async function saveRecordingFile(input: SaveRecordingInput): Promise<SavedRecording> {
  assertUuid(input.callLogId, "callLogId")

  try {
    JSON.parse(input.metaRaw)
  } catch {
    throw new RecordingStorageError("meta must be valid JSON", "INVALID_META")
  }

  const callDir = path.join(STORAGE_RECORDINGS_DIR, input.callLogId)
  const audioPath = path.join(callDir, "mixed.wav")
  const metaPath = path.join(callDir, "meta.json")

  try {
    await fs.mkdir(callDir, { recursive: true })
    const audioBuffer = Buffer.from(await input.audioFile.arrayBuffer())
    await fs.writeFile(audioPath, audioBuffer)
    await fs.writeFile(metaPath, input.metaRaw, "utf8")
  } catch (error) {
    const err = error as NodeJS.ErrnoException
    throw new RecordingStorageError(err.message, "WRITE_FAILED")
  }

  const baseUrl = sanitizeBaseUrl(input.baseUrl)
  const fileUrl = `${baseUrl}/storage/recordings/${encodeURIComponent(input.callLogId)}/mixed.wav`

  return { fileUrl, audioPath, metaPath }
}

import * as path from "node:path"

import { NextRequest, NextResponse } from "next/server"

import { markRecordingUploaded } from "@/lib/db/sync"
import { RecordingStorageError, saveRecordingFile } from "@/lib/storage/recording"

export const runtime = "nodejs"

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

function isUuid(value: string): boolean {
  return UUID_RE.test(value)
}

async function readTextPart(value: FormDataEntryValue | null): Promise<string | null> {
  if (typeof value === "string") {
    return value.trim()
  }
  if (value instanceof File) {
    return (await value.text()).trim()
  }
  return null
}

export async function POST(req: NextRequest) {
  const formData = await req.formData()

  const callLogId = await readTextPart(formData.get("callLogId"))
  const recordingId = await readTextPart(formData.get("recordingId"))
  const metaRaw = await readTextPart(formData.get("meta"))
  const audio = formData.get("audio")

  if (!callLogId || !isUuid(callLogId)) {
    return NextResponse.json({ ok: false, error: "callLogId must be UUID" }, { status: 400 })
  }
  if (!recordingId || !isUuid(recordingId)) {
    return NextResponse.json({ ok: false, error: "recordingId must be UUID" }, { status: 400 })
  }
  if (!metaRaw) {
    return NextResponse.json({ ok: false, error: "meta is required" }, { status: 400 })
  }
  if (!(audio instanceof File)) {
    return NextResponse.json({ ok: false, error: "audio file is required" }, { status: 400 })
  }

  try {
    const baseUrl = process.env.NEXT_PUBLIC_BASE_URL || req.nextUrl.origin
    const saved = await saveRecordingFile({
      callLogId,
      audioFile: audio,
      metaRaw,
      baseUrl,
    })

    await markRecordingUploaded({
      recordingId,
      callLogId,
      filePath: path.relative(process.cwd(), saved.audioPath),
      fileUrl: saved.fileUrl,
    })

    return NextResponse.json({ fileUrl: saved.fileUrl })
  } catch (error) {
    if (error instanceof RecordingStorageError) {
      if (error.code === "INVALID_INPUT" || error.code === "INVALID_META") {
        return NextResponse.json({ ok: false, error: error.message }, { status: 400 })
      }
      return NextResponse.json({ ok: false, error: "failed to save files" }, { status: 500 })
    }

    const maybeErr = error as NodeJS.ErrnoException
    if (maybeErr?.code === "ENOSPC") {
      return NextResponse.json({ ok: false, error: "insufficient storage" }, { status: 507 })
    }

    console.error("[ingest/recording-file] failed", error)
    return NextResponse.json({ ok: false, error: "internal server error" }, { status: 500 })
  }
}

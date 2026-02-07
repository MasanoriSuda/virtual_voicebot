import { promises as fs } from "node:fs"
import * as path from "node:path"

import { NextRequest, NextResponse } from "next/server"

import { isWavBuffer, parseWavDurationSec } from "@/lib/audio/wav"
import {
  AnnouncementsStoreError,
  announcementAudioPath,
  announcementAudioUrl,
  createAnnouncement,
  isAnnouncementType,
  newAnnouncementId,
} from "@/lib/db/announcements"
import type { AnnouncementType } from "@/lib/types"

export const runtime = "nodejs"

const MAX_WAV_SIZE_BYTES = 10 * 1024 * 1024
const ALLOWED_CONTENT_TYPES = new Set(["audio/wav", "audio/x-wav", "audio/wave", ""])

async function readTextPart(value: FormDataEntryValue | null): Promise<string | null> {
  if (typeof value === "string") {
    const trimmed = value.trim()
    return trimmed.length > 0 ? trimmed : null
  }
  if (value instanceof File) {
    const text = (await value.text()).trim()
    return text.length > 0 ? text : null
  }
  return null
}

export async function POST(req: NextRequest) {
  const formData = await req.formData()
  const file = formData.get("file")
  if (!(file instanceof File)) {
    return NextResponse.json({ ok: false, error: "file is required" }, { status: 400 })
  }

  const name = await readTextPart(formData.get("name"))
  if (!name) {
    return NextResponse.json({ ok: false, error: "name is required" }, { status: 400 })
  }

  const announcementTypeRaw = await readTextPart(formData.get("announcementType"))
  const announcementType: AnnouncementType = announcementTypeRaw && isAnnouncementType(announcementTypeRaw)
    ? announcementTypeRaw
    : "custom"

  const folderId = await readTextPart(formData.get("folderId"))

  if (file.size > MAX_WAV_SIZE_BYTES) {
    return NextResponse.json(
      { ok: false, error: "file size must be 10MB or less" },
      { status: 400 },
    )
  }

  if (!file.name.toLowerCase().endsWith(".wav")) {
    return NextResponse.json({ ok: false, error: "only .wav file is allowed" }, { status: 400 })
  }

  if (!ALLOWED_CONTENT_TYPES.has(file.type)) {
    return NextResponse.json(
      { ok: false, error: "content-type must be audio/wav" },
      { status: 400 },
    )
  }

  const id = newAnnouncementId()
  const audioFileUrl = announcementAudioUrl(id)
  const audioPath = announcementAudioPath(id)

  try {
    const bytes = Buffer.from(await file.arrayBuffer())
    if (!isWavBuffer(bytes)) {
      return NextResponse.json(
        { ok: false, error: "file must be a valid WAV (RIFF/WAVE)" },
        { status: 400 },
      )
    }

    await fs.mkdir(path.dirname(audioPath), { recursive: true })
    await fs.writeFile(audioPath, bytes)

    const announcement = await createAnnouncement({
      id,
      name,
      announcementType,
      folderId,
      audioFileUrl,
      durationSec: parseWavDurationSec(bytes),
      language: "ja",
      source: "upload",
    })

    return NextResponse.json({ ok: true, announcement })
  } catch (error) {
    if (error instanceof AnnouncementsStoreError && error.code === "VALIDATION") {
      return NextResponse.json({ ok: false, error: error.message }, { status: 400 })
    }

    console.error("[api/announcements/upload] failed", error)
    return NextResponse.json({ ok: false, error: "failed to upload announcement" }, { status: 500 })
  }
}

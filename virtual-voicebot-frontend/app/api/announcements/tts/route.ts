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

const VOICEVOX_TIMEOUT_MS = 15_000

interface TtsRequestBody {
  text: string
  speakerId: number
  name: string
  announcementType?: string
  folderId?: string
}

interface VoiceVoxSpeaker {
  name: string
  styles?: Array<{ id: number; name: string }>
}

function voiceVoxBaseUrl(): string {
  const fromEnv = process.env.VOICEVOX_BASE_URL?.trim()
  return fromEnv && fromEnv.length > 0 ? fromEnv.replace(/\/+$/, "") : "http://localhost:50021"
}

function voiceVoxUnavailableMessage(baseUrl: string): string {
  try {
    return `VoiceVox に接続できません（${new URL(baseUrl).host}）`
  } catch {
    return "VoiceVox に接続できません"
  }
}

async function fetchWithTimeout(
  input: string,
  init: RequestInit,
  timeoutMs = VOICEVOX_TIMEOUT_MS,
): Promise<Response> {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeoutMs)
  try {
    return await fetch(input, {
      ...init,
      signal: controller.signal,
    })
  } finally {
    clearTimeout(timer)
  }
}

function parseBody(raw: unknown): TtsRequestBody {
  if (typeof raw !== "object" || raw === null) {
    throw new Error("invalid request body")
  }
  const body = raw as Partial<TtsRequestBody>
  if (typeof body.text !== "string" || body.text.trim().length === 0) {
    throw new Error("text is required")
  }
  if (body.text.trim().length > 1000) {
    throw new Error("text must be 1000 characters or less")
  }
  if (typeof body.speakerId !== "number" || !Number.isFinite(body.speakerId)) {
    throw new Error("speakerId is required")
  }
  if (typeof body.name !== "string" || body.name.trim().length === 0) {
    throw new Error("name is required")
  }
  return {
    text: body.text.trim(),
    speakerId: body.speakerId,
    name: body.name.trim(),
    announcementType: body.announcementType,
    folderId: typeof body.folderId === "string" ? body.folderId.trim() : undefined,
  }
}

function normalizeAnnouncementType(value: string | undefined): AnnouncementType {
  if (!value) {
    return "custom"
  }
  if (!isAnnouncementType(value)) {
    throw new Error("invalid announcementType")
  }
  return value
}

async function resolveSpeakerName(baseUrl: string, speakerId: number): Promise<string | null> {
  try {
    const response = await fetchWithTimeout(`${baseUrl}/speakers`, { method: "GET" })
    if (!response.ok) {
      return null
    }
    const payload = (await response.json()) as VoiceVoxSpeaker[]
    for (const speaker of payload) {
      if (!Array.isArray(speaker.styles)) {
        continue
      }
      for (const style of speaker.styles) {
        if (style.id === speakerId) {
          return `${speaker.name} - ${style.name}`
        }
      }
    }
    return null
  } catch {
    return null
  }
}

export async function POST(req: NextRequest) {
  let body: TtsRequestBody
  try {
    body = parseBody(await req.json())
  } catch (error) {
    return NextResponse.json(
      { ok: false, error: error instanceof Error ? error.message : "invalid request body" },
      { status: 400 },
    )
  }

  let announcementType: AnnouncementType
  try {
    announcementType = normalizeAnnouncementType(body.announcementType)
  } catch (error) {
    return NextResponse.json(
      { ok: false, error: error instanceof Error ? error.message : "invalid announcementType" },
      { status: 400 },
    )
  }
  const baseUrl = voiceVoxBaseUrl()
  const unavailableMessage = voiceVoxUnavailableMessage(baseUrl)

  try {
    const queryParams = new URLSearchParams({
      text: body.text,
      speaker: String(body.speakerId),
    })
    const audioQueryResponse = await fetchWithTimeout(`${baseUrl}/audio_query?${queryParams}`, {
      method: "POST",
    })
    if (!audioQueryResponse.ok) {
      return NextResponse.json({ ok: false, error: unavailableMessage }, { status: 502 })
    }

    const audioQuery = await audioQueryResponse.json()
    const synthResponse = await fetchWithTimeout(
      `${baseUrl}/synthesis?speaker=${encodeURIComponent(String(body.speakerId))}`,
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(audioQuery),
      },
    )
    if (!synthResponse.ok) {
      return NextResponse.json({ ok: false, error: unavailableMessage }, { status: 502 })
    }

    const bytes = Buffer.from(await synthResponse.arrayBuffer())
    if (!isWavBuffer(bytes)) {
      return NextResponse.json(
        { ok: false, error: "VoiceVox returned invalid WAV data" },
        { status: 502 },
      )
    }

    const id = newAnnouncementId()
    const audioPath = announcementAudioPath(id)
    const audioFileUrl = announcementAudioUrl(id)
    await fs.mkdir(path.dirname(audioPath), { recursive: true })
    await fs.writeFile(audioPath, bytes)

    const speakerName = await resolveSpeakerName(baseUrl, body.speakerId)
    const announcement = await createAnnouncement({
      id,
      name: body.name,
      announcementType,
      folderId: body.folderId,
      audioFileUrl,
      ttsText: body.text,
      speakerId: body.speakerId,
      speakerName,
      durationSec: parseWavDurationSec(bytes),
      language: "ja",
      source: "tts",
    })

    return NextResponse.json({ ok: true, announcement })
  } catch (error) {
    if (error instanceof AnnouncementsStoreError && error.code === "VALIDATION") {
      return NextResponse.json({ ok: false, error: error.message }, { status: 400 })
    }
    if (error instanceof Error && error.name === "AbortError") {
      return NextResponse.json({ ok: false, error: unavailableMessage }, { status: 502 })
    }
    if (error instanceof Error && /fetch/i.test(error.message)) {
      return NextResponse.json({ ok: false, error: unavailableMessage }, { status: 502 })
    }

    console.error("[api/announcements/tts] failed", error)
    return NextResponse.json({ ok: false, error: "failed to create announcement" }, { status: 500 })
  }
}

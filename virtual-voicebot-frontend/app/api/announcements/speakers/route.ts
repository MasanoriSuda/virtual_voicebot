import { NextResponse } from "next/server"

export const runtime = "nodejs"

const VOICEVOX_TIMEOUT_MS = 10_000

interface VoiceVoxSpeaker {
  name: string
  styles?: Array<{ id: number; name: string }>
}

function voiceVoxBaseUrl(): string {
  const fromEnv = process.env.VOICEVOX_BASE_URL?.trim()
  return fromEnv && fromEnv.length > 0 ? fromEnv.replace(/\/+$/, "") : "http://localhost:50021"
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

export async function GET() {
  const baseUrl = voiceVoxBaseUrl()
  try {
    const response = await fetchWithTimeout(`${baseUrl}/speakers`, { method: "GET" })
    if (!response.ok) {
      return NextResponse.json({ ok: false, error: "VoiceVox に接続できません" }, { status: 502 })
    }

    const payload = (await response.json()) as VoiceVoxSpeaker[]
    const speakers = Array.isArray(payload)
      ? payload
          .filter((speaker) => typeof speaker.name === "string")
          .map((speaker) => ({
            name: speaker.name,
            styles: Array.isArray(speaker.styles)
              ? speaker.styles
                  .filter((style) => typeof style.id === "number" && typeof style.name === "string")
                  .map((style) => ({ id: style.id, name: style.name }))
              : [],
          }))
      : []

    return NextResponse.json({ ok: true, speakers })
  } catch (error) {
    if (error instanceof Error && error.name === "AbortError") {
      return NextResponse.json({ ok: false, error: "VoiceVox に接続できません" }, { status: 502 })
    }
    if (error instanceof Error && /fetch/i.test(error.message)) {
      return NextResponse.json({ ok: false, error: "VoiceVox に接続できません" }, { status: 502 })
    }

    console.error("[api/announcements/speakers] failed", error)
    return NextResponse.json({ ok: false, error: "failed to load speakers" }, { status: 500 })
  }
}

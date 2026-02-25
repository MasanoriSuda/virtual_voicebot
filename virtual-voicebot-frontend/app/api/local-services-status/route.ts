import { NextResponse } from "next/server"

import type { LocalServicesStatusResponse } from "@/lib/api/local-services-status"

export const runtime = "nodejs"

const BACKEND_TIMEOUT_MS = 10_000

async function fetchWithTimeout(
  input: string,
  init: RequestInit,
  timeoutMs: number,
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
  const backendUrl = process.env.BACKEND_URL || "http://localhost:18080"

  try {
    const response = await fetchWithTimeout(
      `${backendUrl}/api/local-services/status`,
      {
        method: "GET",
        cache: "no-store",
      },
      BACKEND_TIMEOUT_MS,
    )

    if (!response.ok) {
      return NextResponse.json(
        { ok: false, error: "failed to fetch local services status from backend" },
        { status: 502 },
      )
    }

    const payload = (await response.json()) as LocalServicesStatusResponse
    return NextResponse.json(payload)
  } catch (error) {
    console.error("[api/local-services-status] failed", error)
    return NextResponse.json(
      { ok: false, error: "backend connection failed" },
      { status: 503 },
    )
  }
}

import { NextResponse } from "next/server"

import type { SyncStatusResponse } from "@/lib/api/sync-status"

export const runtime = "nodejs"

export async function GET() {
  try {
    const backendUrl = process.env.BACKEND_URL || "http://localhost:18080"
    const response = await fetch(`${backendUrl}/api/sync/status`, {
      method: "GET",
      cache: "no-store",
    })

    if (!response.ok) {
      return NextResponse.json(
        { ok: false, error: "failed to fetch sync status from backend" },
        { status: 502 },
      )
    }

    const payload = (await response.json()) as SyncStatusResponse
    return NextResponse.json(payload)
  } catch (error) {
    console.error("[api/sync-status] failed to fetch backend sync status", error)
    return NextResponse.json(
      { ok: false, error: "backend connection failed" },
      { status: 503 },
    )
  }
}

import { NextResponse } from "next/server"

export async function POST() {
  console.warn("[ingest/call] deprecated endpoint called. Use /api/ingest/sync instead.")
  return NextResponse.json({ ok: true })
}

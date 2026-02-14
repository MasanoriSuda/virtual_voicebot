import { NextRequest, NextResponse } from "next/server"

import { applySyncEntries, type SyncIngestEntry } from "@/lib/db/sync"

export const runtime = "nodejs"

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

function parseEntries(payload: unknown): SyncIngestEntry[] {
  if (!isRecord(payload) || !Array.isArray(payload.entries)) {
    throw new Error("entries must be an array")
  }

  return payload.entries.map((entry, index) => {
    if (!isRecord(entry)) {
      throw new Error(`entries[${index}] must be an object`)
    }
    if (typeof entry.entityType !== "string" || entry.entityType.trim() === "") {
      throw new Error(`entries[${index}].entityType is required`)
    }
    if (typeof entry.entityId !== "string" || entry.entityId.trim() === "") {
      throw new Error(`entries[${index}].entityId is required`)
    }
    if (!("payload" in entry)) {
      throw new Error(`entries[${index}].payload is required`)
    }
    if (typeof entry.createdAt !== "string" || Number.isNaN(Date.parse(entry.createdAt))) {
      throw new Error(`entries[${index}].createdAt must be ISO8601`)
    }

    return {
      entityType: entry.entityType,
      entityId: entry.entityId,
      payload: entry.payload,
      createdAt: new Date(entry.createdAt).toISOString(),
    }
  })
}

export async function POST(req: NextRequest) {
  let body: unknown
  try {
    body = await req.json()
  } catch {
    return NextResponse.json({ ok: false, error: "invalid json body" }, { status: 400 })
  }

  let entries: SyncIngestEntry[]
  try {
    entries = parseEntries(body)
  } catch (error) {
    return NextResponse.json(
      {
        ok: false,
        error: error instanceof Error ? error.message : "invalid request body",
      },
      { status: 400 },
    )
  }

  try {
    const result = await applySyncEntries(entries)
    if (result.skipped > 0) {
      console.warn(`[ingest/sync] skipped ${result.skipped} unsupported or invalid entries`)
    }
    return NextResponse.json({ ok: true })
  } catch (error) {
    console.error("[ingest/sync] failed to upsert entries", error)
    return NextResponse.json({ ok: false, error: "failed to persist entries" }, { status: 500 })
  }
}

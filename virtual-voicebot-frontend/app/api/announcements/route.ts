import { NextResponse } from "next/server"

import {
  AnnouncementsStoreError,
  createAnnouncement,
  isAnnouncementType,
  listAnnouncementsSnapshot,
} from "@/lib/db/announcements"
import type { AnnouncementType } from "@/lib/types"

export const runtime = "nodejs"

export async function GET() {
  try {
    const snapshot = await listAnnouncementsSnapshot()
    return NextResponse.json({
      ok: true,
      announcements: snapshot.announcements,
      folders: snapshot.folders,
    })
  } catch (error) {
    console.error("[api/announcements] failed to fetch announcements", error)
    return NextResponse.json(
      { ok: false, error: "failed to load announcements" },
      { status: 500 },
    )
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

export async function POST(req: Request) {
  let body: unknown
  try {
    body = await req.json()
  } catch {
    return NextResponse.json({ ok: false, error: "invalid json body" }, { status: 400 })
  }

  if (!isRecord(body)) {
    return NextResponse.json({ ok: false, error: "invalid request body" }, { status: 400 })
  }

  const name = typeof body.name === "string" ? body.name.trim() : ""
  if (!name) {
    return NextResponse.json({ ok: false, error: "name is required" }, { status: 400 })
  }

  let announcementType: AnnouncementType = "custom"
  if (typeof body.announcementType === "string") {
    if (!isAnnouncementType(body.announcementType)) {
      return NextResponse.json({ ok: false, error: "invalid announcementType" }, { status: 400 })
    }
    announcementType = body.announcementType
  }

  const folderId =
    typeof body.folderId === "string" && body.folderId.trim().length > 0 ? body.folderId.trim() : null
  const description =
    typeof body.description === "string" && body.description.trim().length > 0
      ? body.description.trim()
      : null
  const isActive = typeof body.isActive === "boolean" ? body.isActive : true

  try {
    const announcement = await createAnnouncement({
      name,
      description,
      announcementType,
      isActive,
      folderId,
      source: "upload",
    })
    return NextResponse.json({ ok: true, announcement })
  } catch (error) {
    if (error instanceof AnnouncementsStoreError && error.code === "VALIDATION") {
      return NextResponse.json({ ok: false, error: error.message }, { status: 400 })
    }

    console.error("[api/announcements] failed to create announcement", error)
    return NextResponse.json(
      { ok: false, error: "failed to create announcement" },
      { status: 500 },
    )
  }
}

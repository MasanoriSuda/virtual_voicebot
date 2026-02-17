import { NextRequest, NextResponse } from "next/server"

import {
  AnnouncementsStoreError,
  deleteAnnouncement,
  deleteAnnouncementAudioFile,
  updateAnnouncement,
} from "@/lib/db/announcements"

export const runtime = "nodejs"

const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
}

export async function PATCH(
  req: NextRequest,
  context: { params: Promise<{ id: string }> },
) {
  const { id } = await context.params
  if (!UUID_RE.test(id)) {
    return NextResponse.json({ ok: false, error: "invalid id" }, { status: 400 })
  }

  let body: unknown
  try {
    body = await req.json()
  } catch {
    return NextResponse.json({ ok: false, error: "invalid json body" }, { status: 400 })
  }

  if (!isRecord(body)) {
    return NextResponse.json({ ok: false, error: "invalid request body" }, { status: 400 })
  }

  const patch: { name?: string; isActive?: boolean } = {}
  if ("name" in body) {
    if (typeof body.name !== "string") {
      return NextResponse.json({ ok: false, error: "name must be string" }, { status: 400 })
    }
    patch.name = body.name
  }
  if ("isActive" in body) {
    if (typeof body.isActive !== "boolean") {
      return NextResponse.json({ ok: false, error: "isActive must be boolean" }, { status: 400 })
    }
    patch.isActive = body.isActive
  }

  if (patch.name === undefined && patch.isActive === undefined) {
    return NextResponse.json({ ok: false, error: "name or isActive is required" }, { status: 400 })
  }

  try {
    const announcement = await updateAnnouncement(id, patch)
    if (!announcement) {
      return NextResponse.json({ ok: false, error: "announcement not found" }, { status: 404 })
    }
    return NextResponse.json({ ok: true, announcement })
  } catch (error) {
    if (error instanceof AnnouncementsStoreError && error.code === "VALIDATION") {
      return NextResponse.json({ ok: false, error: error.message }, { status: 400 })
    }
    console.error("[api/announcements/[id]] patch failed", error)
    return NextResponse.json({ ok: false, error: "failed to update announcement" }, { status: 500 })
  }
}

export async function DELETE(
  _req: NextRequest,
  context: { params: Promise<{ id: string }> },
) {
  const { id } = await context.params
  if (!UUID_RE.test(id)) {
    return NextResponse.json({ ok: false, error: "invalid id" }, { status: 400 })
  }

  try {
    const announcement = await deleteAnnouncement(id)
    if (!announcement) {
      return NextResponse.json({ ok: false, error: "announcement not found" }, { status: 404 })
    }
    await deleteAnnouncementAudioFile(announcement.audioFileUrl)
    return NextResponse.json({ ok: true })
  } catch (error) {
    console.error("[api/announcements/[id]] delete failed", error)
    return NextResponse.json({ ok: false, error: "failed to delete announcement" }, { status: 500 })
  }
}

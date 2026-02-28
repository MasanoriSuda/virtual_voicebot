import { NextRequest, NextResponse } from "next/server"

import { deleteIncomingCallNotification } from "@/lib/db/notifications"

export const runtime = "nodejs"

export async function DELETE(
  _req: NextRequest,
  context: { params: Promise<{ id: string }> },
) {
  const { id } = await context.params
  if (id.trim() === "") {
    return NextResponse.json({ ok: false, error: "invalid id" }, { status: 400 })
  }

  try {
    const deleted = await deleteIncomingCallNotification(id)
    if (!deleted) {
      return NextResponse.json({ ok: false, error: "not_found" }, { status: 404 })
    }
    return NextResponse.json({ ok: true })
  } catch (error) {
    console.error("[api/incoming-call-notifications/[id]] delete failed", error)
    return NextResponse.json(
      { ok: false, error: "failed to delete incoming call notification" },
      { status: 500 },
    )
  }
}

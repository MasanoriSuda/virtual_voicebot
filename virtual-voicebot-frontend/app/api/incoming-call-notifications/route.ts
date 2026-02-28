import { NextResponse } from "next/server"

import { listIncomingCallNotifications } from "@/lib/db/notifications"

export const runtime = "nodejs"

export async function GET() {
  try {
    const notifications = await listIncomingCallNotifications()
    return NextResponse.json({ notifications })
  } catch (error) {
    console.error("[api/incoming-call-notifications] failed to list notifications", error)
    return NextResponse.json(
      { ok: false, error: "failed to list incoming call notifications" },
      { status: 500 },
    )
  }
}

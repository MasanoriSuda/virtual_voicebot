import { NextRequest, NextResponse } from "next/server"

import {
  parseNumberGroupsPayload,
  readNumberGroups,
  writeNumberGroups,
} from "@/lib/db/number-groups"

export const runtime = "nodejs"

export async function GET() {
  try {
    const db = await readNumberGroups()
    return NextResponse.json({
      ok: true,
      callerGroups: db.callerGroups,
    })
  } catch (error) {
    console.error("[api/number-groups] failed to read number groups", error)
    return NextResponse.json(
      { ok: false, error: "failed to load number groups" },
      { status: 500 },
    )
  }
}

export async function PUT(req: NextRequest) {
  let body: unknown
  try {
    body = await req.json()
  } catch {
    return NextResponse.json({ ok: false, error: "invalid json body" }, { status: 400 })
  }

  let parsed
  try {
    parsed = parseNumberGroupsPayload(body)
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
    await writeNumberGroups(parsed)
    return NextResponse.json({ ok: true })
  } catch (error) {
    console.error("[api/number-groups] failed to save number groups", error)
    return NextResponse.json(
      { ok: false, error: "failed to save number groups" },
      { status: 500 },
    )
  }
}

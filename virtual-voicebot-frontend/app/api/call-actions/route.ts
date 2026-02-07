import { NextRequest, NextResponse } from "next/server"

import {
  parseCallActionsPayload,
  readCallActions,
  writeCallActions,
} from "@/lib/db/call-actions"
import { readNumberGroups } from "@/lib/db/number-groups"

export const runtime = "nodejs"

export async function GET() {
  try {
    const db = await readCallActions()
    return NextResponse.json({
      ok: true,
      rules: db.rules,
      anonymousAction: db.anonymousAction,
      defaultAction: db.defaultAction,
    })
  } catch (error) {
    console.error("[api/call-actions] failed to read call actions", error)
    return NextResponse.json(
      { ok: false, error: "failed to load call actions" },
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
    parsed = parseCallActionsPayload(body)
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
    const numberGroups = await readNumberGroups()
    const callerGroupIds = new Set(numberGroups.callerGroups.map((group) => group.id))
    for (const rule of parsed.rules) {
      if (!callerGroupIds.has(rule.callerGroupId)) {
        return NextResponse.json(
          {
            ok: false,
            error: `rules callerGroupId not found: ${rule.callerGroupId}`,
          },
          { status: 400 },
        )
      }
    }

    await writeCallActions(parsed)
    return NextResponse.json({ ok: true })
  } catch (error) {
    console.error("[api/call-actions] failed to save call actions", error)
    return NextResponse.json(
      { ok: false, error: "failed to save call actions" },
      { status: 500 },
    )
  }
}

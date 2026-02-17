import { NextRequest, NextResponse } from "next/server"

import {
  parseIvrFlowsPayload,
  readIvrFlows,
  writeIvrFlows,
} from "@/lib/db/ivr-flows"

export const runtime = "nodejs"

export async function GET() {
  try {
    const db = await readIvrFlows()
    return NextResponse.json({
      ok: true,
      flows: db.flows,
    })
  } catch (error) {
    console.error("[api/ivr-flows] failed to read ivr flows", error)
    return NextResponse.json(
      { ok: false, error: "failed to load ivr flows" },
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
    parsed = parseIvrFlowsPayload(body)
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
    await writeIvrFlows(parsed)
    return NextResponse.json({ ok: true })
  } catch (error) {
    console.error("[api/ivr-flows] failed to save ivr flows", error)
    return NextResponse.json(
      { ok: false, error: "failed to save ivr flows" },
      { status: 500 },
    )
  }
}

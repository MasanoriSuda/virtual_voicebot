import { NextResponse } from "next/server"

import { readIvrFlows } from "@/lib/db/ivr-flows"

export const runtime = "nodejs"

export async function GET() {
  try {
    const db = await readIvrFlows()
    return NextResponse.json({
      ok: true,
      flows: db.flows,
    })
  } catch (error) {
    console.error("[api/ivr-flows/export] failed to read ivr flows", error)
    return NextResponse.json(
      { ok: false, error: "failed to load ivr flows" },
      { status: 500 },
    )
  }
}

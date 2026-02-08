import { NextResponse } from "next/server"

import { readScenariosDatabase } from "@/lib/db/scenarios"

export const runtime = "nodejs"

export async function GET() {
  try {
    const db = await readScenariosDatabase()
    return NextResponse.json({
      ok: true,
      scenarios: db.scenarios,
    })
  } catch (error) {
    console.error("[api/scenarios] failed to read scenarios", error)
    return NextResponse.json(
      { ok: false, error: "failed to load scenarios" },
      { status: 500 },
    )
  }
}

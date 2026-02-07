import { NextRequest, NextResponse } from "next/server"

import { getDashboardMetrics } from "@/lib/aggregations"

export const runtime = "nodejs"

export async function GET(request: NextRequest) {
  const dateParam = request.nextUrl.searchParams.get("date")
  const baseDate = dateParam ? new Date(dateParam) : new Date()
  const targetDate = Number.isNaN(baseDate.getTime()) ? new Date() : baseDate

  try {
    const metrics = await getDashboardMetrics(targetDate)
    return NextResponse.json(metrics)
  } catch (error) {
    console.error("[api/kpi] failed to fetch dashboard metrics", error)
    return NextResponse.json({ ok: false, error: "failed to fetch kpi" }, { status: 500 })
  }
}

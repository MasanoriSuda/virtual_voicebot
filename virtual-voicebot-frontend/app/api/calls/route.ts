import { NextRequest, NextResponse } from "next/server"

import { getCallDetail, getCallsPage, type CallFilters } from "@/lib/api"

export const runtime = "nodejs"

function parseDate(value: string | null): Date | undefined {
  if (!value) {
    return undefined
  }
  const parsed = new Date(value)
  return Number.isNaN(parsed.getTime()) ? undefined : parsed
}

function parseInteger(value: string | null, defaultValue: number): number {
  if (!value) {
    return defaultValue
  }
  const parsed = Number.parseInt(value, 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : defaultValue
}

export async function GET(request: NextRequest) {
  const params = request.nextUrl.searchParams
  const callId = params.get("callId")

  if (callId) {
    try {
      const detail = await getCallDetail(callId)
      if (!detail) {
        return NextResponse.json({ ok: false, error: "call not found" }, { status: 404 })
      }
      return NextResponse.json(detail)
    } catch (error) {
      console.error("[api/calls] failed to fetch call detail", error)
      return NextResponse.json({ ok: false, error: "failed to fetch call detail" }, { status: 500 })
    }
  }

  const startDate = parseDate(params.get("startDate"))
  const endDate = parseDate(params.get("endDate"))
  const direction = params.get("direction")
  const keyword = params.get("keyword") ?? undefined
  const page = parseInteger(params.get("page"), 1)
  const pageSize = parseInteger(params.get("pageSize"), 10)

  const filters: CallFilters = {
    page,
    pageSize,
    keyword,
    direction:
      direction === "inbound" || direction === "outbound" || direction === "missed"
        ? direction
        : null,
    dateRange:
      startDate && endDate
        ? {
            start: startDate,
            end: endDate,
          }
        : undefined,
  }

  try {
    const result = await getCallsPage(filters)
    return NextResponse.json(result)
  } catch (error) {
    console.error("[api/calls] failed to fetch call history", error)
    return NextResponse.json({ ok: false, error: "failed to fetch calls" }, { status: 500 })
  }
}

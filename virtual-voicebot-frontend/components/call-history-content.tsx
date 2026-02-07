"use client"

import { useEffect, useMemo, useState } from "react"
import type { Call } from "@/lib/types"
import { DateRange } from "react-day-picker"
import { Download } from "lucide-react"
import { endOfDay, startOfDay, subDays } from "date-fns"

import { FilterBar, isWithinRange, directionLabel } from "@/components/calls/filter-bar"
import { CallsTable } from "@/components/calls/calls-table"
import { CallDetailDrawer } from "@/components/calls/call-detail-drawer"
import { Button } from "@/components/ui/button"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import type { CallRecord } from "@/lib/mock-data"

interface CallHistoryContentProps {
  initialCalls: Call[]
}

const pageSizeOptions = [10, 25, 50]

export function CallHistoryContent({ initialCalls }: CallHistoryContentProps) {
  const [dateRange, setDateRange] = useState<DateRange | undefined>(() => {
    const today = new Date()
    return { from: startOfDay(subDays(today, 6)), to: endOfDay(today) }
  })
  const [typeFilter, setTypeFilter] = useState("all")
  const [search, setSearch] = useState("")
  const [sortDirection, setSortDirection] = useState<"asc" | "desc">("desc")
  const [pageSize, setPageSize] = useState(10)
  const [page, setPage] = useState(1)
  const [selectedCall, setSelectedCall] = useState<CallRecord | null>(null)

  const records = useMemo(() => initialCalls.map(toRecord), [initialCalls])

  const filteredCalls = useMemo(() => {
    return records.filter((call) => {
      const matchesType = typeFilter === "all" || call.direction === typeFilter
      const matchesSearch =
        search.trim().length === 0 ||
        [call.from, call.fromName, call.to, call.callId]
          .join(" ")
          .toLowerCase()
          .includes(search.toLowerCase())
      const matchesDate = isWithinRange(new Date(call.startedAt), dateRange)
      return matchesType && matchesSearch && matchesDate
    })
  }, [dateRange, records, search, typeFilter])

  const sortedCalls = useMemo(() => {
    return [...filteredCalls].sort((a, b) => {
      const delta = new Date(a.startedAt).getTime() - new Date(b.startedAt).getTime()
      return sortDirection === "asc" ? delta : -delta
    })
  }, [filteredCalls, sortDirection])

  const totalPages = Math.max(1, Math.ceil(sortedCalls.length / pageSize))
  const pagedCalls = sortedCalls.slice((page - 1) * pageSize, page * pageSize)
  const startIndex = sortedCalls.length === 0 ? 0 : (page - 1) * pageSize + 1
  const endIndex = Math.min(page * pageSize, sortedCalls.length)

  useEffect(() => {
    setPage(1)
  }, [dateRange, typeFilter, search, pageSize])

  const handleSortToggle = () => {
    setSortDirection((prev) => (prev === "asc" ? "desc" : "asc"))
  }

  const handleExportCSV = () => {
    const headers = ["日時", "方向", "発信者", "着信先", "通話時間", "ステータス"]
    const rows = sortedCalls.map((call) => [
      formatDateTime(call.startedAt),
      directionLabel(call.direction),
      `${call.fromName} ${call.from}`,
      call.to,
      formatDuration(call.durationSec),
      statusLabel(call.status),
    ])

    const csvContent = [headers, ...rows].map((row) => row.join(",")).join("\n")
    const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" })
    const url = URL.createObjectURL(blob)
    const link = document.createElement("a")
    link.href = url
    link.download = `call-history-${new Date().toISOString().split("T")[0]}.csv`
    link.click()
  }

  return (
    <>
      <div className="p-6 space-y-6">
        <div>
          <h1 className="text-2xl font-bold text-balance">発着信履歴</h1>
          <p className="text-muted-foreground">Call History</p>
        </div>

        <div className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
          <div className="flex-1">
            <FilterBar
              dateRange={dateRange}
              onDateChange={setDateRange}
              typeFilter={typeFilter}
              onTypeChange={setTypeFilter}
              search={search}
              onSearchChange={setSearch}
            />
          </div>
          <Button variant="outline" onClick={handleExportCSV} className="bg-transparent">
            <Download className="mr-2 h-4 w-4" />
            CSV出力
          </Button>
        </div>

        <CallsTable
          calls={pagedCalls}
          sortDirection={sortDirection}
          onSortToggle={handleSortToggle}
          onRowClick={setSelectedCall}
        />

        <div className="flex flex-wrap items-center justify-between gap-4">
          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            表示件数
            <Select value={pageSize.toString()} onValueChange={(v) => setPageSize(Number(v))}>
              <SelectTrigger className="w-[90px]">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {pageSizeOptions.map((size) => (
                  <SelectItem key={size} value={size.toString()}>
                    {size}件
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <span>
              {sortedCalls.length} 件中 {startIndex} - {endIndex} を表示
            </span>
          </div>

          <div className="flex items-center gap-2">
            {Array.from({ length: totalPages }).map((_, index) => {
              const pageNumber = index + 1
              return (
                <Button
                  key={pageNumber}
                  variant={pageNumber === page ? "default" : "outline"}
                  size="sm"
                  onClick={() => setPage(pageNumber)}
                >
                  {pageNumber}
                </Button>
              )
            })}
          </div>
        </div>
      </div>

      <CallDetailDrawer
        call={selectedCall}
        open={Boolean(selectedCall)}
        onOpenChange={(open) => !open && setSelectedCall(null)}
      />
    </>
  )
}

function toRecord(call: Call): CallRecord {
  const status = toStatus(call.status)
  const direction = toDirection(call)

  return {
    id: call.id,
    callId: call.externalCallId,
    from: call.callerNumber ?? "非通知",
    fromName: categoryLabel(call.callerCategory),
    to: "未設定",
    startedAt: call.startedAt,
    endedAt: call.endedAt ?? null,
    status,
    durationSec: call.durationSec ?? 0,
    summary: "",
    recordingUrl: null,
    direction,
  }
}

function toStatus(status: Call["status"]): CallRecord["status"] {
  switch (status) {
    case "ringing":
    case "in_call":
      return "in_call"
    case "error":
      return "missed"
    default:
      return "ended"
  }
}

function toDirection(call: Call): CallRecord["direction"] {
  if (call.status === "error" || call.endReason === "rejected") return "missed"
  if (call.actionCode === "AR") return "outbound"
  return "inbound"
}

function categoryLabel(category: Call["callerCategory"]): string {
  switch (category) {
    case "registered":
      return "登録済み"
    case "spam":
      return "迷惑電話"
    case "anonymous":
      return "匿名"
    default:
      return "未登録"
  }
}

function formatDuration(seconds: number) {
  const mins = Math.floor(seconds / 60)
  const secs = Math.floor(seconds % 60)
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`
}

function formatDateTime(value: string) {
  const date = new Date(value)
  return new Intl.DateTimeFormat("ja-JP", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date)
}

function statusLabel(status: CallRecord["status"]) {
  switch (status) {
    case "ended":
      return "完了"
    case "missed":
      return "不在"
    case "in_call":
      return "通話中"
    default:
      return "-"
  }
}

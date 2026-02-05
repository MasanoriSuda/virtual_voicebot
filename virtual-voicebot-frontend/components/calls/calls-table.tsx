"use client"

import { ChevronDown, ChevronUp, PhoneIncoming, PhoneMissed, PhoneOutgoing } from "lucide-react"

import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import type { CallRecord } from "@/lib/mock-data"
import { cn } from "@/lib/utils"

interface CallsTableProps {
  calls: CallRecord[]
  sortDirection: "asc" | "desc"
  onSortToggle: () => void
  onRowClick: (call: CallRecord) => void
}

export function CallsTable({ calls, sortDirection, onSortToggle, onRowClick }: CallsTableProps) {
  return (
    <div className="overflow-hidden rounded-2xl border bg-card/70">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead className="w-[160px]">
              <button
                type="button"
                onClick={onSortToggle}
                className="inline-flex items-center gap-1 text-xs font-semibold uppercase tracking-[0.2em]"
              >
                日時
                {sortDirection === "asc" ? (
                  <ChevronUp className="h-3 w-3" />
                ) : (
                  <ChevronDown className="h-3 w-3" />
                )}
              </button>
            </TableHead>
            <TableHead>方向</TableHead>
            <TableHead>発信者</TableHead>
            <TableHead>着信先</TableHead>
            <TableHead>通話時間</TableHead>
            <TableHead>ステータス</TableHead>
            <TableHead className="text-right">操作</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {calls.length === 0 ? (
            <TableRow>
              <TableCell colSpan={7} className="py-12 text-center text-sm text-muted-foreground">
                該当する通話履歴がありません
              </TableCell>
            </TableRow>
          ) : (
            calls.map((call) => (
              <TableRow
                key={call.id}
                className="cursor-pointer transition-colors hover:bg-muted/40"
                onClick={() => onRowClick(call)}
              >
                <TableCell className="font-medium text-sm">
                  {formatDateTime(call.startedAt)}
                </TableCell>
                <TableCell>
                  <DirectionBadge direction={call.direction} />
                </TableCell>
                <TableCell>
                  <div>
                    <p className="font-medium">{call.fromName}</p>
                    <p className="text-xs text-muted-foreground">{call.from}</p>
                  </div>
                </TableCell>
                <TableCell className="text-sm">{call.to}</TableCell>
                <TableCell className="font-mono text-xs">
                  {formatDuration(call.durationSec)}
                </TableCell>
                <TableCell>
                  <Badge className={cn("px-2 py-0.5 text-xs", statusClass(call.status))}>
                    {statusLabel(call.status)}
                  </Badge>
                </TableCell>
                <TableCell className="text-right">
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={(event) => {
                      event.stopPropagation()
                      onRowClick(call)
                    }}
                  >
                    詳細
                  </Button>
                </TableCell>
              </TableRow>
            ))
          )}
        </TableBody>
      </Table>
    </div>
  )
}

function DirectionBadge({ direction }: { direction: CallRecord["direction"] }) {
  const config = {
    inbound: {
      label: "着信",
      icon: PhoneIncoming,
      className: "bg-primary/10 text-primary",
    },
    outbound: {
      label: "発信",
      icon: PhoneOutgoing,
      className: "bg-emerald-500/10 text-emerald-600 dark:text-emerald-300",
    },
    missed: {
      label: "不在",
      icon: PhoneMissed,
      className: "bg-rose-500/10 text-rose-600 dark:text-rose-300",
    },
  }[direction]

  const Icon = config.icon

  return (
    <span className={cn("inline-flex items-center gap-1 rounded-full px-2 py-1 text-xs", config.className)}>
      <Icon className="h-3 w-3" />
      {config.label}
    </span>
  )
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

function statusClass(status: CallRecord["status"]) {
  switch (status) {
    case "ended":
      return "bg-emerald-500/15 text-emerald-600 dark:text-emerald-300"
    case "missed":
      return "bg-rose-500/15 text-rose-600 dark:text-rose-300"
    case "in_call":
      return "bg-sky-500/15 text-sky-600 dark:text-sky-300"
    default:
      return "bg-muted text-muted-foreground"
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
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(date)
}

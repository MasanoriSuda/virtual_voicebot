"use client"

import { useMemo } from "react"
import { CalendarIcon, Search } from "lucide-react"
import { DateRange } from "react-day-picker"
import { addDays, endOfDay, startOfDay, subDays } from "date-fns"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select"
import { Calendar } from "@/components/ui/calendar"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { cn } from "@/lib/utils"

interface FilterBarProps {
  dateRange: DateRange | undefined
  onDateChange: (range: DateRange | undefined) => void
  typeFilter: string
  onTypeChange: (value: string) => void
  search: string
  onSearchChange: (value: string) => void
}

export function FilterBar({
  dateRange,
  onDateChange,
  typeFilter,
  onTypeChange,
  search,
  onSearchChange,
}: FilterBarProps) {
  const label = useMemo(() => formatRangeLabel(dateRange), [dateRange])

  return (
    <div className="flex flex-col gap-3 rounded-2xl border bg-card/70 p-4 shadow-sm lg:flex-row lg:items-center lg:justify-between">
      <div className="flex flex-wrap items-center gap-3">
        <Popover>
          <PopoverTrigger asChild>
            <Button variant="outline" className="justify-start gap-2">
              <CalendarIcon className="h-4 w-4" />
              <span className="text-sm">{label}</span>
            </Button>
          </PopoverTrigger>
          <PopoverContent align="start" className="w-auto p-0">
            <div className="border-b px-3 py-2 text-xs text-muted-foreground">
              期間を選択
            </div>
            <div className="flex flex-wrap gap-2 p-3">
              <QuickButton onClick={() => onDateChange(rangeToday())} label="今日" />
              <QuickButton onClick={() => onDateChange(rangeYesterday())} label="昨日" />
              <QuickButton onClick={() => onDateChange(rangeLast7Days())} label="過去7日" />
              <QuickButton onClick={() => onDateChange(undefined)} label="クリア" />
            </div>
            <Calendar
              mode="range"
              selected={dateRange}
              onSelect={onDateChange}
              numberOfMonths={2}
              defaultMonth={dateRange?.from}
              className="border-t"
            />
          </PopoverContent>
        </Popover>

        <Select value={typeFilter} onValueChange={onTypeChange}>
          <SelectTrigger className="w-[160px]">
            <SelectValue placeholder="すべて" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">すべて</SelectItem>
            <SelectItem value="inbound">着信</SelectItem>
            <SelectItem value="outbound">発信</SelectItem>
            <SelectItem value="missed">不在</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <div className="relative w-full lg:w-[260px]">
        <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          value={search}
          onChange={(event) => onSearchChange(event.target.value)}
          placeholder="キーワード検索"
          className="pl-9"
        />
      </div>
    </div>
  )
}

function QuickButton({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <Button variant="secondary" size="sm" onClick={onClick} className="rounded-full">
      {label}
    </Button>
  )
}

function formatRangeLabel(range: DateRange | undefined) {
  if (!range?.from) {
    return "期間を選択"
  }

  const from = formatDate(range.from)
  if (!range.to) {
    return from
  }

  return `${from} - ${formatDate(range.to)}`
}

function formatDate(date: Date) {
  return new Intl.DateTimeFormat("ja-JP", {
    month: "2-digit",
    day: "2-digit",
  }).format(date)
}

function rangeToday(): DateRange {
  const today = new Date()
  return { from: startOfDay(today), to: endOfDay(today) }
}

function rangeYesterday(): DateRange {
  const day = subDays(new Date(), 1)
  return { from: startOfDay(day), to: endOfDay(day) }
}

function rangeLast7Days(): DateRange {
  const today = new Date()
  return { from: startOfDay(subDays(today, 6)), to: endOfDay(addDays(today, 0)) }
}

export function isWithinRange(date: Date, range?: DateRange) {
  if (!range?.from) return true
  const from = range.from
  const to = range.to ?? range.from
  return date >= from && date <= to
}

export function directionLabel(value: string) {
  switch (value) {
    case "inbound":
      return "着信"
    case "outbound":
      return "発信"
    case "missed":
      return "不在"
    default:
      return "すべて"
  }
}

export function directionClass(value: string) {
  return cn(
    "rounded-full px-2.5 py-1 text-xs",
    value === "inbound" && "bg-primary/10 text-primary",
    value === "outbound" && "bg-emerald-500/10 text-emerald-600 dark:text-emerald-300",
    value === "missed" && "bg-rose-500/10 text-rose-600 dark:text-rose-300"
  )
}

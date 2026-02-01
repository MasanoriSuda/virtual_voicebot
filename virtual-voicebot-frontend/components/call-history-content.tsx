"use client"

import { useState, useMemo } from "react"
import type { Call } from "@/lib/types"
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from "./ui/card"
import { Input } from "./ui/input"
import { Button } from "./ui/button"
import { Badge } from "./ui/badge"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "./ui/table"
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "./ui/select"
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "./ui/dropdown-menu"
import {
  Search,
  Download,
  Phone,
  PhoneIncoming,
  PhoneOutgoing,
  PhoneMissed,
  ChevronLeft,
  ChevronRight,
  MoreHorizontal,
  Eye,
  Calendar,
} from "lucide-react"
import { cn } from "@/lib/utils"
import { CallDetailDrawer } from "./call-detail-drawer"

interface CallHistoryContentProps {
  initialCalls: Call[]
}

type CallTypeFilter = "all" | "inbound" | "outbound" | "missed"
type DateRangeFilter = "today" | "yesterday" | "week" | "custom"

export function CallHistoryContent({ initialCalls }: CallHistoryContentProps) {
  const [calls] = useState<Call[]>(initialCalls)
  const [searchQuery, setSearchQuery] = useState("")
  const [callTypeFilter, setCallTypeFilter] = useState<CallTypeFilter>("all")
  const [dateRangeFilter, setDateRangeFilter] = useState<DateRangeFilter>("week")
  const [itemsPerPage, setItemsPerPage] = useState(10)
  const [currentPage, setCurrentPage] = useState(1)
  const [selectedCallId, setSelectedCallId] = useState<string | null>(null)

  const filteredCalls = useMemo(() => {
    let result = calls

    // Search filter
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase()
      result = result.filter((call) => {
        const matchesNumber =
          call.from.toLowerCase().includes(query) ||
          call.to.toLowerCase().includes(query)
        return matchesNumber
      })
    }

    // Call type filter
    if (callTypeFilter !== "all") {
      result = result.filter((call) => {
        if (callTypeFilter === "missed") return call.status === "failed"
        if (callTypeFilter === "inbound") return true // Mock: treat all as inbound for demo
        if (callTypeFilter === "outbound") return false
        return true
      })
    }

    // Date range filter
    const now = new Date()
    const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate())
    const startOfYesterday = new Date(startOfToday)
    startOfYesterday.setDate(startOfYesterday.getDate() - 1)
    const startOfWeek = new Date(startOfToday)
    startOfWeek.setDate(startOfWeek.getDate() - 7)

    if (dateRangeFilter === "today") {
      result = result.filter((call) => new Date(call.startTime) >= startOfToday)
    } else if (dateRangeFilter === "yesterday") {
      result = result.filter((call) => {
        const callDate = new Date(call.startTime)
        return callDate >= startOfYesterday && callDate < startOfToday
      })
    } else if (dateRangeFilter === "week") {
      result = result.filter((call) => new Date(call.startTime) >= startOfWeek)
    }

    return result
  }, [calls, searchQuery, callTypeFilter, dateRangeFilter])

  const totalPages = Math.ceil(filteredCalls.length / itemsPerPage)
  const paginatedCalls = filteredCalls.slice(
    (currentPage - 1) * itemsPerPage,
    currentPage * itemsPerPage
  )

  const formatDuration = (seconds: number) => {
    const mins = Math.floor(seconds / 60)
    const secs = seconds % 60
    return `${mins}:${secs.toString().padStart(2, "0")}`
  }

  const formatDateTime = (isoString: string) => {
    const date = new Date(isoString)
    return new Intl.DateTimeFormat("ja-JP", {
      year: "numeric",
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }).format(date)
  }

  const getCallDirectionIcon = (call: Call) => {
    if (call.status === "failed") {
      return <PhoneMissed className="h-4 w-4 text-red-500" />
    }
    // Mock logic: odd IDs are inbound, even are outbound
    const isInbound = Number.parseInt(call.id) % 2 === 1
    return isInbound ? (
      <PhoneIncoming className="h-4 w-4 text-green-600" />
    ) : (
      <PhoneOutgoing className="h-4 w-4 text-primary" />
    )
  }

  const getStatusBadge = (status: Call["status"]) => {
    const config = {
      active: {
        label: "通話中",
        className: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300",
      },
      completed: {
        label: "完了",
        className: "bg-secondary text-secondary-foreground",
      },
      failed: {
        label: "不在",
        className: "bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-300",
      },
    }

    return (
      <Badge variant="outline" className={cn("font-normal", config[status].className)}>
        {config[status].label}
      </Badge>
    )
  }

  const handleExportCSV = () => {
    const headers = ["日時", "発信者", "着信先", "通話時間", "ステータス"]
    const rows = filteredCalls.map((call) => [
      formatDateTime(call.startTime),
      call.from,
      call.to,
      formatDuration(call.duration),
      call.status,
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

        <Card>
          <CardHeader className="pb-4">
            <div className="flex flex-col sm:flex-row gap-4 justify-between">
              <div className="flex flex-wrap gap-2">
                {/* Date Range Filter */}
                <Select
                  value={dateRangeFilter}
                  onValueChange={(v) => {
                    setDateRangeFilter(v as DateRangeFilter)
                    setCurrentPage(1)
                  }}
                >
                  <SelectTrigger className="w-[140px]">
                    <Calendar className="h-4 w-4 mr-2" />
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="today">今日</SelectItem>
                    <SelectItem value="yesterday">昨日</SelectItem>
                    <SelectItem value="week">過去7日</SelectItem>
                    <SelectItem value="custom">カスタム</SelectItem>
                  </SelectContent>
                </Select>

                {/* Call Type Filter */}
                <Select
                  value={callTypeFilter}
                  onValueChange={(v) => {
                    setCallTypeFilter(v as CallTypeFilter)
                    setCurrentPage(1)
                  }}
                >
                  <SelectTrigger className="w-[120px]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">すべて</SelectItem>
                    <SelectItem value="inbound">着信</SelectItem>
                    <SelectItem value="outbound">発信</SelectItem>
                    <SelectItem value="missed">不在</SelectItem>
                  </SelectContent>
                </Select>

                {/* Search */}
                <div className="relative flex-1 min-w-[200px]">
                  <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    placeholder="電話番号または名前で検索..."
                    value={searchQuery}
                    onChange={(e) => {
                      setSearchQuery(e.target.value)
                      setCurrentPage(1)
                    }}
                    className="pl-9"
                  />
                </div>
              </div>

              {/* Export */}
              <Button variant="outline" onClick={handleExportCSV} className="shrink-0 bg-transparent">
                <Download className="h-4 w-4 mr-2" />
                CSV出力
              </Button>
            </div>
          </CardHeader>

          <CardContent className="p-0">
            <div className="border-t">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-[180px]">日時</TableHead>
                    <TableHead className="w-[60px]">方向</TableHead>
                    <TableHead>発信者</TableHead>
                    <TableHead>着信先</TableHead>
                    <TableHead className="w-[100px]">通話時間</TableHead>
                    <TableHead className="w-[100px]">ステータス</TableHead>
                    <TableHead className="w-[80px]" />
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {paginatedCalls.length === 0 ? (
                    <TableRow>
                      <TableCell colSpan={7} className="h-32">
                        <div className="flex flex-col items-center justify-center text-muted-foreground">
                          <Phone className="h-10 w-10 mb-2 opacity-50" />
                          <p>該当する通話履歴がありません</p>
                        </div>
                      </TableCell>
                    </TableRow>
                  ) : (
                    paginatedCalls.map((call) => (
                      <TableRow
                        key={call.id}
                        className="cursor-pointer hover:bg-muted/50"
                        onClick={() => setSelectedCallId(call.id)}
                      >
                        <TableCell className="font-medium">
                          {formatDateTime(call.startTime)}
                        </TableCell>
                        <TableCell>{getCallDirectionIcon(call)}</TableCell>
                        <TableCell>{call.from}</TableCell>
                        <TableCell className="text-muted-foreground">{call.to}</TableCell>
                        <TableCell className="font-mono">
                          {formatDuration(call.duration)}
                        </TableCell>
                        <TableCell>{getStatusBadge(call.status)}</TableCell>
                        <TableCell>
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button
                                variant="ghost"
                                size="icon"
                                className="h-8 w-8"
                                onClick={(e) => e.stopPropagation()}
                              >
                                <MoreHorizontal className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuItem onClick={() => setSelectedCallId(call.id)}>
                                <Eye className="h-4 w-4 mr-2" />
                                詳細を見る
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </TableCell>
                      </TableRow>
                    ))
                  )}
                </TableBody>
              </Table>
            </div>

            {/* Pagination */}
            <div className="flex items-center justify-between px-4 py-4 border-t">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span>表示件数:</span>
                <Select
                  value={itemsPerPage.toString()}
                  onValueChange={(v) => {
                    setItemsPerPage(Number(v))
                    setCurrentPage(1)
                  }}
                >
                  <SelectTrigger className="w-[70px] h-8">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="10">10</SelectItem>
                    <SelectItem value="25">25</SelectItem>
                    <SelectItem value="50">50</SelectItem>
                  </SelectContent>
                </Select>
                <span>
                  / 全{filteredCalls.length}件
                </span>
              </div>

              <div className="flex items-center gap-2">
                <Button
                  variant="outline"
                  size="icon"
                  className="h-8 w-8 bg-transparent"
                  disabled={currentPage === 1}
                  onClick={() => setCurrentPage((p) => p - 1)}
                >
                  <ChevronLeft className="h-4 w-4" />
                </Button>
                <span className="text-sm min-w-[80px] text-center">
                  {currentPage} / {totalPages || 1}
                </span>
                <Button
                  variant="outline"
                  size="icon"
                  className="h-8 w-8 bg-transparent"
                  disabled={currentPage >= totalPages}
                  onClick={() => setCurrentPage((p) => p + 1)}
                >
                  <ChevronRight className="h-4 w-4" />
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>
      </div>

      <CallDetailDrawer
        callId={selectedCallId}
        open={!!selectedCallId}
        onClose={() => setSelectedCallId(null)}
      />
    </>
  )
}

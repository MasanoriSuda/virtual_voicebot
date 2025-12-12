"use client"

import { useState, useMemo } from "react"
import type { Call } from "@/lib/types"
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table"
import { Input } from "@/components/ui/input"
import { Badge } from "@/components/ui/badge"
import { Search } from "lucide-react"
import { useRouter } from "next/navigation"

interface CallsTableProps {
  calls: Call[]
}

export function CallsTable({ calls }: CallsTableProps) {
  const router = useRouter()
  const [searchQuery, setSearchQuery] = useState("")

  const filteredCalls = useMemo(() => {
    if (!searchQuery.trim()) return calls

    const query = searchQuery.toLowerCase()
    return calls.filter((call) => {
      const matchesNumber = call.from.toLowerCase().includes(query) || call.to.toLowerCase().includes(query)
      const matchesDate = new Date(call.startTime).toLocaleDateString("ja-JP").includes(query)
      return matchesNumber || matchesDate
    })
  }, [calls, searchQuery])

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

  const getStatusBadge = (status: Call["status"]) => {
    const variants = {
      active: "default",
      completed: "secondary",
      failed: "destructive",
    } as const

    const labels = {
      active: "通話中",
      completed: "完了",
      failed: "失敗",
    }

    return <Badge variant={variants[status]}>{labels[status]}</Badge>
  }

  return (
    <div className="space-y-4">
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input
          placeholder="電話番号または日付で検索..."
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          className="pl-10"
        />
      </div>

      <div className="border rounded-lg">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>発信元</TableHead>
              <TableHead>発信先</TableHead>
              <TableHead>開始時刻</TableHead>
              <TableHead>通話時間</TableHead>
              <TableHead>ステータス</TableHead>
              <TableHead>要約</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {filteredCalls.length === 0 ? (
              <TableRow>
                <TableCell colSpan={6} className="text-center text-muted-foreground">
                  該当する通話履歴がありません
                </TableCell>
              </TableRow>
            ) : (
              filteredCalls.map((call) => (
                <TableRow
                  key={call.id}
                  className="cursor-pointer hover:bg-muted/50"
                  onClick={() => router.push(`/calls/${call.id}`)}
                >
                  <TableCell className="font-medium">{call.from}</TableCell>
                  <TableCell className="text-muted-foreground">{call.to}</TableCell>
                  <TableCell>{formatDateTime(call.startTime)}</TableCell>
                  <TableCell>{formatDuration(call.duration)}</TableCell>
                  <TableCell>{getStatusBadge(call.status)}</TableCell>
                  <TableCell className="max-w-md truncate">{call.summary}</TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  )
}

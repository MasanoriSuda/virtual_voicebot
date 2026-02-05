"use client"

import React from "react"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card"
import { Badge } from "./ui/badge"
import {
  Phone,
  TrendingUp,
  Pause,
  ArrowRightLeft,
} from "lucide-react"
import { cn } from "@/lib/utils"
import Link from "next/link"
import { WeeklyTrendChart } from "./charts/weekly-trend-chart"
import { KpiCards } from "./dashboard/kpi-cards"
import { HourlyChart } from "./dashboard/hourly-chart"

const liveCallsData = [
  {
    id: "1",
    caller: "090-1234-5678",
    destination: "内線 101",
    duration: "2:45",
    status: "active" as const,
  },
  {
    id: "2",
    caller: "080-9876-5432",
    destination: "内線 203",
    duration: "1:12",
    status: "hold" as const,
  },
  {
    id: "3",
    caller: "070-5555-1234",
    destination: "内線 105",
    duration: "0:34",
    status: "transfer" as const,
  },
]

export function DashboardContent() {
  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-bold text-balance">ダッシュボード</h1>
        <p className="text-muted-foreground">リアルタイムの通話状況と統計を確認</p>
      </div>

      {/* KPI Cards */}
      <KpiCards />

      {/* Charts Section */}
      <div className="grid gap-6 lg:grid-cols-2">
        <HourlyChart />

        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="h-5 w-5 text-primary" />
              週間トレンド
            </CardTitle>
            <CardDescription>Weekly Trend</CardDescription>
          </CardHeader>
          <CardContent>
            <WeeklyTrendChart />
          </CardContent>
        </Card>
      </div>

      {/* Live Status Section */}
      <Card>
        <CardHeader className="flex flex-row items-center justify-between">
          <div>
            <CardTitle className="flex items-center gap-2">
              <span className="relative flex h-3 w-3">
                <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-green-400 opacity-75" />
                <span className="relative inline-flex rounded-full h-3 w-3 bg-green-500" />
              </span>
              リアルタイム通話状況
            </CardTitle>
            <CardDescription>Real-time Call Status</CardDescription>
          </div>
          <Link
            href="/calls"
            className="text-sm text-primary hover:underline"
          >
            すべて表示
          </Link>
        </CardHeader>
        <CardContent>
          <div className="space-y-4">
            {liveCallsData.length === 0 ? (
              <p className="text-center text-muted-foreground py-8">
                現在通話中のコールはありません
              </p>
            ) : (
              <div className="divide-y">
                {liveCallsData.map((call) => (
                  <div
                    key={call.id}
                    className="flex items-center justify-between py-3 first:pt-0 last:pb-0"
                  >
                    <div className="flex items-center gap-3">
                      <div className="p-2 rounded-full bg-muted">
                        {call.status === "active" && <Phone className="h-4 w-4 text-green-600" />}
                        {call.status === "hold" && <Pause className="h-4 w-4 text-yellow-600" />}
                        {call.status === "transfer" && <ArrowRightLeft className="h-4 w-4 text-primary" />}
                      </div>
                      <div>
                        <p className="font-medium">{call.caller}</p>
                        <p className="text-sm text-muted-foreground">{call.destination}</p>
                      </div>
                    </div>
                    <div className="flex items-center gap-3">
                      <span className="text-sm font-mono">{call.duration}</span>
                      <CallStatusBadge status={call.status} />
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

function CallStatusBadge({ status }: { status: "active" | "hold" | "transfer" }) {
  const config = {
    active: { label: "通話中", className: "bg-green-100 text-green-800 dark:bg-green-900 dark:text-green-300" },
    hold: { label: "保留中", className: "bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-300" },
    transfer: { label: "転送中", className: "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-300" },
  }

  return (
    <Badge variant="outline" className={cn("font-normal", config[status].className)}>
      {config[status].label}
    </Badge>
  )
}

"use client"

import React from "react"

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./ui/card"
import { Badge } from "./ui/badge"
import {
  Phone,
  Clock,
  TrendingUp,
  TrendingDown,
  Users,
  PhoneIncoming,
  PhoneOutgoing,
  Pause,
  ArrowRightLeft,
} from "lucide-react"
import { cn } from "@/lib/utils"
import Link from "next/link"
import { HourlyCallChart } from "./charts/hourly-call-chart"
import { WeeklyTrendChart } from "./charts/weekly-trend-chart"

// Mock data
const kpiData = {
  totalCalls: 156,
  totalCallsChange: 12,
  avgDuration: "2:34",
  avgDurationChange: -5,
  answerRate: 94.2,
  answerRateChange: 2.1,
  availableOperators: 8,
  totalOperators: 12,
}

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
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        <KpiCard
          title="本日の総通話数"
          titleEn="Today's Calls"
          value={kpiData.totalCalls.toString()}
          change={kpiData.totalCallsChange}
          icon={Phone}
        />
        <KpiCard
          title="平均通話時間"
          titleEn="Avg Duration"
          value={kpiData.avgDuration}
          change={kpiData.avgDurationChange}
          icon={Clock}
        />
        <KpiCard
          title="応答率"
          titleEn="Answer Rate"
          value={`${kpiData.answerRate}%`}
          change={kpiData.answerRateChange}
          icon={TrendingUp}
          showProgress
          progress={kpiData.answerRate}
        />
        <KpiCard
          title="待機中オペレーター"
          titleEn="Available Operators"
          value={`${kpiData.availableOperators}/${kpiData.totalOperators}`}
          icon={Users}
          showStatus
          statusActive={kpiData.availableOperators > 0}
        />
      </div>

      {/* Charts Section */}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <PhoneIncoming className="h-5 w-5 text-primary" />
              時間帯別通話数
            </CardTitle>
            <CardDescription>Hourly Call Volume</CardDescription>
          </CardHeader>
          <CardContent>
            <HourlyCallChart />
          </CardContent>
        </Card>

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

interface KpiCardProps {
  title: string
  titleEn: string
  value: string
  change?: number
  icon: React.ElementType
  showProgress?: boolean
  progress?: number
  showStatus?: boolean
  statusActive?: boolean
}

function KpiCard({
  title,
  titleEn,
  value,
  change,
  icon: Icon,
  showProgress,
  progress,
  showStatus,
  statusActive,
}: KpiCardProps) {
  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle className="text-sm font-medium">
          {title}
          <span className="block text-xs text-muted-foreground font-normal">{titleEn}</span>
        </CardTitle>
        <Icon className="h-5 w-5 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div className="flex items-end justify-between">
          <div className="flex items-center gap-2">
            <span className="text-3xl font-bold">{value}</span>
            {showStatus && (
              <span
                className={cn(
                  "w-2 h-2 rounded-full",
                  statusActive ? "bg-green-500" : "bg-muted"
                )}
              />
            )}
          </div>
          {change !== undefined && (
            <div
              className={cn(
                "flex items-center text-sm",
                change >= 0 ? "text-green-600" : "text-red-600"
              )}
            >
              {change >= 0 ? (
                <TrendingUp className="h-4 w-4 mr-1" />
              ) : (
                <TrendingDown className="h-4 w-4 mr-1" />
              )}
              {Math.abs(change)}%
            </div>
          )}
        </div>
        {showProgress && progress !== undefined && (
          <div className="mt-3">
            <div className="h-2 w-full bg-muted rounded-full overflow-hidden">
              <div
                className="h-full bg-primary rounded-full transition-all"
                style={{ width: `${progress}%` }}
              />
            </div>
          </div>
        )}
      </CardContent>
    </Card>
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

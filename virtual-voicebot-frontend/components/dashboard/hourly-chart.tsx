"use client"

import { useEffect, useState } from "react"
import {
  Bar,
  BarChart,
  CartesianGrid,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts"

import { Card } from "@/components/ui/card"

interface HourlyData {
  hour: number
  inbound: number
  outbound: number
}

const tooltipStyle = {
  backgroundColor: "var(--card)",
  border: "1px solid var(--border)",
  borderRadius: "0.75rem",
  color: "var(--foreground)",
}

export function HourlyChart() {
  const [data, setData] = useState<HourlyData[]>(
    Array.from({ length: 24 }, (_, hour) => ({ hour, inbound: 0, outbound: 0 })),
  )

  useEffect(() => {
    let active = true
    fetch("/api/kpi")
      .then((res) => {
        if (!res.ok) {
          throw new Error("failed to fetch hourly stats")
        }
        return res.json() as Promise<{ hourlyStats?: HourlyData[] }>
      })
      .then((payload) => {
        if (!active || !Array.isArray(payload.hourlyStats)) {
          return
        }
        setData(payload.hourlyStats)
      })
      .catch((error) => {
        console.error("[hourly-chart] failed to fetch hourly stats", error)
      })

    return () => {
      active = false
    }
  }, [])

  return (
    <Card className="border-none bg-card/80 p-4 shadow-[0_24px_60px_-40px_rgba(15,23,42,0.45)] dark:shadow-[0_24px_60px_-40px_rgba(8,14,25,0.6)]">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-xs uppercase tracking-[0.2em] text-muted-foreground">時間帯別通話数</p>
          <p className="text-lg font-semibold">0時〜23時</p>
        </div>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <span className="inline-flex h-2 w-2 rounded-full bg-[var(--chart-1)]" />
          着信
          <span className="inline-flex h-2 w-2 rounded-full bg-[var(--chart-2)]" />
          発信
        </div>
      </div>
      <div className="mt-4 h-64">
        <ResponsiveContainer width="100%" height="100%">
          <BarChart data={data} barGap={6} barSize={10}>
            <CartesianGrid stroke="var(--border)" strokeDasharray="4 4" />
            <XAxis
              dataKey="hour"
              stroke="var(--muted-foreground)"
              fontSize={12}
              tickFormatter={(value) => `${value}`}
            />
            <YAxis stroke="var(--muted-foreground)" fontSize={12} />
            <Tooltip cursor={{ fill: "var(--accent)" }} contentStyle={tooltipStyle} />
            <Bar dataKey="inbound" fill="var(--chart-1)" radius={[6, 6, 0, 0]} />
            <Bar dataKey="outbound" fill="var(--chart-2)" radius={[6, 6, 0, 0]} />
          </BarChart>
        </ResponsiveContainer>
      </div>
    </Card>
  )
}

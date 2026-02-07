"use client"

import { useEffect, useMemo, useState } from "react"
import { Activity, CheckCircle, Clock, Phone } from "lucide-react"

import { Card } from "@/components/ui/card"
import { cn } from "@/lib/utils"

interface KpiData {
  totalCalls: number
  totalCallsChange: number
  avgDurationSec: number
  avgDurationChange: number
  answerRate: number
  answerRateChange: number
  activeCalls: number
}

const emptyKpi: KpiData = {
  totalCalls: 0,
  totalCallsChange: 0,
  avgDurationSec: 0,
  avgDurationChange: 0,
  answerRate: 0,
  answerRateChange: 0,
  activeCalls: 0,
}

export function KpiCards() {
  const [kpi, setKpi] = useState<KpiData>(emptyKpi)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    let active = true
    fetch("/api/kpi")
      .then((res) => {
        if (!res.ok) {
          throw new Error("failed to fetch kpi")
        }
        return res.json() as Promise<{ kpi: KpiData }>
      })
      .then((data) => {
        if (!active) return
        setKpi(data.kpi ?? emptyKpi)
      })
      .catch((error) => {
        console.error("[kpi-cards] failed to fetch KPI", error)
      })
      .finally(() => {
        if (active) {
          setLoading(false)
        }
      })

    return () => {
      active = false
    }
  }, [])

  const cards = useMemo(
    () => [
      {
        title: "本日の総通話数",
        value: kpi.totalCalls.toString(),
        change: kpi.totalCallsChange,
        suffix: "%",
        icon: Phone,
      },
      {
        title: "平均通話時間",
        value: formatDuration(kpi.avgDurationSec),
        change: kpi.avgDurationChange,
        suffix: "%",
        icon: Clock,
      },
      {
        title: "応答率",
        value: `${Math.round(kpi.answerRate * 100)}%`,
        change: kpi.answerRateChange,
        suffix: "%",
        icon: CheckCircle,
      },
      {
        title: "アクティブ通話",
        value: kpi.activeCalls.toString(),
        change: null,
        suffix: "",
        icon: Activity,
      },
    ],
    [kpi],
  )

  return (
    <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      {cards.map((card, index) => {
        const Icon = card.icon
        return (
          <Card
            key={card.title}
            className="relative overflow-hidden border-none bg-gradient-to-br from-card via-card/90 to-card/70 p-5 shadow-[0_24px_60px_-40px_rgba(15,23,42,0.5)] dark:shadow-[0_24px_60px_-40px_rgba(8,14,25,0.7)] animate-in fade-in slide-in-from-bottom-3 duration-700"
            style={{ animationDelay: `${index * 90}ms` }}
          >
            <div className="flex items-start justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.2em] text-muted-foreground">
                  {card.title}
                </p>
                <p className="mt-2 text-3xl font-semibold tracking-tight">
                  {loading ? "--" : card.value}
                </p>
              </div>
              <div className="rounded-2xl bg-primary/10 p-3 text-primary">
                <Icon className="h-5 w-5" />
              </div>
            </div>
            <div className="mt-4 flex items-center gap-2 text-sm">
              {card.change === null ? (
                <span className="text-muted-foreground">現在</span>
              ) : (
                <span
                  className={cn(
                    "rounded-full px-2.5 py-1 text-xs font-medium",
                    card.change >= 0
                      ? "bg-emerald-500/10 text-emerald-600 dark:text-emerald-300"
                      : "bg-rose-500/10 text-rose-600 dark:text-rose-300",
                  )}
                >
                  {card.change >= 0 ? "+" : ""}
                  {loading ? 0 : card.change.toFixed(1)}
                  {card.suffix} 前日比
                </span>
              )}
            </div>
          </Card>
        )
      })}
    </div>
  )
}

function formatDuration(seconds: number) {
  const mins = Math.floor(seconds / 60)
  const secs = seconds % 60
  return `${mins.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`
}

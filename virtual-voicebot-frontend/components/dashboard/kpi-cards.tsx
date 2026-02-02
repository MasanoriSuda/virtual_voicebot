import { Activity, CheckCircle, Clock, Phone } from "lucide-react"

import { Card } from "@/components/ui/card"
import { cn } from "@/lib/utils"
import { mockKPI } from "@/lib/mock-data"

const cards = [
  {
    title: "本日の総通話数",
    value: mockKPI.totalCalls.toString(),
    change: mockKPI.totalCallsChange,
    suffix: "%",
    icon: Phone,
  },
  {
    title: "平均通話時間",
    value: formatDuration(mockKPI.avgDurationSec),
    change: mockKPI.avgDurationChange,
    suffix: "%",
    icon: Clock,
  },
  {
    title: "応答率",
    value: `${Math.round(mockKPI.answerRate * 100)}%`,
    change: mockKPI.answerRateChange,
    suffix: "%",
    icon: CheckCircle,
  },
  {
    title: "アクティブ通話",
    value: mockKPI.activeCalls.toString(),
    change: null,
    suffix: "",
    icon: Activity,
  },
]

export function KpiCards() {
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
                  {card.value}
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
                      : "bg-rose-500/10 text-rose-600 dark:text-rose-300"
                  )}
                >
                  {card.change >= 0 ? "+" : ""}
                  {card.change}
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

import type { Call, IvrSessionEvent } from "@/lib/types"

interface TimelineItem {
  id: string
  occurredAt: string
  label: string
  order: number
}

function formatTime(value: string): string {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) {
    return "--:--:--"
  }
  return new Intl.DateTimeFormat("ja-JP", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  }).format(date)
}

function describeEvent(event: IvrSessionEvent): string {
  switch (event.eventType) {
    case "node_enter":
      return `ノード訪問: ${event.nodeId ?? "-"}`
    case "dtmf_input":
      return `DTMF入力: ${event.dtmfKey ?? "-"}`
    case "transition":
      return `遷移: ${event.transitionId ?? "-"}`
    case "timeout":
      return "タイムアウト"
    case "invalid_input":
      return "無効入力"
    case "exit":
      return `IVR終了: ${event.exitReason ?? event.exitAction ?? "-"}`
    default:
      return event.eventType
  }
}

export function IvrTraceTimeline({ events, call }: { events: IvrSessionEvent[]; call: Call }) {
  const items: TimelineItem[] = events.map((event) => ({
    id: event.id,
    occurredAt: event.occurredAt,
    label: describeEvent(event),
    order: event.sequence,
  }))

  if (call.transferStartedAt) {
    items.push({
      id: "transfer-started",
      occurredAt: call.transferStartedAt,
      label: "転送試行開始",
      order: Number.MAX_SAFE_INTEGER - 2,
    })
  }
  if (call.transferAnsweredAt) {
    items.push({
      id: "transfer-answered",
      occurredAt: call.transferAnsweredAt,
      label: "転送成立",
      order: Number.MAX_SAFE_INTEGER - 1,
    })
  }
  if (call.transferEndedAt) {
    items.push({
      id: "transfer-ended",
      occurredAt: call.transferEndedAt,
      label: "転送終了",
      order: Number.MAX_SAFE_INTEGER,
    })
  }

  const sorted = items.sort((a, b) => {
    const timeDiff = Date.parse(a.occurredAt) - Date.parse(b.occurredAt)
    if (timeDiff !== 0) {
      return timeDiff
    }
    return a.order - b.order
  })

  if (sorted.length === 0) {
    return <p className="text-sm text-muted-foreground">IVRイベントはありません。</p>
  }

  return (
    <div className="space-y-2">
      {sorted.map((item) => (
        <div key={item.id} className="flex gap-4 border-l-2 border-muted pl-4 py-2">
          <div className="w-24 shrink-0 text-xs text-muted-foreground">{formatTime(item.occurredAt)}</div>
          <div className="text-sm">{item.label}</div>
        </div>
      ))}
    </div>
  )
}
